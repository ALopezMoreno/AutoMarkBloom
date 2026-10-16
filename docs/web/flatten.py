import re, sys, os
# Flattens automark.tex into one pandoc-friendly file plus one standalone
# file per TikZ figure. Usage: flatten.py DOCS_DIR OUT_DIR
D = sys.argv[1]; OUT = sys.argv[2]
main = open(f'{D}/automark.tex').read()
body = main[main.index(r'\begin{document}')+len(r'\begin{document}'):main.index(r'\end{document}')]
def inc(m): return open(f'{D}/{m.group(1)}.tex').read()
body = re.sub(r'\\input\{([^}]*)\}', inc, body)
body = re.sub(r'(?m)^\s*%.*$', '', body)
# figures: tikz -> placeholders
figs = []
def fig(m):
    figs.append(m.group(0)); return f'\n\nFIGURE{len(figs)-1}PLACEHOLDER\n\n'
body = re.sub(r'\\begin\{tikzpicture\}.*?\\end\{tikzpicture\}', fig, body, flags=re.S)
for i,f in enumerate(figs):
    open(f'{OUT}/fig{i}.tex','w').write(r'''\documentclass[border=4pt]{standalone}
\usepackage[T1]{fontenc}\usepackage{lmodern}\usepackage{amsmath,amssymb,amsthm}\usepackage{xcolor}\usepackage{tikz}
\usetikzlibrary{arrows.meta,positioning,calc}
\input{macros}
\begin{document}'''+f+r'\end{document}')
# titles for refs
titles = {}
for m in re.finditer(r'\\(?:sub)?section\*?\{((?:[^{}]|\{[^{}]*\})*)\}\\label\{([^}]*)\}', body):
    titles[m.group(2)] = m.group(1)
def clean(t): return re.sub(r'\\[a-zA-Z]+\{?|[{}]', '', t)
def cref(m):
    parts = [p.strip() for p in m.group(1).split(',')]
    return ' and '.join(r'\hyperref[%s]{%s}' % (p, clean(titles.get(p,p))) for p in parts)
body = re.sub(r'\\cref\{([^}]*)\}', cref, body)
body = body.replace(r'\statusbuilt', 'STATUS:built').replace(r'\statusrework', 'STATUS:rework').replace(r'\statusplanned','STATUS:planned').replace(r'\statuspaused','STATUS:paused')
body = re.sub(r'\\src\{([^}]*)\}', lambda m: r'\texttt{' + m.group(1).replace('_', r'\_') + '}', body)
body = re.sub(r'\\stagebox\{((?:[^{}]|\{[^{}]*\})*)\}\{((?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*)\}', r'\n\nSTAGEOPEN \1 SOURCES \2 STAGECLOSE\n\n', body)
body = re.sub(r'\\addcontentsline\{[^}]*\}\{[^}]*\}\{[^}]*\}', '', body)
for k in [r'\maketitle', r'\tableofcontents', r'\appendix']: body = body.replace(k, '')
body = body.replace(r'\begin{abstract}', '').replace(r'\end{abstract}', '')
mac = open(f'{D}/macros.tex').read()
mathmac = '\n'.join(l for l in mac.splitlines() if re.match(r'\\(newcommand|DeclareMathOperator)\{\\(R|vect|x|rt|tmax|Unif|Norm|NA|NP|NU|NV|clamp|round)\}', l))
open(f'{OUT}/flat.tex','w').write(r'\documentclass{article}' + '\n' + mathmac + '\n' + r'\begin{document}' + body + r'\end{document}')
print(len(figs), 'figures;', len(titles), 'labels')
