"""Same pencil scenario for any window: python pencil_test2.py <title> <knob_x> <knob_y> <origin_x> <origin_y> <out>"""
import sys, time
sys.path.insert(0, '.')
import cap
title, kx, ky, ox, oy, out = sys.argv[1], *map(int, sys.argv[2:6]), sys.argv[6]
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
VK_UP, VK_HOME = 0x26, 0x24
def set_size(s):
    cap.drag('l', [(cx + kx, cy + ky)]); time.sleep(0.2)
    cap.key(VK_HOME); time.sleep(0.2)
    if s > 1:
        cap.key(VK_UP, s - 1)
    time.sleep(0.3)
# canvas-relative positions (image pixels at 100%)
for i, s in enumerate(range(1, 11)):
    set_size(s)
    x = ox + 30 + 36 * i
    cap.drag('l', [(cx + x, cy + oy + 20)])
    cap.drag('l', [(cx + x - 8, cy + oy + 60), (cx + x + 12, cy + oy + 60)])
    cap.drag('l', [(cx + x - 8, cy + oy + 100), (cx + x + 12, cy + oy + 120)])
cap.move(cx + ox - 100, cy + oy + 150); time.sleep(0.4)
cap.shot(h, out)
