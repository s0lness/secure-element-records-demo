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


def sleeve(size: int) -> Image.Image:
    """Empty record sleeve: outline square, center hole, nothing inside."""
    n = size * SS
    img = Image.new("L", (n, n), 255)
    d = ImageDraw.Draw(img)
    m = n * 0.06
    d.rounded_rectangle([m, m, n - m, n - m], radius=n * 0.06, outline=60,
                        width=max(SS // 2, 1) * 2)
    c = n / 2
    r = n * 0.13
    d.ellipse([c - r, c - r, c + r, c + r], outline=140, width=max(SS // 2, 1))
    return img.resize((size, size), Image.LANCZOS)


def press(size: int) -> Image.Image:
    """The press: a vinyl under the stamper plate."""
    n = size * SS
    img = Image.new("L", (n, n), 255)
    d = ImageDraw.Draw(img)
    c = n / 2
    plate_h = n * 0.14
    d.rounded_rectangle([n * 0.12, 0, n * 0.88, plate_h], radius=n * 0.04, fill=60)
    d.rectangle([c - n * 0.05, plate_h, c + n * 0.05, n * 0.30], fill=60)
    disc_c = n * 0.62
    disc_r = n * 0.36

    def circle(r, **kw):
        d.ellipse([c - r, disc_c - r, c + r, disc_c + r], **kw)

    circle(disc_r, fill=25)
    for frac in (0.62, 0.84):
        circle(disc_r * frac, outline=110, width=max(SS // 2, 1))
    circle(disc_r * 0.30, fill=200)
    circle(disc_r * 0.07, fill=255)
    return img.resize((size, size), Image.LANCZOS)


def save_gif(img: Image.Image, path: str):
    img.convert("P").save(path, format="GIF")
    print(f"wrote {path} ({img.size[0]}x{img.size[1]})")


if __name__ == "__main__":
    save_gif(vinyl(64), os.path.join(ROOT, "glyphs", "vinyl_64x64.gif"))
    save_gif(sleeve(64), os.path.join(ROOT, "glyphs", "sleeve_64x64.gif"))
    save_gif(press(64), os.path.join(ROOT, "glyphs", "press_64x64.gif"))
    save_gif(vinyl(40), os.path.join(ROOT, "icons", "vinyl_40x40.gif"))
    save_gif(vinyl(32), os.path.join(ROOT, "icons", "vinyl_32x32.gif"))
