# Build the model documentation PDF: `make` (needs latexmk and a TeX Live
# with the latex-extra collection). `make clean` removes build files.

automark.pdf: automark.tex macros.tex $(wildcard sections/*.tex) $(wildcard figures/*.tex)
	latexmk -pdf -interaction=nonstopmode -halt-on-error automark.tex

check:
	./check-sync.sh

clean:
	latexmk -C automark.tex

.PHONY: check clean html

# Phone-friendly HTML page (needs pandoc): build/automark.html
html: automark.tex macros.tex $(wildcard sections/*.tex) $(wildcard figures/*.tex) web/page.tpl.html
	mkdir -p build && cp macros.tex build/
	cd build && python3 ../web/flatten.py .. . && \
	  for f in fig*.tex; do pdflatex -interaction=nonstopmode $$f >/dev/null && pdftocairo -svg $${f%.tex}.pdf $${f%.tex}.svg; done && \
	  pandoc flat.tex -f latex -t html5 --mathjax --section-divs -o body.html && \
	  python3 ../web/build.py ..
