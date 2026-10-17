"""Window capture (PrintWindow) and SendInput mouse helpers for comparing Paint and PaintRS.

Usage:
  python cap.py shot <title-substring> <out.png>          # client area, physical pixels
  python cap.py info <title-substring>                     # window + client rect on screen
  python cap.py drag <btn> x0,y0 x1,y1 [x2,y2 ...]        # screen coords, btn = l|r
  python cap.py click <btn> x,y
  python cap.py max <title-substring>                      # maximize and bring to front

Helpers for the clean-room comparison of PaintRS with the original (observed behaviour only):
PrintWindow captures of a window's client area (1 screen pixel = 1 canvas pixel at 100%) and
SendInput mouse/keyboard input. Paint must be the foreground window for key() to reach it.
"""
import ctypes
import ctypes.wintypes as wt
import sys
import time

from PIL import Image

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))


def find(sub):
    found = []

    def cb(hwnd, _):
        if user32.IsWindowVisible(hwnd):
            n = user32.GetWindowTextLengthW(hwnd)
            buf = ctypes.create_unicode_buffer(n + 1)
            user32.GetWindowTextW(hwnd, buf, n + 1)
            if sub.lower() in buf.value.lower():
                found.append((hwnd, buf.value))
        return True

    proto = ctypes.WINFUNCTYPE(ctypes.c_bool, wt.HWND, wt.LPARAM)
    user32.EnumWindows(proto(cb), 0)
    if not found:
        sys.exit(f"no window with '{sub}'")
    return found[0][0]


def client_rect(hwnd):
    r = wt.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    p = wt.POINT(0, 0)
    user32.ClientToScreen(hwnd, ctypes.byref(p))
    return p.x, p.y, r.right, r.bottom


def shot(hwnd, out):
    wr = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(wr))
    w, h = wr.right - wr.left, wr.bottom - wr.top
    hdc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mdc, bmp)
    user32.PrintWindow(hwnd, mdc, 2)

    class BMI(ctypes.Structure):
        _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG),
                    ("biPlanes", wt.WORD), ("biBitCount", wt.WORD), ("biCompression", wt.DWORD),
                    ("biSizeImage", wt.DWORD), ("biXPelsPerMeter", wt.LONG),
                    ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD), ("biClrImportant", wt.DWORD)]

    bmi = BMI(ctypes.sizeof(BMI), w, -h, 1, 32, 0, 0, 0, 0, 0, 0)
    buf = ctypes.create_string_buffer(w * h * 4)
    gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bmi), 0)
    img = Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).convert("RGB")
    cx, cy, cw, ch = client_rect(hwnd)
    ox, oy = cx - wr.left, cy - wr.top
    img.crop((ox, oy, ox + cw, oy + ch)).save(out)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(hwnd, hdc)


MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE = 0x0001, 0x8000
DOWN = {"l": 0x0002, "r": 0x0008}
UP = {"l": 0x0004, "r": 0x0010}


class MI(ctypes.Structure):
    _fields_ = [("dx", wt.LONG), ("dy", wt.LONG), ("mouseData", wt.DWORD), ("dwFlags", wt.DWORD),
                ("time", wt.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", wt.DWORD), ("mi", MI)]


def send(flags, x=0, y=0):
    sw, sh = user32.GetSystemMetrics(0), user32.GetSystemMetrics(1)
    ax = int(x * 65535 / (sw - 1)) if flags & MOUSEEVENTF_ABSOLUTE else 0
    ay = int(y * 65535 / (sh - 1)) if flags & MOUSEEVENTF_ABSOLUTE else 0
    inp = INPUT(0, MI(ax, ay, 0, flags, 0, 0))
    user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))


def move(x, y):
    send(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, x, y)


def drag(btn, pts, step=4, delay=0.004):
    x, y = pts[0]
    move(x, y)
    time.sleep(0.05)
    send(DOWN[btn])
    time.sleep(0.05)
    for nx, ny in pts[1:]:
        n = max(1, int(max(abs(nx - x), abs(ny - y)) / step))
        for i in range(1, n + 1):
            move(x + (nx - x) * i / n, y + (ny - y) * i / n)
            time.sleep(delay)
        x, y = nx, ny
    time.sleep(0.05)
    send(UP[btn])
    time.sleep(0.05)


def main():
    cmd = sys.argv[1]
    if cmd == "shot":
        shot(find(sys.argv[2]), sys.argv[3])
    elif cmd == "info":
        h = find(sys.argv[2])
        wr = wt.RECT()
        user32.GetWindowRect(h, ctypes.byref(wr))
        print("window", wr.left, wr.top, wr.right, wr.bottom, "client", client_rect(h))
    elif cmd == "drag":
        pts = [tuple(float(v) for v in a.split(",")) for a in sys.argv[3:]]
        drag(sys.argv[2], pts)
    elif cmd == "max":
        h = find(sys.argv[2])
        user32.ShowWindow(h, 3)
        user32.SetForegroundWindow(h)
    elif cmd == "click":
        x, y = (float(v) for v in sys.argv[3].split(","))
        drag(sys.argv[2], [(x, y)])


if __name__ == "__main__":
    main()


class KI(ctypes.Structure):
    _fields_ = [("wVk", wt.WORD), ("wScan", wt.WORD), ("dwFlags", wt.DWORD), ("time", wt.DWORD),
                ("dwExtraInfo", ctypes.c_size_t)]


class KINPUT(ctypes.Structure):
    _fields_ = [("type", wt.DWORD), ("ki", KI), ("pad", ctypes.c_byte * 8)]


def wheel(x, y, notches):
    """Mouse wheel at screen (x, y): positive = away from the user (scroll up)."""
    move(x, y)
    time.sleep(0.05)
    inp = INPUT(0, MI(0, 0, ctypes.c_uint32(int(120 * notches) & 0xFFFFFFFF).value, 0x0800, 0, 0))
    user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(INPUT))
    time.sleep(0.1)


def key(vk, n=1, extended=True):
    for _ in range(n):
        for up in (0, 2):
            flags = up | (1 if extended else 0)
            inp = KINPUT(1, KI(vk, 0, flags, 0, 0))
            user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(KINPUT))
            time.sleep(0.01)
