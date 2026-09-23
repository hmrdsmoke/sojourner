"""The CMU Pronouncing Dictionary's ARPAbet → espeak-ng "en" mnemonics, by the
rule recorded in assets/SOURCES.md section 5, plus two checks on an entry:
that its consonants follow the spelling, and that a two-syllable name is
stressed on the first syllable. Used by compose.py."""
import re
VOW = {'AA':'A:','AE':'a','AH':None,'AO':'O:','AW':'aU','AY':'aI','EH':'E','ER':None,'EY':'eI','IH':'I','IY':'i:','OW':'oU','OY':'OI','UH':'U','UW':'u:'}
CON = {'B':'b','CH':'tS','D':'d','DH':'D','F':'f','G':'g','HH':'h','JH':'dZ','K':'k','L':'l','M':'m','N':'n','NG':'N','P':'p','R':'r','S':'s','SH':'S','T':'t','TH':'T','V':'v','W':'w','Y':'j','Z':'z','ZH':'Z'}
R_COLOR = {'A:':'A:','a':'A:','E':'e@','I':'i@','i:':'i@','O:':'O:','oU':'O:','U':'U@','u:':'U@','aI':'aI@','aU':'aU@','eI':'e@','V':'3:','@':'@'}
def load(path):
    cmu = {}
    for line in open(path, encoding='utf-8', errors='replace'):
        line = line.strip()
        if not line or line.startswith(';;;'): continue
        w, _, pron = line.partition(' ')
        base = re.sub(r'\(\d+\)$', '', w)
        pron = pron.split('#')[0].strip()
        cmu.setdefault(base, []).append(pron)
    return cmu
def convert(pron):
    ps = pron.split()
    out = []
    i = 0
    while i < len(ps):
        p = ps[i]
        m = re.match(r'^([A-Z]+)(\d)?$', p)
        base, st = m.group(1), m.group(2)
        if st is not None:   # a vowel
            if base == 'AH':
                v = '@' if st == '0' else 'V'
            elif base == 'ER':
                v = '3:' if st != '0' else '@'
            else:
                v = VOW[base]
            # r after this vowel, before a consonant or at the end: dropped, vowel colored
            if i + 1 < len(ps) and ps[i+1] == 'R' and (i + 2 >= len(ps) or not re.search(r'\d$', ps[i+2])):
                if base != 'ER':
                    v = R_COLOR.get(v, v)
                i += 1
            elif base == 'ER' and i + 1 < len(ps) and ps[i+1] == 'R':
                i += 1
            # ER before a vowel keeps its r ("Jerusalem" JH ER0 UW1)
            if base == 'ER' and i + 1 < len(ps) and re.search(r'\d$', ps[i+1]):
                v += 'r'
            mark = "'" if st == '1' else (',' if st == '2' else '')
            out.append(mark + v)
        else:
            out.append(CON[base])
        i += 1
    ph = ''.join(out)
    # ER followed by a vowel keeps its r ("Miriam" M IH1 R IY0 AH0 M has R before a vowel already)
    return ph
def spelling_skeleton(word):
    w = word.lower()
    w = re.sub(r'^p(?=[st])', '', w)          # Ptolemy, Psalms
    w = re.sub(r'c(?=[eiyæ]|ae)', 's', w)      # Caesar, Cilicia
    for a, b in [('tth','T'),('ph','f'),('ch','k'),('th','T'),('sh','S'),('ck','k'),('qu','k'),('x','ks'),('c','k'),('q','k'),('j','J'),('g','g')]:
        w = w.replace(a, b)
    w = re.sub(r'[aeiouyhw]', '', w)
    return re.sub(r'(.)\1', r'\1', w)
def pron_skeleton(pron):
    ps = [re.sub(r'\d','',p) for p in pron.split()]
    m = {'CH':'k','JH':'J','SH':'S','TH':'T','DH':'T','NG':'n','HH':'','Y':'','W':'','R':'r','ZH':'Z'}
    return ''.join(m.get(p, p.lower()) if p not in VOW else '' for p in ps)
def plausible(word, pron):
    """Every consonant the spelling has appears, in order, in the pronunciation."""
    a = spelling_skeleton(word).replace('r','')
    b = pron_skeleton(pron).replace('r','')
    # subsequence test, with g/J and s/z and k/S tolerated
    it = iter(b)
    equiv = {'g':'gJk','J':'Jg','s':'szS','z':'zs','k':'kSg','S':'Sks','T':'Tt','t':'tTS','d':'dt','f':'fv','v':'vf'}
    for ch in a:
        for x in it:
            if x == ch or x in equiv.get(ch, ''):
                break
        else:
            return False
    return True

def stresses_first_of_two(pron):
    """Chambers's rule for Scripture names of two syllables: the accent is on
    the first. A two-syllable entry stressed on the second contradicts it."""
    vs = [p for p in pron.split() if p[-1].isdigit()]
    if len(vs) != 2:
        return True
    return vs[0].endswith('1')
