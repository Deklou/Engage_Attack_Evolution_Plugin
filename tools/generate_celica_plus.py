#!/usr/bin/env python3
"""Complete Celica Warp Ragnarok+ styles from Skill.xml without overwriting it.

Supports the user's already-created SID_セリカエンゲージ技＋ root row,
including its custom text IDs and icon. Four class-style rows are added.
"""
from __future__ import annotations

import argparse
import codecs
import re
from pathlib import Path

BASE = "SID_セリカエンゲージ技"
PLUS = "SID_セリカエンゲージ技＋"  # Full-width '+' in the actual modpack.
SUFFIXES = ("", "_竜族", "_騎馬", "_飛行", "_魔法")
ICON = "セリカエンゲージ技＋"
SHEET_RE = re.compile(r'(<Sheet\s+Name="スキル"\s+Count=")(\d+)(")')
SID_RE = re.compile(r'\bSid="([^"]+)"')


def get_attr(row: str, name: str) -> str:
    match = re.search(r'\b' + re.escape(name) + r'="([^"]*)"', row)
    if match is None:
        raise ValueError(f"Missing XML attribute: {name}")
    return match.group(1)


def replace_attr(row: str, name: str, value: str) -> str:
    regex = re.compile(r'(\b' + re.escape(name) + r'=")[^"]*(")')
    result, count = regex.subn(lambda m: m.group(1) + value + m.group(2), row, count=1)
    if count != 1:
        raise ValueError(f"Missing XML attribute: {name}")
    return result


def upgrade_row(row: str, sid: str, reference: str) -> str:
    """Keep source values like Rewarp/ActValues but adapt upgrade-specific fields."""
    for key, value in (
        ("Sid", sid),
        ("Name", get_attr(reference, "Name")),
        ("Help", get_attr(reference, "Help")),
        ("CommandName", get_attr(reference, "CommandName")),
        ("CommandHelp", get_attr(reference, "CommandHelp")),
        ("IconLabel", ICON),
        ("EquipIids", ""),
        ("WeaponProhibit", "959"),
    ):
        row = replace_attr(row, key, value)
    # Only redirect class-style links, not the root SID or arbitrary skill IDs.
    for suffix in SUFFIXES[1:]:
        row = row.replace(BASE + suffix, PLUS + suffix)
    return row


def generate(source: bytes) -> bytes:
    bom = source.startswith(codecs.BOM_UTF8)
    text = source.decode("utf-8-sig" if bom else "utf-8")
    lines = text.splitlines(keepends=True)

    old_rows: dict[str, str] = {}
    plus_rows: dict[str, tuple[int, str]] = {}
    old_end = -1
    old_wanted = {BASE + suffix for suffix in SUFFIXES}
    plus_wanted = {PLUS + suffix for suffix in SUFFIXES}

    for index, row in enumerate(lines):
        match = SID_RE.search(row)
        if match is None:
            continue
        sid = match.group(1)
        if sid in old_wanted:
            if sid in old_rows:
                raise ValueError(f"Duplicate vanilla skill: {sid}")
            old_rows[sid] = row
            old_end = index
        if sid in plus_wanted:
            if sid in plus_rows:
                raise ValueError(f"Duplicate upgraded skill: {sid}")
            plus_rows[sid] = (index, row)

    missing = sorted(old_wanted.difference(old_rows))
    if missing:
        raise ValueError("Missing vanilla variants: " + ", ".join(missing))
    if len(plus_rows) == len(SUFFIXES):
        raise ValueError("Upgrade already complete; no XML changes needed")
    if any(sid != PLUS for sid in plus_rows):
        raise ValueError("Some upgraded style variants already exist; review manually")

    # If the root + row was authored manually, preserve its text IDs.
    # Otherwise create the root + from vanilla with a conservative name scheme.
    if PLUS in plus_rows:
        root_index, reference = plus_rows[PLUS]
    else:
        root_index = None
        reference = old_rows[BASE]
        for key, value in (
            ("Sid", PLUS),
            ("Name", "MSID_CelicaEngageAtk＋"),
            ("Help", "MSID_H_CelicaEngageAtk＋"),
            ("CommandName", "MSID_CelicaEngageAtk＋"),
            ("CommandHelp", "MSID_H_CelicaEngageAtk_CMD＋"),
        ):
            reference = replace_attr(reference, key, value)

    additions: list[str] = []
    if root_index is not None:
        lines[root_index] = upgrade_row(reference, PLUS, reference)
    else:
        additions.append(upgrade_row(reference, PLUS, reference))

    for suffix in SUFFIXES[1:]:
        row = old_rows[BASE + suffix]
        additions.append(upgrade_row(row, PLUS + suffix, reference))

    headers = [index for index, row in enumerate(lines) if SHEET_RE.search(row)]
    if len(headers) != 1:
        raise ValueError("Expected exactly one スキル sheet header")
    header_index = headers[0]
    lines[header_index], count = SHEET_RE.subn(
        lambda m: m.group(1) + str(int(m.group(2)) + len(additions)) + m.group(3),
        lines[header_index],
        count=1,
    )
    if count != 1:
        raise ValueError("Could not update XML skill count")

    lines[old_end + 1:old_end + 1] = additions
    return "".join(lines).encode("utf-8-sig" if bom else "utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Current Skill.xml with or without the + row")
    parser.add_argument("output", type=Path, help="NEW output file; never overwrites input")
    args = parser.parse_args()
    if args.source.resolve() == args.output.resolve():
        parser.error("Input and output paths must be different")
    if not args.source.is_file():
        parser.error(f"Missing source: {args.source}")
    if args.output.exists():
        parser.error(f"Output exists: {args.output}")
    result = generate(args.source.read_bytes())
    args.output.write_bytes(result)
    print(f"Created {args.output}: Celica's five + variants are ready")


if __name__ == "__main__":
    main()
