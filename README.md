# Engage Attack Evolution Plugin

Fire Emblem Engage **2.0.0** · **Skyline / Cobalt** · Rust.

Warp Ragnarok de Célica devient automatiquement **Warp Ragnarok+** au **niveau de lien 10**. La version + remplace l'attaque originale : aucune entrée de menu supplémentaire.

## Fonctionnement

Le plugin intercepte `App.GodUnit.GetEngageAttack` (offset Skyline `0x02341640`), appelle la fonction native et remplace le skill retourné uniquement pour Célica, lien ≥ 10, lorsque la variante + correspondante est disponible. Le niveau numérique est obtenu via `GodUnit.GetLevel` (`0x023302C0`). Sinon, il conserve le comportement vanilla.

La configuration initiale est dans `src/lib.rs`. Le SID utilisé correspond à **ton XML réel** :

- Original : `SID_セリカエンゲージ技`
- Évolution : `SID_セリカエンゲージ技＋` (attention : `＋` pleine largeur, **pas** `_Plus`)
- Variantes : `＋_竜族`, `＋_騎馬`, `＋_飛行`, `＋_魔法`

Les variantes ennemies `_闇` et les autres emblèmes ne sont pas modifiés.

## Préparer le XML déjà ajouté dans engage_modpack

Au 18 novembre 2026, ton `Skill.xml` contient **la ligne Warp Ragnarok+ de base**, ainsi que son icône `セリカエンゲージ技＋.png`. Il manque les quatre variantes « + » de classe : la ligne de base pointe encore vers les variantes natives.

Le script `tools/generate_celica_plus.py` complète **ton XML existant** sans en écraser les données :

- Corrige les liens de la ligne « + » existante et conserve ses clés MSBT personnalisées.
- Ajoute quatre skills `＋_竜族`, `＋_騎馬`, `＋_飛行`, `＋_魔法`, en gardant les effets propres à chaque classe.
- Associe l'icône « + » aux cinq skills améliorés.
- Applique `EquipIids=""` et `WeaponProhibit="959"` pour autoriser uniquement les tomes (masque à valider en jeu).
- Met à jour le compteur XML, préserve le BOM et les retours ligne.

Depuis le dossier du plugin, en utilisant le `Skill.xml` **à jour** du modpack :

~~~bash
python3 tools/generate_celica_plus.py /chemin/vers/Skill.xml /tmp/Skill.celica-plus.xml
~~~

Le script refuse d'écraser son entrée ou sa sortie. Pour tester, jeu fermé, **sauvegarder le Skill.xml actif** puis y installer la **copie générée**.

**Option rapide pour ce test**, si le modpack actif dans Ryujinx contient déjà ta ligne « + » :

~~~bash
bash scripts/install_test_xml.sh
~~~

Cette commande génère et installe le XML à partir du dossier Ryujinx, avec une sauvegarde datée dans `Attack_Evolution/backups`. **Elle modifie le Skill.xml de ton dossier de mods actif** : ton dépôt local sera donc signalé comme modifié par Git. Elle ne touche pas au dépôt distant `engage_modpack` et ne doit être exécutée qu'avec Ryujinx fermé.

Les **traductions peuvent attendre** : le nom ou la description peut temporairement s'afficher comme une clé MSBT. Les variantes générées réutilisent les clés du skill + existant.

Test du générateur (facultatif) :

~~~bash
python3 -m unittest discover -s tests
~~~

## Compiler et déployer

Le plugin reste sur la branche de la [PR #1](https://github.com/Deklou/Engage_Attack_Evolution_Plugin/pull/1). Après mise à jour de la branche, fermer Ryujinx puis lancer :

~~~bash
bash scripts/deploy.sh
~~~

Le script compile et sauvegarde l'ancien NRO dans `.bak` avant d'installer uniquement `sdcard/engage/mods/Attack_Evolution/libengage_attack_evolution.nro`.

## Validation

Le log Ryujinx fourni le **18 novembre 2026** confirme le chargement de l'ancien build jusqu'à l'écran titre : `[engage_attack_evolution] loaded (Engage 2.0.0, Celica level 10)`. Le nouveau SID et le générateur nécessitent **un nouveau build et un test en combat**.

Scénario : Célica au lien **9** (attaque originale), puis au lien **10** (attaque +). Chercher `[engage_attack_evolution] Celica evolution active: level=10` dans le log. Avec une unité magique et au moins deux tomes utilisables, tester le choix du tome, la prévision, le ciblage, le déplacement, les animations et les dégâts. Vérifier ensuite la classe cavalière/volante/dragon et l'absence d'effet sur un autre emblème.

Si `upgrade SID not found` apparaît, le skill + correspondant n'est pas disponible. Si le moteur choisit automatiquement un tome, l'interface de sélection manuelle nécessitera un travail distinct.

**La PR reste en brouillon tant que le comportement en jeu n'est pas validé.**
