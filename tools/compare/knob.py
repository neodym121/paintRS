"""Finds a side-slider knob in a capture: python knob.py <png> <center_x> <y0> <y1> (client px)."""
import sys
import numpy as np
from PIL import Image
im = np.asarray(Image.open(sys.argv[1]).convert("L")).astype(int)
cx = int(sys.argv[2]); y0, y1 = int(sys.argv[3]), int(sys.argv[4])
band = im[y0:y1, cx - 7:cx + 8]
width = (band > 140).sum(axis=1)
rows = np.where((width >= 7) & (width <= 13))[0]
print(f"center {y0 + (rows.min() + rows.max()) / 2:.1f} rows {y0+rows.min()}-{y0+rows.max()}")
