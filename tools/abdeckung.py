"""Abdeckung der Original-Oberfläche messen.

Sammelt aus Formular-Auszügen des Originals (*.txt im Ordner aus EP_FORMULARE, nicht Teil des Repos) jede sichtbare
Beschriftung (Caption, Hint, Listeneinträge, Reiter) und prüft, ob der Nachbau (src/*.rs) sie anbietet.
Ausgabe: je Formular gefundene/gesamte Beschriftungen und Prozent, fehlende Beschriftungen, Gesamtwert.

Aufruf:  python tools/abdeckung.py [--fehlend]
"""
import os, re, sys, glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
src = ''
for f in glob.glob(os.path.join(ROOT, 'src', '*.rs')):
    src += open(f, encoding='utf-8').read()
src_l = src.lower()

# Platzhalter/Komponentennamen, die keine sichtbare Beschriftung sind
SKIP = re.compile(r'^(-|ToolButton\d*|Label\d*|Button\d*|Panel\d*|GroupBox\d*|TabSheet\d*|StaticText\d*|Frei|\d+|[A-Z][a-z]+\d+|.)$')

def fix(s):
    # Formular-Dumps sind latin1 dekodiert, Umlaute als Ersatzzeichen – zurück nach cp1252
    try:
        return s.encode('latin1').decode('cp1252')
    except Exception:
        return s.replace('�', '?')

def labels(path):
    out = []
    for line in open(path, encoding='utf-8', errors='replace'):
        if 'Visible=False' in line:
            continue
        for key in ('Caption', 'Hint'):
            for m in re.finditer(key + r'=(.*?)(?= [A-Z][A-Za-z.]+=|$)', line):
                out.append(m.group(1).strip())
        for m in re.finditer(r'(?:Items|Tabs)\.Strings=\[(.*?)\]', line):
            out.extend(x.strip() for x in m.group(1).split(';'))
    res = []
    for l in out:
        l = fix(l).strip()
        if not l or SKIP.match(l) or l in res:
            continue
        res.append(l)
    return res

def found(label):
    l = label.lower().replace('&', '')
    if '?' in l or '�' in l:
        pat = re.escape(l).replace(r'\?', '.').replace('�', '.')
        return re.search(pat, src_l) is not None
    return l in src_l

total = hit = 0
rows = []
missing_all = []
FORMULARE = os.environ.get('EP_FORMULARE') or os.path.join(ROOT, 'docs', 'original')
for f in sorted(glob.glob(os.path.join(FORMULARE, '*.txt'))):
    name = os.path.basename(f)[:-4]
    ls = labels(f)
    if not ls:
        continue
    ok = [l for l in ls if found(l)]
    miss = [l for l in ls if not found(l)]
    total += len(ls)
    hit += len(ok)
    rows.append((name, len(ok), len(ls), miss))
    missing_all += [(name, m) for m in miss]

w = max(len(r[0]) for r in rows)
for name, ok, n, miss in rows:
    print(f'{name:<{w}}  {ok:>3}/{n:<3} {100 * ok / n:5.1f} %')
print(f'{"GESAMT":<{w}}  {hit:>3}/{total:<3} {100 * hit / total:5.1f} %')
if '--fehlend' in sys.argv:
    print()
    for name, m in missing_all:
        print(f'{name}: {m}')
