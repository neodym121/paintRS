"""Brush outline and fill styles of shapes in the original (maximized window, canvas at
client (210, 201)): python style_study.py <title>. Row y=470: outlines, row y=590: fills."""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap

h = cap.find(sys.argv[1])
cx, cy, _, _ = cap.client_rect(h)
OX, OY = 210, 201
P = lambda x, y: (cx + OX + x, cy + OY + y)
SHAPE = {'oval': (506, 84), 'rect': (528, 84)}
OUT_BTN, FILL_BTN = (646, 88), (646, 128)
OUT_ITEMS = [133, 181, 225, 273, 318, 364, 410]
FILL_ITEMS = [173, 220, 265, 313, 357, 404, 450]
COMMIT = P(1490, 712)


def choose(btn, items, i):
    cap.drag('l', [btn]); time.sleep(0.5)
    cap.drag('l', [(621, items[i])]); time.sleep(0.4)


for row, (y, outline, fills) in enumerate([(470, None, 0), (590, 0, None)]):
    for k, style in enumerate(range(2, 7)):
        for j, shape in enumerate(['rect', 'oval']):
            cap.drag('l', [SHAPE[shape]]); time.sleep(0.3)
            if row == 0:
                choose(FILL_BTN, FILL_ITEMS, 0)
                choose(OUT_BTN, OUT_ITEMS, style)
            else:
                choose(OUT_BTN, OUT_ITEMS, 0)
                choose(FILL_BTN, FILL_ITEMS, style)
            x = 40 + 280 * k + 130 * j
            cap.drag('l', [P(x, y), P(x + 110, y + 90)], step=6); time.sleep(0.3)
            cap.drag('l', [COMMIT]); time.sleep(0.3)
cap.move(cx + 5, cy + 990); time.sleep(0.4)
