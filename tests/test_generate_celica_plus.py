"""Non-destructive standard-library tests: python3 -m unittest discover -s tests."""
import codecs
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from generate_celica_plus import BASE, PLUS, SUFFIXES, generate


class GenerateTests(unittest.TestCase):
    def xml(self, newline="\n", with_bom=False):
        header = '<Book Count="1">' + newline + '<Sheet Name="スキル" Count="5">' + newline
        rows = []
        for suffix in SUFFIXES:
            rows.append(
                '<Param Sid="' + BASE + suffix + '" Name="OLD" Help="OLD" '
                'CommandName="OLD" CommandHelp="OLD" EquipIids="IID_セリカ_ライナロック;" '
                'WeaponProhibit="0" HorseSkill="' + BASE + '_騎馬" />' + newline
            )
        text = header + "".join(rows) + "</Sheet>" + newline + "</Book>" + newline
        return text.encode("utf-8-sig" if with_bom else "utf-8")

    def test_generates_five_variants_without_changing_existing_rows(self):
        source = self.xml()
        result = generate(source).decode("utf-8")
        self.assertIn('<Sheet Name="スキル" Count="10">', result)
        for suffix in SUFFIXES:
            self.assertIn('Sid="' + BASE + suffix + '"', result)
            self.assertIn('Sid="' + PLUS + suffix + '"', result)
        self.assertIn('HorseSkill="' + PLUS + '_騎馬"', result)
        self.assertEqual(result.count('WeaponProhibit="959"'), 5)
        self.assertEqual(result.count('EquipIids=""'), 5)

    def test_preserves_windows_newlines_and_bom(self):
        source = self.xml("\r\n", with_bom=True)
        output = generate(source)
        self.assertTrue(output.startswith(codecs.BOM_UTF8))
        self.assertNotIn(b"\n", output.replace(b"\r\n", b""))

    def test_rejects_duplicate_generation(self):
        with self.assertRaisesRegex(ValueError, "already exists"):
            generate(generate(self.xml()))


if __name__ == "__main__":
    unittest.main()
