"""Draws all 23 gallery shapes of the original Paint in a grid (window maximized at 1920×1080,
canvas at client (210, 201)): python shapes_study.py <title> [kinds...]. A click outside commits each shape."""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap

title = sys.argv[1]
kinds = [int(a) for a in sys.argv[2:]] or list(range(23))
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
OX, OY = 210, 201
GX = [462, 484, 506, 528, 551, 573, 595]  # gallery columns (screen)
GY = [84, 106, 128]
VK_RETURN = 0x0D


def pick(k):
    r, c = divmod(k, 7)
    if r >= 3:
        cap.wheel(GX[3], GY[1], -1)
        cap.drag('l', [(GX[c], GY[2])])
        cap.wheel(GX[3], GY[1], 1)
    else:
        cap.drag('l', [(GX[c], GY[r])])
    time.sleep(0.3)


def box(k):
    r, c = divmod(k, 6)
    x0, y0 = OX + 250 * c + 25, OY + 180 * r + 25
    return (cx + x0, cy + y0), (cx + x0 + 200, cy + y0 + 130)


for k in kinds:
    pick(k)
    a, b = box(k)
    cap.drag('l', [a, b], step=6)
    time.sleep(0.3)
    # A plain click outside the draft commits it (Enter needs keyboard focus on the canvas).
    cap.drag('l', [(cx + OX + 1490, cy + OY + 712)])
    time.sleep(0.3)
cap.move(cx + 5, cy + 990)
time.sleep(0.4)
