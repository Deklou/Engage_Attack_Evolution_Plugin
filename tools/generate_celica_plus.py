#!/usr/bin/env python3
"""Generate Celica Warp Ragnarok+ rows without changing the original Skill.xml."""
from __future__ import annotations

import argparse
import codecs
import re
from pathlib import Path

BASE = "SID_セリカエンゲージ技"
PLUS = "SID_セリカエンゲージ技_Plus"
SUFFIXES = ("", "_竜族", "_騎馬", "_飛行", "_魔法")
HELP_SUFFIX = {
    "": "",
    "_竜族": "_Dragon",
    "_騎馬": "_Horse",
    "_飛行": "_Fly",
    "_魔法": "_Magic",
}
SID_RE = re.compile(r'\bSid="([^"]+)"')
SHEET_RE = re.compile(r'(<Sheet\s+Name="スキル"\s+Count=")(\d+)(")')


def replace_attr(line: str, attr: str, value: str) -> str:
    regex = re.compile(r'(\b' + re.escape(attr) + r'=")[^"]*(")')
    result, count = regex.subn(lambda m: m.group(1) + value + m.group(2), line, count=1)
    if count != 1:
        raise ValueError(f"Missing attribute: {attr}")
    return result


def generate(source: bytes) -> bytes:
    bom = source.startswith(codecs.BOM_UTF8)
    text = source.decode("utf-8-sig" if bom else "utf-8")
    lines = text.splitlines(keepends=True)
    wanted = {BASE + suffix: suffix for suffix in SUFFIXES}
    original_rows: dict[str, str] = {}
    last_row_index = -1

    for index, line in enumerate(lines):
        match = SID_RE.search(line)
        if match is None:
            continue
        sid = match.group(1)
        if sid.startswith(PLUS):
            raise ValueError(f"Upgrade skill already exists: {sid}")
        if sid in wanted:
            if sid in original_rows:
                raise ValueError(f"Duplicate source skill: {sid}")
            original_rows[sid] = line
            last_row_index = index

    missing = sorted(set(wanted) - set(original_rows))
    if missing:
        raise ValueError("Missing source variants: " + ", ".join(missing))

    additions = []
    for suffix in SUFFIXES:
        row = original_rows[BASE + suffix]
        row = replace_attr(row, "Sid", PLUS + suffix)
        row = replace_attr(row, "Name", "MSID_CelicaEngageAtkPlus")
        row = replace_attr(row, "Help", "MSID_H_CelicaEngageAtkPlus" + HELP_SUFFIX[suffix])
        row = replace_attr(row, "CommandName", "MSID_CelicaEngageAtkPlus")
        row = replace_attr(row, "CommandHelp", "MSID_H_CelicaEngageAtkPlus_CMD")
        row = replace_attr(row, "EquipIids", "")
        # 10 weapon-kind bits, allow only kind 6 (tomes): 1023 - (1 << 6) = 959.
        row = replace_attr(row, "WeaponProhibit", "959")
        # The root attack must resolve to its OWN class-style variants.
        for style_suffix in SUFFIXES[1:]:
            row = row.replace(BASE + style_suffix, PLUS + style_suffix)
        additions.append(row)

    sheet_indices = [i for i, line in enumerate(lines) if SHEET_RE.search(line)]
    if len(sheet_indices) != 1:
        raise ValueError("Expected one スキル sheet with Count attribute")
    header = sheet_indices[0]
    lines[header], changed = SHEET_RE.subn(
        lambda m: m.group(1) + str(int(m.group(2)) + len(additions)) + m.group(3),
        lines[header],
        count=1,
    )
    if changed != 1:
        raise ValueError("Unable to update skill sheet count")

    lines[last_row_index + 1:last_row_index + 1] = additions
    result = "".join(lines)
    return result.encode("utf-8-sig" if bom else "utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Existing Skill.xml")
    parser.add_argument("output", type=Path, help="NEW output path (never overwrites source)")
    args = parser.parse_args()
    if args.source.resolve() == args.output.resolve():
        parser.error("Input and output paths must be different")
    if not args.source.is_file():
        parser.error(f"Source not found: {args.source}")
    if args.output.exists():
        parser.error(f"Output already exists: {args.output}")
    result = generate(args.source.read_bytes())
    args.output.write_bytes(result)
    print(f"Generated {args.output} with {len(SUFFIXES)} additional skills")


if __name__ == "__main__":
    main()
