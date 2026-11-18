"""Non-destructive standard-library tests: python3 -m unittest discover -s tests."""
import codecs
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from generate_celica_plus import BASE, PLUS, ICON, SUFFIXES, generate, replace_attr


class GenerateTests(unittest.TestCase):
    def xml(self, newline="\n", with_bom=False, existing_plus=False):
        header = '<Book Count="1">' + newline + '<Sheet Name="スキル" Count="' + ("6" if existing_plus else "5") + '">' + newline
        rows = []
        for suffix in SUFFIXES:
            rows.append(
                '<Param Sid="' + BASE + suffix + '" Name="OLD" Help="OLD" '
                'CommandName="OLD" CommandHelp="OLD" IconLabel="" '
                'EquipIids="IID_セリカ_ライナロック;" '
                'WeaponProhibit="0" Rewarp="12" HorseSkill="' + BASE + '_騎馬" />' + newline
            )
        if existing_plus:
            # Mirrors the user-authored full-width + root: only one new row,
            # custom translation keys, vanilla references still present.
            root = rows[0]
            plus_root = root
            for key, value in (
                ("Sid", PLUS),
                ("Name", "CUSTOM_PLUS"),
                ("CommandName", "CUSTOM_COMMAND"),
                ("Help", "CUSTOM_HELP"),
                ("CommandHelp", "CUSTOM_COMMAND_HELP"),
            ):
                plus_root = replace_attr(plus_root, key, value)
            rows.insert(1, plus_root)
        text = header + "".join(rows) + "</Sheet>" + newline + "</Book>" + newline
        return text.encode("utf-8-sig" if with_bom else "utf-8")

    def test_generates_five_variants_from_vanilla(self):
        result = generate(self.xml()).decode("utf-8")
        self.assertIn('<Sheet Name="スキル" Count="10">', result)
        for suffix in SUFFIXES:
            self.assertIn('Sid="' + BASE + suffix + '"', result)
            self.assertIn('Sid="' + PLUS + suffix + '"', result)
        self.assertIn('HorseSkill="' + PLUS + '_騎馬"', result)
        self.assertEqual(result.count('WeaponProhibit="959"'), 5)
        self.assertEqual(result.count('EquipIids=""'), 5)
        self.assertEqual(result.count('IconLabel="' + ICON + '"'), 5)

    def test_completes_existing_plus_without_losing_custom_text_keys(self):
        result = generate(self.xml(existing_plus=True)).decode("utf-8")
        self.assertIn('<Sheet Name="スキル" Count="10">', result)
        self.assertEqual(result.count('Sid="' + PLUS + '"'), 1)
        self.assertEqual(result.count('Name="CUSTOM_PLUS"'), 5)
        self.assertEqual(result.count('CommandName="CUSTOM_COMMAND"'), 5)
        self.assertEqual(result.count('CommandHelp="CUSTOM_COMMAND_HELP"'), 5)
        self.assertEqual(result.count('Help="CUSTOM_HELP"'), 5)
        self.assertEqual(result.count('WeaponProhibit="959"'), 5)
        self.assertIn('Rewarp="12"', result)
        self.assertIn('HorseSkill="' + PLUS + '_騎馬"', result)
        for suffix in SUFFIXES:
            self.assertIn('Sid="' + PLUS + suffix + '"', result)

    def test_preserves_windows_newlines_and_bom(self):
        source = self.xml("\r\n", with_bom=True, existing_plus=True)
        output = generate(source)
        self.assertTrue(output.startswith(codecs.BOM_UTF8))
        self.assertNotIn(b"\n", output.replace(b"\r\n", b""))

    def test_refuses_to_run_on_complete_xml(self):
        with self.assertRaisesRegex(ValueError, "already complete"):
            generate(generate(self.xml()))


if __name__ == "__main__":
    unittest.main()
