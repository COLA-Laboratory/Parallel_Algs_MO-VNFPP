# http://blog.analogmachine.org/2013/08/12/how-to-increase-miktex-2-9-memory/

pdflatex pcp_12.tex
pdflatex pcp_16.tex
pdflatex pcp_20.tex
pdflatex pcp_24.tex
pdflatex pcp_28.tex
pdflatex pcp_key.tex

pdfcrop pcp_12.pdf
pdfcrop pcp_16.pdf
pdfcrop pcp_20.pdf
pdfcrop pcp_24.pdf
pdfcrop pcp_28.pdf
pdfcrop --margins '-250 0 0 -204' pcp_key.pdf 

rm *.log
rm *.aux
rm *.fdb_latexmk
rm *.fls
rm *.synctex.gz