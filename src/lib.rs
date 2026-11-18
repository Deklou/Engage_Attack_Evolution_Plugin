use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use engage::{
    gamedata::{
        item::{unititemlist_add, ItemData, UnitItem},
        skill::SkillData,
        unit::{GodUnit, Unit},
        Gamedata,
    },
    mapmind::MapMind,
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
// The suffix '＋' is the full-width plus sign used in the modpack (NOT '_Plus').
// Do not include the enemy-only _闇 variants.
const CELICA_VARIANTS: &[&str] = &["", "_竜族", "_騎馬", "_飛行", "_魔法"];

const EVOLUTIONS: &[EvolutionRule] = &[EvolutionRule {
    god_gid: "GID_セリカ",
    original_sid: "SID_セリカエンゲージ技",
    upgraded_sid: "SID_セリカエンゲージ技＋",
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

        // Do not return the root + skill for a styled unit: that would drop
        // the class-specific bonuses (magic, flying, cavalry, dragon).
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


// Temporary weapon picker: only activated while opening Warp Ragnarok+'s
// item selection. Never unlock Ragnarok in GodGrowthData or at Engage start.
//
// main_200: UnitItemList.Add, UnitItem.SetEngageWeaponGod(Unit), and
// UnitItem.Dispose are native routines. Dispose clears just the temporary
// slot, without rebuilding or disturbing other Engage weapons.
const CELICA_PLUS_SID: &str = "SID_セリカエンゲージ技＋";
const CELICA_RAGNAROK_IID: &str = "IID_セリカ_ライナロック";
const RAGNAROK_UNLOCK_LEVEL: i32 = 15;

#[skyline::from_offset(0x01A265E0)]
fn unit_is_engaging(unit: &Unit, method: OptionalMethod) -> bool;

#[skyline::from_offset(0x01FB04E0)]
fn set_engage_weapon_god(item: &UnitItem, unit: &Unit, method: OptionalMethod);

// IL2CPP's Unit references are stable during a map sequence. We retain an
// owner only until the next menu/cursor transition and clear it on map dispose.
// We never store the temporary UnitItem itself across a menu transition.
static TEMP_OWNER: AtomicUsize = AtomicUsize::new(0);
static TEMP_SLOT: AtomicI32 = AtomicI32::new(-1);

fn celica_god(unit: &Unit) -> Option<&GodUnit> {
    for god in [unit.god_link, unit.god_unit].into_iter().flatten() {
        if god.data.gid.get_string().ok().as_deref() == Some("GID_セリカ") {
            return Some(god);
        }
    }
    None
}

fn is_celica_plus_skill(skill: &SkillData) -> bool {
    skill.sid
        .get_string()
        .ok()
        .and_then(|sid| sid.strip_prefix(CELICA_PLUS_SID).map(str::to_owned))
        .is_some_and(|suffix| CELICA_VARIANTS.contains(&suffix.as_str()))
}

fn should_offer_temporary_ragnarok(bond: i32, already_present: bool) -> bool {
    (10..RAGNAROK_UNLOCK_LEVEL).contains(&bond) && !already_present
}

// Only dispose Ragnarok inserted by this plugin; do not resynchronize the
// entire inventory, which could interfere with other mods' Engage weapons.
fn clear_temporary_ragnarok(reason: &str) {
    let owner = TEMP_OWNER.swap(0, Ordering::SeqCst);
    let slot = TEMP_SLOT.swap(-1, Ordering::SeqCst);
    if owner == 0 {
        return;
    }

    let unit = unsafe { &*(owner as *const Unit) };
    // An item can change slots when the attack menu equips it. Look in the
    // remembered slot first, then scan the other seven slots.
    for index in core::iter::once(slot).chain((0..8).filter(|&i| i != slot)) {
        if !(0..8).contains(&index) {
            continue;
        }
        let Some(item) = unit.item_list.get_item(index) else {
            continue;
        };
        if item.item.iid.get_string().ok().as_deref() == Some(CELICA_RAGNAROK_IID)
            && item.god_unit.is_some_and(|god| {
                god.data.gid.get_string().ok().as_deref() == Some("GID_セリカ")
            })
        {
            item.dispose(); // App.UnitItem.Dispose, with native GC write barriers.
            println!(
                "[engage_attack_evolution] temporary Ragnarok removed: unit={} slot={} after {}",
                unit.ident, index, reason
            );
            return;
        }
    }
    println!(
        "[engage_attack_evolution] temporary Ragnarok already absent: unit={} after {}",
        unit.ident, reason
    );
}

fn offer_temporary_ragnarok(skill: &SkillData, engage_target: bool) {
    // When the engine routes Celica through the generic Engage target path,
    // CommandSkill can still refer to the base attack even though the equipped
    // skill has evolved. Mind 6 plus a verified Celica bond is authoritative.
    if !engage_target && !is_celica_plus_skill(skill) {
        println!("[engage_attack_evolution] picker skip: received non-plus skill");
        return;
    }
    let unit = MapMind::get_unit();
    if !unsafe { unit_is_engaging(unit, None) } {
        println!("[engage_attack_evolution] picker skip: unit is not engaged");
        return;
    }
    let Some(god) = celica_god(unit) else {
        println!("[engage_attack_evolution] picker skip: Celica not found on selected unit");
        return;
    };
    let bond = unsafe { god_unit_get_level(god, unit, None) };
    let owned = unit.item_list.has_item_iid(CELICA_RAGNAROK_IID);
    if !should_offer_temporary_ragnarok(bond, owned) {
        println!("[engage_attack_evolution] picker skip: bond={} Ragnarok already_present={}", bond, owned);
        return;
    }

    let owner = unit as *const Unit as usize;
    let existing = TEMP_OWNER.load(Ordering::SeqCst);
    if existing != 0 {
        if existing != owner {
            clear_temporary_ragnarok("unit change");
        } else {
            let slot = TEMP_SLOT.load(Ordering::SeqCst);
            if (0..8).contains(&slot) {
                if let Some(item) = unit.item_list.get_item(slot) {
                    if item.item.iid.get_string().ok().as_deref() == Some(CELICA_RAGNAROK_IID) {
                        return;
                    }
                }
            }
            TEMP_OWNER.store(0, Ordering::SeqCst);
            TEMP_SLOT.store(-1, Ordering::SeqCst);
        }
    }

    let Some(ragnarok) = ItemData::get(CELICA_RAGNAROK_IID) else {
        println!("[engage_attack_evolution] Ragnarok IID missing; leaving existing tome choices");
        return;
    };
    // The engine has eight physical slots. When all are full, never evict an
    // existing item or modify a user's actual inventory to make room.
    let index = unsafe { unititemlist_add(unit.item_list, ragnarok, None) };
    if index < 0 {
        println!("[engage_attack_evolution] no free item slot for temporary Ragnarok");
        return;
    }
    let Some(inserted) = unit.item_list.get_item(index) else {
        // Native Add should always return a valid slot on success.
        println!("[engage_attack_evolution] temporary Ragnarok add returned invalid slot {}", index);
        return;
    };
    // Native method applies Celica's stored weapon Augments, including
    // upgrades to stats and skills. Do not create a plain, unaugmented copy.
    unsafe { set_engage_weapon_god(inserted, unit, None) };
    TEMP_SLOT.store(index, Ordering::SeqCst);
    TEMP_OWNER.store(owner, Ordering::SeqCst);
    println!(
        "[engage_attack_evolution] temporary augmented Ragnarok available: unit={} bond={} slot={}",
        unit.ident, bond, index
    );
}

// This is the common target-selection entry point, reached also by the
// normal attack dispatch used for skills upgraded by get_engage_attack.
// Hooking CreateBindEngageAttack alone did not fire in the actual link-10
// save (diagnostic v1), because the game can dispatch through CreateBindAttack.
// Insert *before* CreateBindTarget enumerates and creates the weapon menu.
#[skyline::hook(offset = 0x01DEDC80)]
fn create_bind_target(
    menu: *mut u8,
    mind: i32,
    mask: i32,
    skill: *const SkillData,
    method: OptionalMethod,
) {
    if mind == 3 || mind == 6 {
        if let Some(skill) = unsafe { skill.as_ref() } {
            let is_plus = is_celica_plus_skill(skill);
            println!(
                "[engage_attack_evolution] picker target hook: mind={} mask={} plus={}",
                mind, mask, is_plus
            );
            if mind == 6 || is_plus {
                offer_temporary_ragnarok(skill, mind == 6);
            }
        } else {
            println!("[engage_attack_evolution] picker target hook: mind={} null skill", mind);
        }
    }
    call_original!(menu, mind, mask, skill, method);
}

// These transitions end the selection/attack flow. Keeping the item until a
// transition (rather than removing it when the item menu closes) ensures that
// the target forecast and battle can still refer to the selected slot.
#[skyline::hook(offset = 0x02671410)]
fn unit_command_prepare(sequence: *mut u8, method: OptionalMethod) {
    clear_temporary_ragnarok("command prepare");
    call_original!(sequence, method);
}

#[skyline::hook(offset = 0x02671850)]
fn unit_command_cancel(sequence: *mut u8, method: OptionalMethod) {
    clear_temporary_ragnarok("command cancel");
    call_original!(sequence, method);
}

#[skyline::hook(offset = 0x0265CF60)]
fn free_cursor_prepare(sequence: *mut u8, method: OptionalMethod) {
    clear_temporary_ragnarok("free cursor");
    call_original!(sequence, method);
}

#[skyline::hook(offset = 0x0265B760)]
fn human_map_dispose(sequence: *mut u8, method: OptionalMethod) {
    clear_temporary_ragnarok("map dispose");
    call_original!(sequence, method);
}

#[skyline::main(name = "engage_attack_evolution")]
pub fn main() {
    skyline::install_hooks!(get_engage_attack, create_bind_target, unit_command_prepare, unit_command_cancel, free_cursor_prepare, human_map_dispose);
    println!("[engage_attack_evolution] loaded (Engage 2.0.0, Celica level 10; picker target-hook build v2)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ragnarok_temporary_scope() {
        assert!(!should_offer_temporary_ragnarok(9, false));
        assert!(should_offer_temporary_ragnarok(10, false));
        assert!(should_offer_temporary_ragnarok(14, false));
        assert!(!should_offer_temporary_ragnarok(15, false));
        assert!(!should_offer_temporary_ragnarok(10, true));
    }

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
        assert_eq!(variant_suffix("SID_セリカエンゲージ技＋", rule), None);
    }
}
