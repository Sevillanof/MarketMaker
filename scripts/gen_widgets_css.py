#!/usr/bin/env python3
"""Genera style/widgets-new.css a partir de scripts/widgets-maqueta.css.

La maqueta (treinta-disenos-widgets.html) marca el diseño de cada widget con
una clase en el propio elemento (`.hero.v2`) y consulta el ancho del contenedor
de muestra (`@container sample`). En la tienda el diseño va en el envoltorio
del bloque (`.mk-d2` en escritorio y `.mk-m2` en celular, ver `domain::Look`) y
el contenedor es el propio widget (`.mk-nw`), así que cada regla de diseño se
emite dos veces: para el ancho de escritorio con `.mk-dN` y para el ancho de
celular con `.mk-mN`. Las clases se prefijan con `nw-` para no chocar con las
del resto de la tienda.

Uso: python3 scripts/gen_widgets_css.py
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = re.sub(r"/\*.*?\*/", "", (ROOT / "scripts" / "widgets-maqueta.css").read_text(), flags=re.S)

WIDE = "(min-width:701px)"
NARROW = "(max-width:700px)"


def blocks(css):
    """Parte `css` en elementos de primer nivel: (prelude, cuerpo)."""
    out, i, n = [], 0, len(css)
    while i < n:
        j = css.find("{", i)
        if j < 0:
            break
        prelude = css[i:j].strip()
        depth, k = 1, j + 1
        while depth:
            c = css[k]
            depth += (c == "{") - (c == "}")
            k += 1
        out.append((prelude, css[j + 1 : k - 1].strip()))
        i = k
    return out


def split_selectors(sel):
    parts, depth, cur = [], 0, ""
    for ch in sel:
        if ch in "([":
            depth += 1
        if ch in ")]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur.strip())
            cur = ""
        else:
            cur += ch
    parts.append(cur.strip())
    return [p for p in parts if p]


def convert_selector(sel):
    """-> (selector sin la clase de diseño, N o None, texto antes del selector)."""
    sel = re.sub(r"^\.store\b", ".mk-nw", sel)
    sel = sel.replace(".store", ".mk-nw")
    designs = set(re.findall(r"\.v(\d)\b", sel))
    sel = re.sub(r"\.v\d\b", "", sel)
    sel = re.sub(r"(?<![\w-])\.(?!mk-)([a-zA-Z][\w-]*)", r".nw-\1", sel)
    # Elementos sueltos (h4, p, img...) quedan dentro del widget.
    if not sel.startswith(".mk-nw") and not sel.startswith(".nw-"):
        sel = ".mk-nw " + sel
    return sel, (designs.pop() if designs else None)


def rule_for(selectors, body, mobile_ctx):
    """Reglas de un bloque de selectores. Si `mobile_ctx`, los diseños usan .mk-mN."""
    plain, per_design = [], {}
    for s in selectors:
        sel, d = convert_selector(s)
        if d is None:
            plain.append(sel)
        else:
            per_design.setdefault(d, []).append(sel)
    out = []
    if plain:
        out.append(f"{', '.join(plain)} {{ {body} }}")
    for d, sels in sorted(per_design.items()):
        if mobile_ctx:
            sels_c = ", ".join(f".mk-m{d} {s}" for s in sels)
            out.append(f"{sels_c} {{ {body} }}")
        else:
            wide = ", ".join(f".mk-d{d} {s}" for s in sels)
            narrow = ", ".join(f".mk-m{d} {s}" for s in sels)
            out.append(f"@container nw {WIDE} {{ {wide} {{ {body} }} }}")
            out.append(f"@container nw {NARROW} {{ {narrow} {{ {body} }} }}")
    return out


def convert(css):
    out = []
    for prelude, body in blocks(css):
        if prelude.startswith("@container"):
            q = prelude.replace("sample", "nw")
            inner = []
            for p, b in blocks(body):
                inner += rule_for(split_selectors(p), b, True)
            out.append(f"{q} {{\n  " + "\n  ".join(inner) + "\n}")
        elif prelude.startswith("@media"):
            inner = []
            for p, b in blocks(body):
                for r in rule_for(split_selectors(p), b, False):
                    inner.append(r)
            out.append(f"{prelude} {{\n  " + "\n  ".join(inner) + "\n}")
        else:
            out += rule_for(split_selectors(prelude), body, False)
    return "\n".join(out)


# La maqueta pone `min-height` solo en las tarjetas que no son «Lateral»
# (`.products:not(.v1) ...`); acá es la regla común y «Lateral» la anula.
SRC = SRC.replace(
    ".products:not(.v1) .product h5 { min-height:2.7em; }",
    ".products .product h5 { min-height:2.7em; }\n  .products.v1 .product h5 { min-height:0; }",
)

# Las fotos van dentro de un cuadro `.nw-img` (el componente Image arma su
# propio envoltorio), así que las reglas de `img` pasan a ese cuadro. Las
# columnas, la proporción y las esquinas salen de variables del bloque.
SRC = re.sub(r"(\.category(?:\.v\d)?) img", r"\1 .img", SRC)
SRC = re.sub(r"(\.product(?:\.v\d)?) > img", r"\1 > .img", SRC)
SRC = re.sub(r"(\.story(?:\.v\d)?) img", r"\1 .img", SRC)
SRC = SRC.replace(
    ".benefits { display:grid; grid-template-columns:repeat(3,", ".benefits { display:grid; grid-template-columns:repeat(var(--n,3),"
).replace(
    ".categories { display:grid; grid-template-columns:repeat(3,", ".categories { display:grid; grid-template-columns:repeat(var(--n,3),"
).replace(
    ".products { display:grid; grid-template-columns:repeat(4,", ".products { display:grid; grid-template-columns:repeat(var(--n,4),"
).replace(
    ".category .img { width:100%; aspect-ratio:1.25; object-fit:cover; border-radius:var(--radius); }",
    ".category .img { width:100%; aspect-ratio:var(--nw-ratio,1.25); border-radius:var(--nw-rad,var(--radius)); }",
)

SRC = SRC.replace(
    ".footer-grid { display:grid; grid-template-columns:2fr 1fr 1fr;",
    ".footer-grid { display:grid; grid-template-columns:2fr repeat(var(--fc,2),1fr);",
)

HEADER = """/* Widgets nuevos (generado por scripts/gen_widgets_css.py; no editar a mano).
   Copia fiel de treinta-disenos-widgets.html. El diseño de escritorio va en
   .mk-dN y el de celular en .mk-mN del envoltorio del bloque. */
.mk-nw {
  container: nw / inline-size;
  --bg: var(--color-base-100);
  --surface: var(--color-base-200);
  --panel: var(--mk-panel, var(--color-base-100));
  --ink: var(--color-base-content);
  --muted: var(--mk-muted, color-mix(in oklab, var(--color-base-content) 75%, transparent));
  --accent: var(--color-primary);
  --on-accent: var(--color-primary-content);
  --line: var(--mk-line, var(--color-base-300));
  --radius: var(--radius-box);
  --button-radius: var(--mk-btn-radius, var(--radius-field));
  --heading: var(--mk-heading, inherit);
  --hero: var(--mk-hero-bg, var(--color-base-200));
  --footer: var(--mk-footer-bg, var(--color-neutral));
  --footer-ink: var(--mk-footer-ink, var(--color-neutral-content));
  background: var(--bg);
  color: var(--ink);
  line-height: 1.45;
  min-width: 0;
}
.nw-benefits, .nw-categories, .nw-products { grid-template-columns: repeat(var(--n, 3), minmax(0, 1fr)); }
"""

TAIL = """
.nw-img { position: relative; display: block; overflow: hidden; background: var(--surface); }
.nw-img > .mk-img { position: absolute; inset: 0; }
.nw-category { display: block; color: inherit; text-decoration: none; }
.nw-product { position: relative; }
.nw-product-info s { color: var(--muted); font-size: 11px; font-weight: 400; margin-left: 6px; }
.nw-badge { position: absolute; left: 8px; top: 8px; z-index: 1; padding: 2px 8px; font-size: 10px; font-weight: 700; background: var(--ink); color: var(--bg); border-radius: var(--radius); }
.nw-foot-link { color: inherit; text-decoration: none; }
.nw-foot-icons { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 14px; }
.nw-foot-icons > * { display: grid; place-items: center; width: 34px; height: 34px; border: 1px solid currentColor; border-radius: 50%; opacity: .85; }
.nw-footer-legal { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 6px 18px; margin-top: 18px; max-width: none; opacity: .75; }
.nw-footer .nw-footer-legal { margin-top: 18px; max-width: none; }
@container nw (min-width:701px) { .mk-d1 .nw-story.nw-right .nw-img, .mk-d3 .nw-story.nw-right .nw-img { order: 2; } }
@container nw (max-width:700px) { .mk-m1 .nw-story.nw-right .nw-img, .mk-m3 .nw-story.nw-right .nw-img { order: 2; } }
.nw-pager { display: flex; justify-content: center; margin-top: 22px; }
.nw-ann-inner { display: block; color: inherit; text-decoration: none; }
/* Menú del celular: el botón «Menú» abre un desplegable flotante (popover). */
.nw-nav-mobile { position: relative; }
.nw-nav-mobile-btn { display: flex; align-items: center; gap: 8px; padding: 7px; font-size: 13px; font-weight: 700; cursor: pointer; }
.nw-linkbtn { display: inline-flex; align-items: center; gap: 4px; font-size: inherit; font-weight: inherit; cursor: pointer; }
.nw-nav-links { list-style: none; margin: 0; padding: 0; }
.nw-nav-links > li { list-style: none; }
.nw-upper > li { text-transform: uppercase; letter-spacing: .06em; }
.nw-nav-button { display: contents; }
@container nw (max-width: 700px) {
  .nw-navsearch, .nw-nav-button { display: none; }
  .mk-m2 .nw-navigation .nw-nav-row { display: grid; grid-template-columns: 1fr auto 1fr; padding: 0; min-height: 70px; }
  .mk-m2 .nw-navigation .nw-nav-mobile { grid-column: 1; grid-row: 1; justify-self: start; width: auto; }
  .mk-m2 .nw-navigation .nw-brand { grid-column: 2; grid-row: 1; }
  .mk-m2 .nw-navigation .nw-nav-icons { grid-column: 3; grid-row: 1; justify-self: end; }
}
.nw-ann-mark { display: none; }
@container nw (min-width: 701px) { .mk-d2 .nw-ann .nw-ann-mark { display: inline-block; } }
@container nw (max-width: 700px) { .mk-m2 .nw-ann .nw-ann-mark { display: inline-block; } }
/* La portada llena el alto del bloque (en la maqueta crece con su contenido). */
.nw-hero-wrap, .nw-hero { height: 100%; }
.nw-hero-photo { min-height: 0; }
@container nw (min-width: 701px) { .mk-d1 .nw-hero { margin: 0; } .mk-d1 .nw-hero-wrap { padding: 4%; } }
@container nw (max-width: 700px) { .mk-m1 .nw-hero { margin: 0; } .mk-m1 .nw-hero-wrap { padding: 4%; } }
.nw-hero.nw-veil::after { background: rgb(0 0 0 / var(--nw-veil, 0.68)); }
"""

if __name__ == "__main__":
    out = HEADER + "\n" + convert(SRC) + "\n" + TAIL
    (ROOT / "style" / "widgets-new.css").write_text(out)
    print("style/widgets-new.css:", len(out), "bytes")
