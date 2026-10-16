"""python brush_study.py <title> <row> <size positions...>  -- draws test cells for the current brush.
Canvas origin and size knob are given in client coordinates via env-like constants below."""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap
title = sys.argv[1]
row = int(sys.argv[2])
positions = [int(a) for a in sys.argv[3:]]
ORIGIN = (int(os.environ.get('OX', 210)), int(os.environ.get('OY', 201)))
KNOB = (int(os.environ.get('KX', 31)), int(os.environ.get('KY', 534)))
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
VK_UP, VK_HOME = 0x26, 0x24
def set_pos(p):
    cap.drag('l', [(cx + KNOB[0], cy + KNOB[1])]); time.sleep(0.25)
    cap.key(VK_HOME); time.sleep(0.2)
    if p > 1:
        cap.key(VK_UP, p - 1)
    time.sleep(0.3)
def C(x, y):
    return (cx + ORIGIN[0] + x, cy + ORIGIN[1] + y)
for i, p in enumerate(positions):
    set_pos(p)
    x0, y0 = 300 * i, 180 * row
    cap.drag('l', [C(x0 + 25, y0 + 30)])
    time.sleep(0.15)
    cap.drag('l', [C(x0 + 60, y0 + 30), C(x0 + 280, y0 + 30)])
    time.sleep(0.15)
    cap.drag('l', [C(x0 + 25, y0 + 70), C(x0 + 95, y0 + 150)])
    time.sleep(0.15)
    cap.drag('l', [C(x0 + 115, y0 + 150), C(x0 + 185, y0 + 70)])
    time.sleep(0.15)
    cap.drag('l', [C(x0 + 215, y0 + 70), C(x0 + 215, y0 + 150)])
    time.sleep(0.15)
    cap.drag('l', [C(x0 + 245, y0 + 150), C(x0 + 285, y0 + 70)])
    time.sleep(0.15)
cap.move(cx + 5, cy + 990); time.sleep(0.5)
