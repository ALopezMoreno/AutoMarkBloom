# Build the model documentation PDF: `make` (needs latexmk and a TeX Live
# with the latex-extra collection). `make clean` removes build files.

automark.pdf: automark.tex macros.tex $(wildcard sections/*.tex)
	latexmk -pdf -interaction=nonstopmode -halt-on-error automark.tex

check:
	./check-sync.sh

clean:
	latexmk -C automark.tex

.PHONY: check clean
