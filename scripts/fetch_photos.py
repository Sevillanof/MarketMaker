"""Baja las fotos de ejemplo de `public/img` desde Unsplash (licencia libre,
uso comercial sin atribución) y las guarda en AVIF + WebP.

    python3 scripts/fetch_photos.py

Los créditos están en public/img/CREDITOS.md. Necesita Pillow 11.3+ y salida
a images.unsplash.com.
"""

import io
import urllib.request
from pathlib import Path

from PIL import Image, ImageOps

OUT = Path(__file__).resolve().parent.parent / "public" / "img"

# product-N sigue el orden de la tienda demo: 1 auriculares, 2 mochila,
# 3 lámpara, 4 taza, 5 zapatillas, 6 reloj, 7 cuaderno, 8 parlante.
# nombre: (id de la foto en Unsplash, ancho, alto, (centro x, centro y) 0..1)
PHOTOS = {
    "product-1": ("1505740420928-5e560c06d30e", 800, 600, (0.5, 0.5)),
    "product-2": ("1553062407-98eeb64c6a62", 800, 600, (0.5, 0.5)),
    "product-3": ("1507473885765-e6ed057f782c", 800, 600, (0.5, 0.5)),
    "product-4": ("1514228742587-6b1558fcca3d", 800, 600, (0.5, 0.5)),
    "product-5": ("1491553895911-0055eca6402d", 800, 600, (0.5, 0.5)),
    "product-6": ("1524592094714-0f0654e20314", 800, 600, (0.5, 0.5)),
    "product-7": ("1531346878377-a5be20888e57", 800, 600, (0.5, 0.5)),
    "product-8": ("1545454675-3531b543be5d", 800, 600, (0.5, 0.5)),
    "category-moda": ("1558769132-cb1aea458c5e", 600, 600, (0.5, 0.5)),
    "category-hogar": ("1586023492125-27b2c045efd7", 600, 600, (0.5, 0.5)),
    "category-tecnologia": ("1550745165-9bc0b252726f", 600, 600, (0.5, 0.5)),
    "category-deportes": ("1579952363873-27f3bade9f55", 600, 600, (0.5, 0.5)),
    "slide-1": ("1441986300917-64674bd600d8", 1600, 600, (0.5, 0.5)),
    "slide-2": ("1445205170230-053b83016050", 1600, 600, (0.5, 0.5)),
    "slide-3": ("1553413077-190dd305871c", 1600, 600, (0.5, 0.5)),
    "banner": ("1531297484001-80022131f5a1", 1600, 600, (0.5, 0.6)),
    "hero": ("1493106641515-6b5631de4bb9", 1600, 600, (0.5, 0.5)),
    "card": ("1556905055-8f358a7a47b2", 800, 600, (0.5, 0.5)),
    "gallery-1": ("1565193566173-7a0ee3dbe261", 600, 750, (0.5, 0.5)),
    "gallery-2": ("1452860606245-08befc0ff44b", 600, 750, (0.5, 0.5)),
    "gallery-3": ("1556742049-0cfed4f6a45d", 600, 750, (0.5, 0.5)),
    "gallery-4": ("1513364776144-60967b0f800f", 600, 750, (0.5, 0.5)),
}


def fetch(photo_id):
    url = f"https://images.unsplash.com/photo-{photo_id}?w=2000&q=85&fm=jpg"
    with urllib.request.urlopen(url, timeout=60) as r:
        return Image.open(io.BytesIO(r.read())).convert("RGB")


for name, (pid, w, h, center) in PHOTOS.items():
    img = ImageOps.fit(fetch(pid), (w, h), Image.LANCZOS, centering=center)
    img.save(OUT / f"{name}.webp", quality=80, method=6)
    img.save(OUT / f"{name}.avif", quality=55)
    print(name, "ok")
