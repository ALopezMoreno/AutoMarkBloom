# Assembles the phone-friendly HTML page from pandoc output, the figure SVGs
# and page.tpl.html. Usage (from the build dir): build.py DOCS_DIR
import re, base64, sys, os
D = sys.argv[1]
body = open('body.html').read()
body = body.replace('id="sec:', 'id="sec-').replace('href="#sec:', 'href="#sec-').replace('id="fig:', 'id="fig-')
label = {'built':'Built','rework':'Built, being reworked','planned':'Designed, not built','paused':'On hold','progress':'In progress'}
body = re.sub(r'STATUS:(built|rework|planned|paused|progress)', lambda m: f'<span class="chip chip-{m.group(1)}">{label[m.group(1)]}</span>', body)
def box(m):
    st, src = m.group(1).strip(), m.group(2).strip()
    return f'<div class="stage"><div><span class="k">Status</span> {st}</div><div><span class="k">Source</span> {src}</div></div>'
body = re.sub(r'<p>STAGEOPEN\s+(.*?)\s+SOURCES\s+(.*?)\s+STAGECLOSE</p>', box, body, flags=re.S)
assert 'STAGEOPEN' not in body and 'STAGECLOSE' not in body
import glob
for i in range(len(glob.glob('fig*.svg'))):
    svg = base64.b64encode(open(f'fig{i}.svg','rb').read()).decode()
    body = body.replace(f'<p>FIGURE{i}PLACEHOLDER</p>', f'<div class="figpanel"><img src="data:image/svg+xml;base64,{svg}" alt=""></div>')
assert 'PLACEHOLDER' not in body
body = re.sub(r'<table>', '<div class="tablewrap"><table>', body); body = body.replace('</table>', '</table></div>')
# TOC from h1
toc = []
for m in re.finditer(r'<section id="([^"]+)" class="level1( unnumbered)?">\s*<h1[^>]*>(.*?)</h1>', body, flags=re.S):
    toc.append((m.group(1), re.sub('<[^>]+>','',m.group(3)).strip(), bool(m.group(2))))
n=0; items=[]
for id_, t, un in toc:
    if un: items.append(f'<li><a href="#{id_}">{t}</a></li>')
    else:
        n+=1; items.append(f'<li><a href="#{id_}"><span class="num">{n}</span>{t}</a></li>')
tpl = open(os.path.join(D, 'web', 'page.tpl.html')).read()
mac=open(os.path.join(D, 'macros.tex')).read()
tpl=tpl.replace('{{COMMIT}}', re.search(r'docsyncedcommit\}\{([^}]*)\}',mac).group(1)).replace('{{DATE}}', re.search(r'docsynced\}\{([^}]*)\}',mac).group(1))
out = tpl.replace('{{TOC}}', '\n'.join(items)).replace('{{BODY}}', body)
open('automark.html','w').write(out)
print(len(out), 'bytes', n, 'sections')
