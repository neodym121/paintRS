"""python cmp_row.py <paint.png> <paintrs.png> <row> <out.png>: stacks one study row of both and prints stats."""
import sys
import numpy as np
from PIL import Image

pa, pb, row, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
y0 = 201 + 180 * row
a = np.asarray(Image.open(pa).convert('L')).astype(int)[y0:y0 + 180, 210:1710]
b = np.asarray(Image.open(pb).convert('L')).astype(int)[y0:y0 + 180, 210:1710]
Image.fromarray(np.vstack([a, np.full((4, 1500), 128), b]).astype(np.uint8)).save(out)
d = np.abs(a - b)
print('mean abs diff', d.mean().round(3), '| >32:', (d > 32).sum(), '| ink paint/ours', (a < 250).sum(), (b < 250).sum(),
      '| mean tone paint/ours', round(255 - a[a < 250].mean(), 1) if (a < 250).any() else 0,
      round(255 - b[b < 250].mean(), 1) if (b < 250).any() else 0)
for c in range(5):
    ca, cb = a[:, 300 * c:300 * c + 300], b[:, 300 * c:300 * c + 300]
    print(f'  cell {c}: ink {(ca < 250).sum()} vs {(cb < 250).sum()}, tone {255 - ca[ca < 250].mean() if (ca < 250).any() else 0:.0f} vs {255 - cb[cb < 250].mean() if (cb < 250).any() else 0:.0f}')
