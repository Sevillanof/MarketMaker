"""Genera las imágenes de ejemplo de `public/img`.

Ilustraciones planas dibujadas con Pillow, guardadas en AVIF (la principal) y
WebP (respaldo para navegadores sin AVIF). Los logos son SVG.

    python3 scripts/gen_images.py

Necesita Pillow 11.3 o más nuevo (trae AVIF).
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parent.parent / "public" / "img"
SS = 2  # supersampling para bordes suaves

PALETTES = [
    ("#6366f1", "#a855f7"),
    ("#0ea5e9", "#22d3ee"),
    ("#f97316", "#facc15"),
    ("#10b981", "#84cc16"),
    ("#ef4444", "#f472b6"),
    ("#334155", "#64748b"),
    ("#8b5cf6", "#ec4899"),
    ("#0f766e", "#14b8a6"),
]


def hex2rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def gradient(w, h, a, b):
    a, b = hex2rgb(a), hex2rgb(b)
    base = Image.new("RGB", (w, h))
    px = base.load()
    for y in range(h):
        for x in range(0, w):
            t = (x / w * 0.6 + y / h * 0.4)
            px[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))
    return base


def canvas(w, h, pal):
    w, h = w * SS, h * SS
    img = gradient(w // 4, h // 4, *pal).resize((w, h), Image.BICUBIC)
    # Círculos suaves de fondo.
    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    g = ImageDraw.Draw(glow)
    g.ellipse((-w * 0.1, h * 0.45, w * 0.45, h * 1.4), fill=(255, 255, 255, 40))
    g.ellipse((w * 0.6, -h * 0.4, w * 1.2, h * 0.5), fill=(255, 255, 255, 30))
    img = Image.alpha_composite(img.convert("RGBA"), glow)
    return img, ImageDraw.Draw(img)


W = (255, 255, 255, 255)
SHADOW = (0, 0, 0, 60)


def shadow(img, box):
    x0, y0, x1, y1 = box
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ImageDraw.Draw(layer).ellipse((x0, y1 - (y1 - y0) * 0.08, x1, y1 + (y1 - y0) * 0.08), fill=SHADOW)
    layer = layer.filter(ImageFilter.GaussianBlur(img.size[0] // 60))
    img.alpha_composite(layer)


# Íconos de productos, dibujados en una caja (cx, cy, s) donde s es el lado.
def headphones(d, cx, cy, s, c):
    lw = int(s * 0.07)
    d.arc((cx - s * 0.35, cy - s * 0.4, cx + s * 0.35, cy + s * 0.3), 180, 360, fill=c, width=lw)
    d.line((cx - s * 0.35, cy - s * 0.05, cx - s * 0.35, cy + s * 0.1), fill=c, width=lw)
    d.line((cx + s * 0.35, cy - s * 0.05, cx + s * 0.35, cy + s * 0.1), fill=c, width=lw)
    for sx in (-1, 1):
        x = cx + sx * s * 0.33
        d.rounded_rectangle((x - s * 0.1, cy, x + s * 0.1, cy + s * 0.32), radius=s * 0.06, fill=c)


def backpack(d, cx, cy, s, c, acc):
    d.rounded_rectangle((cx - s * 0.12, cy - s * 0.45, cx + s * 0.12, cy - s * 0.25), radius=s * 0.08, outline=c, width=int(s * 0.04))
    d.rounded_rectangle((cx - s * 0.28, cy - s * 0.32, cx + s * 0.28, cy + s * 0.42), radius=s * 0.14, fill=c)
    d.rounded_rectangle((cx - s * 0.18, cy + s * 0.05, cx + s * 0.18, cy + s * 0.32), radius=s * 0.06, fill=acc)
    d.line((cx - s * 0.12, cy + s * 0.12, cx + s * 0.12, cy + s * 0.12), fill=c, width=int(s * 0.025))


def lamp(d, cx, cy, s, c, acc):
    d.polygon([(cx - s * 0.08, cy - s * 0.42), (cx + s * 0.16, cy - s * 0.42), (cx + s * 0.32, cy - s * 0.12), (cx - s * 0.24, cy - s * 0.12)], fill=c)
    d.ellipse((cx - s * 0.03, cy - s * 0.16, cx + s * 0.11, cy - s * 0.04), fill=acc)
    d.line((cx + s * 0.04, cy - s * 0.12, cx - s * 0.1, cy + s * 0.15, cx + s * 0.04, cy + s * 0.38), fill=c, width=int(s * 0.05), joint="curve")
    d.rounded_rectangle((cx - s * 0.22, cy + s * 0.36, cx + s * 0.3, cy + s * 0.45), radius=s * 0.04, fill=c)


def mug(d, cx, cy, s, c, acc):
    d.arc((cx + s * 0.1, cy - s * 0.15, cx + s * 0.42, cy + s * 0.2), 270, 90, fill=c, width=int(s * 0.07))
    d.rounded_rectangle((cx - s * 0.3, cy - s * 0.3, cx + s * 0.22, cy + s * 0.38), radius=s * 0.08, fill=c)
    d.rectangle((cx - s * 0.3, cy - s * 0.1, cx + s * 0.22, cy - s * 0.02), fill=acc)


def shoe(d, cx, cy, s, c, acc):
    d.polygon([(cx - s * 0.42, cy - s * 0.15), (cx - s * 0.18, cy - s * 0.18), (cx + s * 0.02, cy + s * 0.02), (cx + s * 0.38, cy + s * 0.08), (cx + s * 0.45, cy + s * 0.22), (cx - s * 0.45, cy + s * 0.22)], fill=c)
    d.rounded_rectangle((cx - s * 0.47, cy + s * 0.18, cx + s * 0.47, cy + s * 0.3), radius=s * 0.05, fill=acc)
    for i in range(3):
        x = cx - s * 0.15 + i * s * 0.08
        d.line((x, cy - s * 0.1 + i * s * 0.04, x + s * 0.06, cy - s * 0.02 + i * s * 0.04), fill=acc, width=int(s * 0.025))


def watch(d, cx, cy, s, c, acc):
    d.rounded_rectangle((cx - s * 0.12, cy - s * 0.5, cx + s * 0.12, cy + s * 0.5), radius=s * 0.05, fill=c)
    d.ellipse((cx - s * 0.28, cy - s * 0.28, cx + s * 0.28, cy + s * 0.28), fill=c)
    d.ellipse((cx - s * 0.22, cy - s * 0.22, cx + s * 0.22, cy + s * 0.22), fill=acc)
    lw = int(s * 0.03)
    d.line((cx, cy, cx, cy - s * 0.15), fill=c, width=lw)
    d.line((cx, cy, cx + s * 0.1, cy + s * 0.04), fill=c, width=lw)


def notebook(d, cx, cy, s, c, acc):
    d.rounded_rectangle((cx - s * 0.28, cy - s * 0.4, cx + s * 0.28, cy + s * 0.4), radius=s * 0.04, fill=c)
    d.rectangle((cx - s * 0.28, cy - s * 0.4, cx - s * 0.2, cy + s * 0.4), fill=acc)
    d.rounded_rectangle((cx - s * 0.08, cy - s * 0.2, cx + s * 0.18, cy - s * 0.08), radius=s * 0.02, fill=acc)
    d.line((cx + s * 0.18, cy - s * 0.4, cx + s * 0.18, cy + s * 0.48), fill=acc, width=int(s * 0.03))


def speaker(d, cx, cy, s, c, acc):
    d.rounded_rectangle((cx - s * 0.4, cy - s * 0.22, cx + s * 0.4, cy + s * 0.25), radius=s * 0.2, fill=c)
    for sx in (-1, 1):
        x = cx + sx * s * 0.2
        d.ellipse((x - s * 0.13, cy - s * 0.13, x + s * 0.13, cy + s * 0.13), fill=acc)
        d.ellipse((x - s * 0.05, cy - s * 0.05, x + s * 0.05, cy + s * 0.05), fill=c)


ICONS = [headphones, backpack, lamp, mug, shoe, watch, notebook, speaker]


def draw_icon(img, fn, cx, cy, s, pal):
    acc = hex2rgb(pal[0]) + (255,)
    shadow(img, (cx - s * 0.4, cy - s * 0.4, cx + s * 0.4, cy + s * 0.45))
    d = ImageDraw.Draw(img)
    if fn is headphones:
        fn(d, cx, cy, s, W)
    else:
        fn(d, cx, cy, s, W, acc)


def save(img, name, w, h):
    img = img.resize((w, h), Image.LANCZOS).convert("RGB")
    img.save(OUT / f"{name}.avif", quality=55, speed=4)
    img.save(OUT / f"{name}.webp", quality=78, method=6)


def products():
    for i, fn in enumerate(ICONS):
        pal = PALETTES[i]
        w, h = 800, 600
        img, _ = canvas(w, h, pal)
        draw_icon(img, fn, w * SS / 2, h * SS / 2, h * SS * 0.6, pal)
        save(img, f"product-{i + 1}", w, h)


def wide(name, pal, icons):
    w, h = 1600, 600
    img, _ = canvas(w, h, pal)
    n = len(icons)
    for k, fn in enumerate(icons):
        cx = w * SS * (0.5 + 0.46 * (k + 0.5) / n)
        cy = h * SS * (0.5 + (0.08 if k % 2 else -0.06))
        draw_icon(img, fn, cx, cy, h * SS * 0.55, pal)
    save(img, name, w, h)


def square(name, pal, fn, w=600, h=600, scale=0.55):
    img, _ = canvas(w, h, pal)
    draw_icon(img, fn, w * SS / 2, h * SS / 2, min(w, h) * SS * scale, pal)
    save(img, name, w, h)


def gallery():
    pal = PALETTES[2]
    for i in range(4):
        w, h = 600, 750
        img, _ = canvas(w, h, PALETTES[(2 + i * 3) % len(PALETTES)])
        s = w * SS * (0.75 if i % 2 == 0 else 0.6)
        draw_icon(img, shoe, w * SS / 2, h * SS / 2, s, pal)
        save(img, f"gallery-{i + 1}", w, h)


LOGO_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 64" width="200" height="64">
<g fill="currentColor" color="#334155">{shape}<text x="64" y="41" font-family="system-ui,-apple-system,Segoe UI,Roboto,sans-serif" font-size="22" font-weight="700" fill="#334155">{name}</text></g>
</svg>
"""
SHAPES = [
    '<circle cx="32" cy="32" r="18" fill="#334155"/><circle cx="32" cy="32" r="8" fill="#fff"/>',
    '<path d="M14 48 32 14 50 48Z" fill="#334155"/>',
    '<rect x="14" y="14" width="36" height="36" rx="10" fill="#334155"/><rect x="24" y="24" width="16" height="16" rx="4" fill="#fff"/>',
    '<path d="M12 36c8-16 16-16 20 0s12 16 20 0" stroke="#334155" stroke-width="6" fill="none" stroke-linecap="round"/>',
    '<path d="M12 46 24 22l8 12 8-16 12 28Z" fill="#334155"/>',
    '<circle cx="32" cy="32" r="18" fill="none" stroke="#334155" stroke-width="6"/><circle cx="40" cy="26" r="6" fill="#334155"/>',
]
BRANDS = ["Nórdika", "Altavista", "Kumo", "Brisa", "Valle Sur", "Oriente"]


def logos():
    for i, (name, shape) in enumerate(zip(BRANDS, SHAPES)):
        (OUT / f"brand-{i + 1}.svg").write_text(LOGO_SVG.format(shape=shape, name=name), encoding="utf-8")
    (OUT / "logo.svg").write_text(
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">'
        '<rect width="64" height="64" rx="16" fill="#6366f1"/>'
        '<path d="M18 44V22l14 12 14-12v22" stroke="#fff" stroke-width="6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>'
        "</svg>\n",
        encoding="utf-8",
    )


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    products()
    wide("hero", PALETTES[0], [headphones, watch, speaker])
    wide("banner", PALETTES[1], [backpack, lamp])
    wide("slide-1", PALETTES[6], [headphones, speaker])
    wide("slide-2", PALETTES[4], [shoe, watch, backpack])
    wide("slide-3", PALETTES[7], [mug, notebook])
    square("category-tecnologia", PALETTES[0], headphones)
    square("category-hogar", PALETTES[2], lamp)
    square("category-moda", PALETTES[4], backpack)
    square("category-deportes", PALETTES[3], shoe)
    square("card", PALETTES[5], notebook, w=800, h=600)
    gallery()
    logos()


if __name__ == "__main__":
    main()
