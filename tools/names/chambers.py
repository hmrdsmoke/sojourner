"""Chambers's Twentieth Century Dictionary (1908), "Pronouncing Vocabulary of
Scripture Proper Names": its respelling → espeak-ng "en" mnemonics.

The vocabulary writes each name in syllables with a stress mark (′) after the
accented syllable, long vowels with a macron (ā ē ī ō ū), ç for a soft c;
ch and unmarked c are k, g is hard. Its key: "This vocabulary contains all
common Scripture Names except monosyllables and dissyllables, the latter
being always accented on the first syllable." A parenthetical after an entry
respells part of it ("Tig-lath-pi-lē′ser (′zer)"); an "or" alternative is
ignored in favour of the first form. Used by compose.py; the rules are
recorded in assets/SOURCES.md section 5.
"""
import re, sys, unicodedata

def load_entries(path):
    s = open(path, encoding='utf-8').read()
    body = s.split('{{block center/s}}')[1]
    entries = []
    for l in body.splitlines():
        l = l.strip()
        if not l: continue
        m = re.match(r"^\|.*\|\s*(.+)$", l)
        if l.startswith('|') or l.startswith('{|') or l.startswith('{{'):
            if m and '<br' in l: entries.append(m.group(1))
            continue
        entries.append(l)
    entries = [re.sub(r'<br\s*/?>', '', e).strip() for e in entries]
    entries = [e for e in entries if e and not e.startswith('{{') and not e.startswith('|')]
    entries = [re.sub(r'<span style="text-decoration: overline;">y</span>', 'ȳ', e) for e in entries]
    return entries

LONG = {'ā':'A','ē':'E','ī':'I','ō':'O','ū':'U','ȳ':'I'}  # long-vowel classes
def strip_marks(w):
    w = unicodedata.normalize('NFD', w)
    w = ''.join(c for c in w if not unicodedata.combining(c))
    return w.replace('ç','c').replace('æ','ae').replace('Æ','Ae').replace('ȳ','y')

def headword(main):
    return strip_marks(main.replace('′','').replace('-',''))

def split_entry(e):
    """→ (main form, first parenthetical fragment or None)"""
    e = e.strip().rstrip('.')
    paren = None
    m = re.match(r"^(.*?)\s*\((.*)\)\s*$", e)
    if m:
        e, paren = m.group(1).strip(), m.group(2).strip()
    e = e.replace("''or''", ' or ')
    forms = e.split(' or ')[0].split(',')
    main = forms[0].strip()   # "Am′a-na, A-mā′na" → first
    frag = None
    # a parenthetical after a list of forms qualifies the last form, not the first
    if paren is not None and len(forms) == 1:
        p = paren.replace("''or''", ' or ')
        p = re.sub(r'\s+', ' ', p)
        first = p.split(' or ')[0].strip().rstrip(',').strip()
        if first and not first.startswith('or') and not first.startswith('Heb'):
            frag = first
    return main, frag

def syllables(form):
    """'A-bī′jah' → ['A','bī′','jah']: the stress mark ends its syllable
    the way a hyphen does (Chambers writes "A-bad′don", not "A-bad′-don")."""
    return [x for x in form.replace('′', '′-').split('-') if x]

def apply_fragment(syls, frag):
    """Overlay a respelling fragment onto the syllable list, by the rules
    read off the 107 parenthetical entries. Returns new syllables, or None
    if the fragment can't be placed."""
    lead = frag.startswith('-'); trail = frag.endswith('-')
    core = frag.strip('-')
    after_stress = core.startswith('′')
    fs = syllables(core.lstrip('′'))
    n = len(syls); k = len(fs)
    stressed = next((i for i, s in enumerate(syls) if '′' in s), None)
    has_stress = any('′' in s for s in fs)
    if k == 0: return None
    if after_stress:
        # "′zer": the syllables after the stressed one
        if stressed is None: return None
        start = stressed + 1
        if start + k > n: return None
        # the stress stays on the main form's stressed syllable
        return syls[:start] + fs + syls[start + k:]
    if lead and not trail:
        if has_stress:
            if stressed is None: return None
            start = stressed
        else:
            start = n - k
        if start < 0 or start + k > n: return None
        return syls[:start] + fs + syls[start + k:]
    if trail and not lead:
        if k > n: return None
        return fs + syls[k:]
    # no dashes
    if k == n:
        return fs
    if has_stress:
        # ends at the stressed syllable: leading part
        if stressed is None: return None
        if fs[-1].endswith('′') and k == stressed + 1:
            return fs + syls[k:]
        if k == 1:
            return syls[:stressed] + fs + syls[stressed + 1:]
    return None

VOW = 'aeiouyāēīōūȳæ'
def is_vowel(c): return c in VOW

def convert(syls):
    """Syllables (with ′ after the stressed one) → espeak mnemonics."""
    stressed = next((i for i, s in enumerate(syls) if '′' in s), None)
    if stressed is None:
        # Chambers left the mark off ("Nin-e-veh"): the first syllable, as for its dissyllables
        stressed = 0
    out = []
    for i, syl in enumerate(syls):
        raw = syl.replace('′', '').lower()
        stress = (i == stressed)
        last = (i == len(syls) - 1)
        nxt = syls[i + 1].replace('′', '').lower() if not last else ''
        prev = syls[i - 1].replace('′', '').lower() if i > 0 else ''
        prev_coda = re.sub(r"^.*[aeiouyāēīōūȳæ]", "", prev)
        ph = syllable_phonemes(raw, stress, last, nxt, i == 0, prev_coda, next_stressed=(i + 1 == stressed))
        if ph is None: return None
        out.append(("'" if stress else '') + ph)
    joined = ''.join(out)
    # a doubled consonant across a syllable break is one sound ("Abad-don"),
    # and two schwas in a row are one ("Mi-cha-el")
    joined = re.sub(r"([bdfgklmnprstvzTDSZN])('?)\1", r"\2\1", joined)
    return joined.replace('@@', '@')

def syllable_phonemes(s, stress, last, nxt, first, prev_coda='', next_stressed=False):
    """One syllable: onset consonants, vowel nucleus, coda consonants."""
    # a silent final e after a consonant, with an earlier vowel: "īte", "ēnes", "cūse"
    m2 = re.match(r"^(.*[aeiouyāēīōūȳæ][^aeiouyāēīōūȳæ]+)e(s?)$", s)
    if m2:
        s = m2.group(1) + m2.group(2)
    # y before a vowel is the consonant j ("ya")
    yc = ''
    if len(s) > 1 and s[0] == 'y' and is_vowel(s[1]):
        yc, s = 'j', s[1:]
    # split into consonant/vowel runs; a w right after a vowel is part of it (aw, ew, ow)
    m = re.match(r"^([^aeiouyāēīōūȳæ]*)([aeiouyāēīōūȳæ]+w?)([^aeiouyāēīōūȳæ]*)$", s)
    if not m:
        # a syllable without a vowel (e.g. 'chr' in 'Cen-chre-a'?) — treat as consonants only
        if re.match(r"^[^aeiouyāēīōūȳæ]+$", s):
            return consonants(s, first, before_vowel=True)
        return None
    onset, nucleus, coda = m.groups()
    # r-coloring: 'r' in the coda (before a consonant or at syllable end) is dropped (espeak "en" is non-rhotic)
    # unless the next syllable starts with a vowel (then r links).
    r_drop = False
    if coda.startswith('r') and not (nxt and is_vowel(nxt[0])):
        coda = coda[1:]; r_drop = True
    on = consonants(onset, first, before_vowel=True)
    before = onset if onset else prev_coda
    v = vowel(nucleus, stress, last and coda == '' , r_drop, coda, before, next_starts_with_vowel=(coda == '' and bool(nxt) and is_vowel(nxt[0])))
    if v is None: return None
    co = consonants(coda, False, before_vowel=False)
    if on is None or co is None: return None
    # x before a stressed syllable that starts with a vowel is voiced ("Alexandria")
    if co.endswith('ks') and coda.endswith('x') and nxt and is_vowel(nxt[0]) and next_stressed:
        co = co[:-2] + 'gz'
    # a plural s after m, n, l or r in the last syllable is voiced ("Anakims", "Herodians")
    if last and co.endswith('s') and len(co) >= 2 and co[-2] in 'mnlr':
        co = co[:-1] + 'z'
    return yc + on + v + co

def vowel(nuc, stress, final_open, r_drop, coda, onset, next_starts_with_vowel=False):
    n = nuc
    plain = strip_marks(n)
    # digraphs, marked or not
    if plain in ('ai', 'ay'): return 'eI'
    if plain in ('ei', 'ey'): return 'eI'
    if plain in ('au', 'aw'): return 'O:'
    if plain in ('oi', 'oy'): return 'OI'
    if plain in ('ou', 'ow'): return 'aU'
    if plain == 'oo': return 'u:'
    if plain == 'ee': return 'i:'
    if plain == 'ea': return 'i:'
    if plain == 'ew': return 'ju:'
    if plain == 'eu': return 'ju:'
    if plain == 'ie': return 'i:' if stress else 'I'
    if plain == 'ia': return 'i@'
    if plain == 'io': return 'i@'
    if plain == 'ua': return 'ju@'
    if plain == 'ue': return 'ju:'
    if plain == 'ui': return 'u:I'
    if plain == 'ao': return 'oU'
    if plain == 'aa': return 'a' if stress else '@'
    if plain in ('ae', 'æ'): return 'i:'
    if len(n) == 2 and n[0] in LONG and n[1] in 'aeiou':   # "ā-i" written together? e.g. 'ūē'
        return vowel(n[0], stress, final_open, r_drop, coda, onset) + vowel(n[1], False, final_open, r_drop, coda, onset)
    if len(n) != 1: return None
    c = n
    if c in LONG:
        cls = LONG[c]
        if r_drop:
            return {'A':'e@','E':'i@','I':'aI@','O':'O:','U':'jU@'}[cls]
        return {'A':'eI','E':'i:','I':'aI','O':'oU','U':('u:' if onset[-1:] in ('l','r','j','s','z') or onset.endswith('sh') or onset.endswith('ch') else 'ju:')}[cls]
    # short vowels
    if r_drop:
        if stress:
            return {'a':'A:','e':'3:','i':'3:','o':'O:','u':'3:','y':'3:'}[c]
        return {'a':'@','e':'@','i':'@','o':'@','u':'@','y':'@'}[c]
    if stress:
        return {'a':'a','e':'E','i':'I','o':'0','u':'V','y':'I'}[c]
    if final_open:
        return {'a':'@','e':'i','i':'i','o':'oU','u':'u:','y':'i'}[c]
    if next_starts_with_vowel and c in 'ei':
        return 'i'
    return {'a':'@','e':'@','i':'I','o':'@','u':'@','y':'I'}[c]

CONS = [
    ('tch','tS'),('sch','sk'),('ch','k'),('sh','S'),('ph','f'),('th','T'),('zh','Z'),('wh','w'),('qu','kw'),('ck','k'),('dg','dZ'),('ng','N'),('gh','g'),('rh','r'),
    ('b','b'),('c','k'),('ç','s'),('d','d'),('f','f'),('g','g'),('h','h'),('j','dZ'),('k','k'),('l','l'),('m','m'),('n','n'),('p','p'),('q','k'),('r','r'),('s','s'),('t','t'),('v','v'),('w','w'),('x','ks'),('z','z'),
]
def consonants(s, first, before_vowel):
    out = ''
    i = 0
    prev = ''
    # a lone h after a vowel is silent ("-jah", "-seh")
    if not before_vowel and s.endswith('h') and not s.endswith(('th', 'sh', 'ch', 'ph', 'zh', 'gh')):
        s = s[:-1]
    while i < len(s):
        for k, v in CONS:
            if s.startswith(k, i):
                if v != prev or k == 'ng':   # collapse doubles (tt, ss, ll)
                    out += v
                prev = v
                i += len(k)
                break
        else:
            return None
    return out

# Entries whose parenthetical the rules cannot read; left to the voice.
UNREADABLE = {'Hig-gāi′on'}
