#!/usr/bin/env python3
"""Builds assets/names.tsv — how the voice says the names of scripture.

Run from the repository root, after `cargo build` (it needs the espeak-ng
that espeak-rs-sys compiled under target/):

    cargo run --bin names-list > assets/names-source/web-names.tsv
    python3 tools/names/compose.py

Inputs, all in assets/names-source/ and all recorded in assets/SOURCES.md
section 5:

  web-names.tsv                         the text's proper names, with counts
  cmudict-names.txt                     the lines of the CMU Pronouncing
                                        Dictionary for those names (ARPAbet)
  chambers-1908-scripture-names.wikitext
                                        Chambers's Twentieth Century Dictionary
                                        (1908), "Pronouncing Vocabulary of
                                        Scripture Proper Names", from Wikisource

For every name: cmudict first (modern American usage), if its entry passes
two checks — every consonant the spelling has is in the pronunciation, in
order (cmudict has garbled entries for rare names), and a two-syllable name
is stressed on its first syllable (Chambers's stated rule for scripture
names). Chambers second; and Chambers over cmudict when both have the name
but disagree on its consonants. Each pronunciation is turned into espeak-ng
"en" mnemonics (chambers.py, cmu.py), and a row is written only when the
voice would otherwise say the name differently — espeak-ng is asked for
both renderings and they are compared — so every row in the table changes
something.

Output: assets/names.tsv, sorted by name.
"""
import glob
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(__file__))
import chambers  # noqa: E402
import cmu  # noqa: E402

SOURCE = 'assets/names-source'
OUT = 'assets/names.tsv'

# Decided by hand; the reasons are in SOURCES.md section 5.
VARIANT = {'Job': 1}       # cmudict's second entry: the man, not the work
EXCLUDE = set()            # names left to the voice on purpose (none, since the Updated edition)


def find_espeak():
    """The espeak-ng binary and data that espeak-rs-sys built, under target/."""
    target = os.environ.get('CARGO_TARGET_DIR', 'target')
    outs = sorted(glob.glob(f'{target}/*/build/espeak-rs-sys-*/out'), key=os.path.getmtime, reverse=True)
    for out in outs:
        binary = os.path.join(out, 'bin', 'espeak-ng')
        data = os.path.join(out, 'share', 'espeak-ng-data')
        if os.path.exists(binary) and os.path.isdir(data):
            return binary, data
    sys.exit('no espeak-ng build found under target/; run `cargo build` first')


ESPEAK, DATA = find_espeak()
ENV = dict(os.environ, ESPEAK_DATA_PATH=DATA)


def ipa(text):
    r = subprocess.run([ESPEAK, '-v', 'en', '-q', '--ipa', text], capture_output=True, text=True, env=ENV)
    return ' '.join(r.stdout.split())


def esp_skeleton(ph):
    """The consonants of an espeak mnemonic string, in cmudict's skeleton alphabet."""
    t = ph.replace("'", '').replace(',', '')
    for a, b in [('tS', 'k'), ('dZ', 'J'), ('S', 'S'), ('T', 'T'), ('D', 'T'), ('N', 'n'), ('Z', 'Z')]:
        t = t.replace(a, b)
    t = re.sub(r'[aeiouAEIOUV@03:]', '', t)
    for c in 'rhjw':
        t = t.replace(c, '')
    return t


def main():
    names = [l.rstrip('\n').split('\t') for l in open(f'{SOURCE}/web-names.tsv', encoding='utf-8')]
    names = [(w, int(n)) for w, n in names]

    c = cmu.load(f'{SOURCE}/cmudict-names.txt')
    ch = {}
    unread = []
    for e in chambers.load_entries(f'{SOURCE}/chambers-1908-scripture-names.wikitext'):
        main_form, frag = chambers.split_entry(e)
        if ' ' in main_form or main_form in chambers.UNREADABLE:
            unread.append(e)
            continue
        syls = chambers.syllables(main_form)
        if frag:
            placed = chambers.apply_fragment(syls, frag)
            if placed is None:
                unread.append(e)
                continue
            syls = placed
        ph = chambers.convert(syls)
        if ph is None:
            unread.append(e)
            continue
        ch.setdefault(chambers.headword(main_form), (ph, e))

    rows = []
    stats = {'cmudict': 0, 'chambers1908': 0, 'same as the voice': 0, 'cmudict rejected': 0, 'uncovered': 0}
    for word, count in names:
        if word in EXCLUDE:
            continue
        src = None
        prons = c.get(word.lower())
        if prons:
            idx = VARIANT.get(word)
            candidates = [prons[idx]] if idx is not None else prons
            for pron in candidates:
                if cmu.plausible(word, pron) and cmu.stresses_first_of_two(pron):
                    src = ('cmudict', cmu.convert(pron), pron)
                    break
            if src is None:
                stats['cmudict rejected'] += 1
        if src is not None and word in ch:
            ph, e = ch[word]
            if cmu.pron_skeleton(src[2]).replace('r', '') != esp_skeleton(ph):
                src = ('chambers1908', ph, e)
        if src is None and word in ch:
            ph, e = ch[word]
            src = ('chambers1908', ph, e)
        if src is None:
            stats['uncovered'] += 1
            continue
        source, ph, written = src
        if ipa(word) == ipa(f'[[{ph}]]'):
            stats['same as the voice'] += 1
            continue
        stats[source] += 1
        rows.append((word, ph, written.replace('\t', ' '), source))

    rows.sort()
    with open(OUT, 'w', encoding='utf-8') as f:
        f.write(HEADER)
        for word, ph, written, source in rows:
            f.write(f'{word}\t{ph}\t{written}\t{source}\n')
    print(f'{len(rows)} rows written to {OUT}: {stats}')
    print(f'{len(unread)} Chambers entries the rules cannot read: {unread}')


HEADER = '''# Sojourner — the names of scripture, and how the voice says them.
#
# Generated by tools/names/compose.py from the sources in assets/names-source/;
# do not edit by hand. Columns (tab-separated):
#   word        exactly as spelled in the World English Bible text (case matters)
#   phonemes    espeak-ng phoneme mnemonics for its English ("en") voice, as
#               written inside [[ ]] — ' before the stressed syllable
#   as written  the pronunciation as the source gives it, for checking
#   source      an id from assets/SOURCES.md section 5
#
# Every row changes something: a name the voice already says as the source
# says it gets no row.
#
'''

if __name__ == '__main__':
    main()
