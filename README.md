# Engage Attack Evolution Plugin

Fire Emblem Engage **2.0.0** · **Skyline / Cobalt** · Rust.

Premier cas : l'attaque de fusion de **Célica** évolue automatiquement en **Warp Ragnarok+** au **niveau de lien 10**. L'attaque originale reste disponible aux niveaux 1–9 et n'est pas ajoutée en double au menu.

## Fonctionnement

Le hook intercepte `App.GodUnit.GetEngageAttack(GodUnit, Unit)` (offset `0x02341640` dans `main_200`), appelle d'abord le jeu, puis applique les règles définies dans `src/lib.rs`. Il utilise `GodUnit.GetLevel(GodUnit, Unit)` (`0x023302C0`), qui renvoie le **niveau de lien numérique**, et non le rang de soutien.

Configuration initiale :

| Emblème | Niveau | SID original | SID amélioré |
| --- | --- | --- | --- |
| Célica (`GID_セリカ`) | ≥ 10 | `SID_セリカエンゲージ技` | `SID_セリカエンゲージ技_Plus` |

Les variantes `_竜族`, `_騎馬`, `_飛行`, `_魔法` sont conservées. Les variantes ennemies `_闇` ne sont pas touchées. Le hook ne modifie **ni God.xml ni le menu**, et restitue l'attaque native si la version améliorée manque.

L'ajout d'autres emblèmes se fait en étendant `EVOLUTIONS` dans `src/lib.rs`.

## Préparer les XML

Le plugin **n'invente pas les données du skill** : les cinq SID améliorés doivent exister dans le `Skill.xml` effectivement chargé par Cobalt. Ce dépôt propose un générateur conservateur, à utiliser sur une **copie** de ton fichier actuel :

~~~bash
python3 tools/generate_celica_plus.py /chemin/vers/Skill.xml /tmp/Skill.plus.xml
~~~

Le script refuse d'écraser le fichier source ou un fichier de sortie déjà existant. Il duplique les cinq lignes de Célica, ajuste les références entre variantes, incrémente le `Count` de la feuille et utilise :
- `EquipIids=""` : l'attaque n'impose plus Ragnarok ;
- `WeaponProhibit="959"` : masque visant à n'autoriser que les tomes (kind 6, à valider en jeu) ;
- les nouveaux identifiants `MSID_CelicaEngageAtkPlus`, `MSID_H_CelicaEngageAtkPlus`, `MSID_H_CelicaEngageAtkPlus_Dragon`, `_Horse`, `_Fly`, `_Magic` et `MSID_H_CelicaEngageAtkPlus_CMD`.

**À faire manuellement :** ajouter les messages dans tes fichiers MSBT (nom, aide et description), puis **après sauvegarde du XML existant et jeu fermé** installer la copie résultante comme `Emblem_Rework/patches/xml/Skill.xml`. Les fichiers XML et MSBT de `engage_modpack` ne sont pas modifiés depuis ce dépôt.

Le générateur de XML ne garantit **pas** que le moteur affiche une liste de tomes à choisir : ce comportement doit être confirmé par un test en jeu. Il ne modifie pas les animations.

Test du générateur, sans jeu ni mod installé :

~~~bash
python3 -m unittest discover -s tests
~~~

## Compiler / tester (WSL Ubuntu)

Cloner ou ouvrir **ce dépôt dédié**, et se placer sur la branche de la PR. Vérifier que Rust / Cargo Skyline sont accessibles, puis **fermer Ryujinx** :

~~~bash
bash scripts/deploy.sh
~~~

Le script compile avec `cargo skyline build --release`, puis installe le NRO uniquement dans `sdcard/engage/mods/Attack_Evolution/libengage_attack_evolution.nro`. Il sauvegarde un éventuel NRO précédent en `.bak`. Aucun autre plugin existant n'est remplacé. Utiliser `RYUJINX_DIR=/chemin/vers/Ryujinx` si le chemin WSL ne peut pas être détecté.

### Vérification en jeu

1. **Test de chargement** : chercher `[engage_attack_evolution] loaded` dans le log Ryujinx.
2. **Sans XML amélioré** : Célica à lien 10+ doit conserver Warp Ragnarok et loguer une fois `upgrade SID not found`.
3. **Avec XML amélioré et messages** : vérifier Célica aux niveaux **9**, **10** et **11**, puis avec les classes magie, dragon, cavalerie et volante : nom affiché, portée, destination de téléportation, prévisions, animations et résultat du combat.
4. **Armes** : avoir au moins deux tomes valides, vérifier la possibilité de choisir le tome, et contrôler que les armes non magiques ne passent pas. Si le moteur choisit un tome automatiquement, ce sera une évolution **distincte** du plugin.
5. **Régression** : un autre emblème doit conserver son attaque originale.

### État de validation

**Sources écrites, pas encore compilées ni testées dans Ryujinx.** Une PR en brouillon conserve `main` intacte. Les offsets et signatures ont été vérifiés dans Ghidra pour `main_200`. Le `Cargo.lock` de départ reprend les révisions de dépendances du Language Plugin ; son fonctionnement dans ce nouveau dépôt devra être confirmé par la compilation WSL.

En cas d'erreur de compilation ou de crash : arrêter le test et récupérer le log exact avant tout autre changement.
