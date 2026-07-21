"""Generate the vinyl record icons (grayscale GIFs) for the device app.
Run in WSL: python3 scripts/make-icons.py"""

import os

from PIL import Image, ImageDraw

ROOT = os.path.join(os.path.dirname(__file__), "..", "device-app")
SS = 8  # supersampling factor


def vinyl(size: int) -> Image.Image:
    n = size * SS
    img = Image.new("L", (n, n), 255)
    d = ImageDraw.Draw(img)
    c = n / 2
    disc_r = n * 0.48

    def circle(r, **kw):
        d.ellipse([c - r, c - r, c + r, c + r], **kw)

    circle(disc_r, fill=25)
    for frac in (0.62, 0.74, 0.86, 0.97):
        circle(disc_r * frac, outline=110, width=max(SS // 2, 1))
    circle(disc_r * 0.32, fill=200)
    circle(disc_r * 0.30, outline=140, width=max(SS // 2, 1))
    circle(disc_r * 0.07, fill=255)
    return img.resize((size, size), Image.LANCZOS)


def save_gif(img: Image.Image, path: str):
    img.convert("P").save(path, format="GIF")
    print(f"wrote {path} ({img.size[0]}x{img.size[1]})")


if __name__ == "__main__":
    save_gif(vinyl(64), os.path.join(ROOT, "glyphs", "vinyl_64x64.gif"))
    save_gif(vinyl(40), os.path.join(ROOT, "icons", "vinyl_40x40.gif"))
    save_gif(vinyl(32), os.path.join(ROOT, "icons", "vinyl_32x32.gif"))
