`check_bud.py a.json [b.json ...] out.png`: quick matplotlib check of flower JSON (3D view, cut-open view, slice). Needs numpy and matplotlib.

`slice_bud.py out.png a.json [b.json ...]`: slice through each bud at y = 0, drawing every petal as a line as thick as the petal. With recorded frames (`automark --frames`) it shows four development stages per file; with single-frame files it puts the buds side by side.
