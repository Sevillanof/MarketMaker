#!/usr/bin/env python3
"""Genera los 12 temas de tienda (maqueta temas-tienda.html) para daisyUI.

Imprime dos bloques: el CSS para style/tailwind.css y las entradas de
THEME_BASES de crates/domain/src/theme.rs. Los tokens salen de la maqueta.
"""
import sys
T = {
 # id: (label, bg, surface, panel, ink, muted, primary, on_primary, line, radius, button_radius, footer_bg, footer_ink, dark)
 "linea":  ("Línea",  "#faf8f4","#f0ece4","#fffefa","#292821","#625e54","#35382e","#fffefa","#d9d3c7",2,2,"#35382e","#fffefa",0),
 "pulso":  ("Pulso",  "#f6f7ff","#e9edff","#ffffff","#10193b","#485574","#2947c9","#ffffff","#cbd4f5",16,999,"#111d4c","#ffffff",0),
 "noche":  ("Noche",  "#18221e","#26342c","#202c26","#f4f3e8","#c6d1c4","#c5dcae","#17241a","#485b4d",10,4,"#101a15","#f4f3e8",1),
 "brisa":  ("Brisa",  "#f5fafb","#dceff0","#ffffff","#16323b","#405c64","#17627b","#ffffff","#567f89",16,999,"#153f4e","#ffffff",0),
 "tinta":  ("Tinta",  "#fbfaf7","#e8e7e2","#ffffff","#181818","#52514e","#171717","#ffffff","#817e78",0,0,"#171717","#ffffff",0),
 "citrico":("Cítrico","#fffaf0","#fff0d6","#fffdf6","#352415","#705039","#a33b15","#ffffff","#96704f",10,8,"#733013","#fffaf0",0),
 "aura":   ("Aura",   "#faf8fc","#eee8f5","#ffffff","#33273f","#62526d","#654778","#ffffff","#88719a",14,999,"#49364f","#ffffff",0),
 "brote":  ("Brote",  "#f7faf4","#e2edda","#ffffff","#1c3323","#4c634e","#315f3b","#ffffff","#6a8a6e",14,4,"#244732","#ffffff",0),
 "orbita": ("Órbita", "#111f33","#223854","#1b2c45","#f5f8fc","#c6d4e4","#9ee4e9","#102b38","#7894aa",6,4,"#091626","#f5f8fc",1),
 "dalia":  ("Dalia",  "#fff9f8","#f7e6e5","#ffffff","#45262d","#705057","#943d51","#ffffff","#9c7278",5,2,"#5b3039","#ffffff",0),
 "cacao":  ("Cacao",  "#261b17","#47332c","#352720","#fff7eb","#e2cec0","#e6c49c","#2a1b15","#aa8d77",3,2,"#1c120f","#fff7eb",1),
 "sol":    ("Sol",    "#fffdf6","#fff0bd","#ffffff","#30270e","#65552b","#544000","#ffffff","#95752b",10,999,"#3c3010","#ffffff",0),
}
LIGHT = [("#0369a1","#ffffff"),("#15803d","#ffffff"),("#b45309","#ffffff"),("#b91c1c","#ffffff")]
DARK  = [("#7dd3fc","#082f49"),("#86efac","#052e16"),("#fcd34d","#451a03"),("#fca5a5","#450a0a")]
KEYS = ["base-100","base-200","base-300","base-content","primary","primary-content","secondary","secondary-content",
        "accent","accent-content","neutral","neutral-content","info","info-content","success","success-content",
        "warning","warning-content","error","error-content"]
def colors(t):
    _,bg,sf,pn,ink,mu,pr,onp,ln,r,br,fb,fi,dk = T[t]
    st = (DARK if dk else LIGHT)
    v = [bg,sf,ln,ink,pr,onp,mu,bg,pr,onp,fb,fi]
    for a,b in st: v += [a,b]
    return v
def px(n): return min(n,32)
def css():
    out=[]
    for t in T:
        _,bg,sf,pn,ink,mu,pr,onp,ln,r,br,fb,fi,dk = T[t]
        out.append(f'@plugin "./daisyui-theme.mjs" {{\n  name: "{t}";\n  default: false;\n  prefersdark: false;\n  color-scheme: {"dark" if dk else "light"};')
        for k,c in zip(KEYS,colors(t)): out.append(f'  --color-{k}: {c};')
        out.append(f'  --radius-box: {px(r)/16}rem;\n  --radius-field: {px(br)/16}rem;\n  --radius-selector: {px(r)/16}rem;\n  --size-field: 0.25rem;\n  --size-selector: 0.25rem;\n  --border: 1px;\n  --depth: 0;\n  --noise: 0;\n}}')
    return "\n".join(out)
def rust():
    out=[]
    for t in T:
        _,bg,sf,pn,ink,mu,pr,onp,ln,r,br,fb,fi,dk = T[t]
        c=colors(t)
        rows=[", ".join(f'"{x}"' for x in c[i:i+4]) for i in range(0,20,4)]
        out.append(f'''    // {T[t][0]}
    ThemeBase {{
        dark: {str(bool(dk)).lower()},
        colors: [
            {(","+chr(10)+"            ").join(rows)},
        ],
        radius_box: {px(r)},
        radius_field: {px(br)},
        radius_selector: {px(r)},
        size_field: 4.0,
        size_selector: 4.0,
        border: 1.0,
        depth: false,
        noise: false,
    }},''')
    return "\n".join(out)
if __name__=="__main__":
    print(css() if sys.argv[1:]==["css"] else rust())
