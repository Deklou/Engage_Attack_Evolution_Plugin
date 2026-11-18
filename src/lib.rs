use core::sync::atomic::{AtomicBool, Ordering};
use engage::gamedata::{
    skill::SkillData,
    unit::{GodUnit, Unit},
    Gamedata,
};
use unity::prelude::*;

// Fire Emblem Engage 2.0.0 / main_200 (image base 0x7100000000).
// Only replace GodUnit.GetEngageAttack's result, leaving vanilla's state,
// linked-attack selection, class-style dispatch, and menu logic intact.
#[skyline::from_offset(0x023302C0)]
fn god_unit_get_level(god: &GodUnit, unit: &Unit, method: OptionalMethod) -> i32;

struct EvolutionRule {
    god_gid: &'static str,
    original_sid: &'static str,
    upgraded_sid: &'static str,
    bond_level: i32,
    variants: &'static [&'static str],
}

// Style suffixes are based on the playable Celica rows in Skill.xml.
// Do not include the enemy-only _闇 variants.
const CELICA_VARIANTS: &[&str] = &["", "_竜族", "_騎馬", "_飛行", "_魔法"];

const EVOLUTIONS: &[EvolutionRule] = &[EvolutionRule {
    god_gid: "GID_セリカ",
    original_sid: "SID_セリカエンゲージ技",
    upgraded_sid: "SID_セリカエンゲージ技_Plus",
    bond_level: 10,
    variants: CELICA_VARIANTS,
}];

static MISSING_SKILL_REPORTED: AtomicBool = AtomicBool::new(false);
static UPGRADE_REPORTED: AtomicBool = AtomicBool::new(false);

// Match only an explicitly supported style of the vanilla attack.
// This prevents unintentionally modifying enemy-only or linked attacks.
fn variant_suffix<'a>(original_sid: &'a str, rule: &EvolutionRule) -> Option<&'a str> {
    let suffix = original_sid.strip_prefix(rule.original_sid)?;
    rule.variants.contains(&suffix).then_some(suffix)
}

// Ghidra: App.GodUnit.GetEngageAttack(GodUnit, Unit, MethodInfo) at
// 0x7102341640. This is the THREE-argument overload.
#[skyline::hook(offset = 0x02341640)]
fn get_engage_attack(
    god: &GodUnit,
    unit: &Unit,
    method: OptionalMethod,
) -> Option<&'static SkillData> {
    let vanilla = call_original!(god, unit, method);
    let Some(original_skill) = vanilla else {
        return vanilla;
    };

    let Ok(gid) = god.data.gid.get_string() else {
        return vanilla;
    };
    let Ok(sid) = original_skill.sid.get_string() else {
        return vanilla;
    };

    for rule in EVOLUTIONS {
        if gid != rule.god_gid {
            continue;
        }
        let Some(suffix) = variant_suffix(&sid, rule) else {
            continue;
        };

        // This is the numeric bond level (1..20), not GetRelianceLevel's rank.
        let bond = unsafe { god_unit_get_level(god, unit, None) };
        if bond < rule.bond_level {
            return vanilla;
        }

        let target_sid = format!("{}{}", rule.upgraded_sid, suffix);
        if let Some(upgrade) = SkillData::get(&target_sid) {
            if !UPGRADE_REPORTED.swap(true, Ordering::Relaxed) {
                println!(
                    "[engage_attack_evolution] Celica evolution active: level={} sid={}",
                    bond, target_sid
                );
            }
            return Some(upgrade);
        }

        // Missing XML must never make the attack disappear or crash the game.
        if !MISSING_SKILL_REPORTED.swap(true, Ordering::Relaxed) {
            println!(
                "[engage_attack_evolution] upgrade SID not found: {}; keeping vanilla",
                target_sid
            );
        }
        return vanilla;
    }
    vanilla
}

#[skyline::main(name = "engage_attack_evolution")]
pub fn main() {
    skyline::install_hooks!(get_engage_attack);
    println!("[engage_attack_evolution] loaded (Engage 2.0.0, Celica level 10)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celica_styles_match() {
        let rule = &EVOLUTIONS[0];
        assert_eq!(variant_suffix("SID_セリカエンゲージ技", rule), Some(""));
        assert_eq!(
            variant_suffix("SID_セリカエンゲージ技_魔法", rule),
            Some("_魔法")
        );
        assert_eq!(variant_suffix("SID_セリカエンゲージ技_闇", rule), None);
        assert_eq!(variant_suffix("SID_マルスエンゲージ技", rule), None);
        assert_eq!(variant_suffix("SID_セリカエンゲージ技_Plus", rule), None);
    }
}
