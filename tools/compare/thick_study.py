"""Rectangles, ovals and lines with 1/3/5/8 px outlines in the original (window maximized,
canvas at client (210, 201)). python thick_study.py <title> <y0>"""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap

title, y0 = sys.argv[1], int(sys.argv[2])
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
OX, OY = 210, 201
P = lambda x, y: (cx + OX + x, cy + OY + y)
# Screen positions in the maximized original (CU coords × 1.3187).
SHAPE = {'line': (462, 84), 'oval': (506, 84), 'rect': (528, 84)}
SIZE_BTN = (695, 112)
SIZE_ITEM = {1: (696, 162), 3: (696, 207), 5: (696, 254), 8: (696, 299)}
COMMIT = P(1490, 712)
for i, size in enumerate([1, 3, 5, 8]):
    for j, shape in enumerate(['rect', 'oval', 'line']):
        cap.drag('l', [SHAPE[shape]]); time.sleep(0.3)
        cap.drag('l', [SIZE_BTN]); time.sleep(0.5)
        cap.drag('l', [SIZE_ITEM[size]]); time.sleep(0.4)
        x = 40 + 360 * i + 110 * j
        cap.drag('l', [P(x, y0), P(x + 80, y0 + 60)], step=6); time.sleep(0.3)
        cap.drag('l', [COMMIT]); time.sleep(0.3)
cap.move(cx + 5, cy + 990); time.sleep(0.4)
