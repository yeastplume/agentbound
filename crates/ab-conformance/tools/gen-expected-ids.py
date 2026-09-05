#!/usr/bin/env python3
"""Derive the expected-ID manifest for the conformance runner from the frozen test catalogue.

Usage: gen-expected-ids.py docs/architecture/test-catalogue.md > crates/ab-conformance/expected-ids.txt

One line per catalogue row whose Milestone column includes 1A or 1B: `<id> <milestones> <section>`.
The runner (ab-conformance) treats this file as the population it MUST cover; a catalogue ID with no
strong-pass row is reported as not-executed/weak/recorded/fail and the run exits non-zero. The file is
checked in so a reviewer can diff it against the catalogue; regenerate and commit when the catalogue
revision changes (the header records the catalogue version it was derived from).
"""
import re, sys
src = sys.argv[1]
ver = None; section = None; out = []
for l in open(src):
    m = re.match(r"\*\*Version:\*\* ([0-9.]+)", l)
    if m: ver = m.group(1)
    m = re.match(r"^#{2,3} ([0-9.]+) ", l)
    if m: section = m.group(1)
    m = re.match(r"\| ((?:D|T|F|GS)-[A-Za-z0-9.\-]+) \| ([^|]*) \|", l)
    if m:
        ms = m.group(2).strip()
        if "1A" in ms or "1B" in ms:
            out.append((m.group(1), ms.replace(" ", ""), section or "?"))
print(f"# expected-ids: derived from test-catalogue.md {ver}; milestones 1A+1B; {len(out)} ids; regenerate with tools/gen-expected-ids.py")
print("# id milestones catalogue-section")
for i, ms, sec in out: print(f"{i} {ms} {sec}")
# ADR-0002 Decision 4/7 rows that are not catalogue rows but are required by the ADR (WP3 register §4); kept explicit so they are
# neither hidden in the catalogue count nor reported as unknown ids.
for i in ["D4", "D7-8", "D7-9"]: print(f"{i} 1B ADR-0002")
