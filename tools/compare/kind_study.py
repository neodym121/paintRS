"""Draws one gallery shape over several boxes (canvas coordinates; maximized window, canvas at
client (210, 201)): python kind_study.py <title> <kind|-1> x0,y0,x1,y1 ...
kind -1 keeps the shape that is already selected."""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cap
title, kind = sys.argv[1], int(sys.argv[2])
boxes = [tuple(int(v) for v in a.split(',')) for a in sys.argv[3:]]
h = cap.find(title)
cx, cy, _, _ = cap.client_rect(h)
OX, OY = 210, 201
GX = [462, 484, 506, 528, 551, 573, 595]
GY = [84, 106, 128]
r, c = divmod(kind, 7)
if kind < 0:
    pass
elif r >= 3:
    cap.wheel(GX[3], GY[1], -1)
    cap.drag('l', [(GX[c], GY[2])])
    cap.wheel(GX[3], GY[1], 1)
else:
    cap.drag('l', [(GX[c], GY[r])])
time.sleep(0.3)
for x0, y0, x1, y1 in boxes:
    cap.drag('l', [(cx + OX + x0, cy + OY + y0), (cx + OX + x1, cy + OY + y1)], step=6)
    time.sleep(0.3)
    cap.drag('l', [(cx + OX + 1490, cy + OY + 712)])
    time.sleep(0.3)
cap.move(cx + 5, cy + 990)
time.sleep(0.4)
