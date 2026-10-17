"""Same interactive shape scenario for Paint and PaintRS (window maximized, canvas at client
(210, 201), gallery scrolled to the top, 1 px outline): curve with two bends, polygon closed at its
start, rectangle moved and resized as a draft, triangle dragged right to left.
python interact_study.py <title>"""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap
title = sys.argv[1]
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
OX, OY = 210, 201
P = lambda x, y: (cx + OX + x, cy + OY + y)
GX = [462, 484, 506, 528, 551, 573, 595]
GY = [84, 106, 128]
def pick(k):
    r, c = divmod(k, 7)
    cap.drag('l', [(GX[c], GY[r])]); time.sleep(0.4)
def commit():
    cap.drag('l', [P(1490, 712)]); time.sleep(0.4)
# A: curve with two bends
pick(1)
cap.drag('l', [P(100, 100), P(400, 100)], step=6); time.sleep(0.3)
cap.drag('l', [P(200, 100), P(200, 180)], step=6); time.sleep(0.3)
cap.drag('l', [P(330, 100), P(330, 40)], step=6); time.sleep(0.3)
commit()
# B: polygon closed by clicking near its start
pick(5)
cap.drag('l', [P(500, 100), P(700, 100)], step=6); time.sleep(0.3)
cap.drag('l', [P(700, 250)]); time.sleep(0.3)
cap.drag('l', [P(550, 300)]); time.sleep(0.3)
cap.drag('l', [P(502, 102)]); time.sleep(0.4)
commit()
# C: rectangle moved and resized while still a draft
pick(3)
cap.drag('l', [P(100, 300), P(250, 400)], step=6); time.sleep(0.3)
cap.drag('l', [P(175, 350), P(215, 370)], step=6); time.sleep(0.3)
cap.drag('l', [P(290, 420), P(340, 450)], step=6); time.sleep(0.3)
commit()
# F: shape drawn right to left / bottom to top
pick(6)
cap.drag('l', [P(700, 450), P(500, 330)], step=6); time.sleep(0.3)
commit()
cap.move(cx + 5, cy + 990); time.sleep(0.4)
