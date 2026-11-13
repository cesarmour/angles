"""Ícone do Kill House: o nome em duas linhas, letra pesada e estreita, branco sobre preto com HOUSE na cor do OSB.
Cada linha é ajustada pelo tamanho da letra para ocupar a mesma largura (sem esticar o espaço entre letras).

Gera assets/icon/icon-1024.png, assets/icon/AppIcon.icns e site/img/icon.png.
Uso: python3 tools/make_icon.py
"""
import os, subprocess, shutil, tempfile
from PIL import Image, ImageDraw, ImageFont, ImageFilter

S = 4
N = 1024 * S
FONT = "/System/Library/Fonts/Supplemental/Impact.ttf"
WHITE, OSB = (244, 244, 242, 255), (222, 184, 116, 255)

def fit(txt, width, track):
    """Corpo da letra para o texto ocupar `width`, com `track` (fração do corpo) entre letras."""
    size = 400
    for _ in range(14):
        f = ImageFont.truetype(FONT, size)
        w = sum(f.getlength(c) for c in txt) + track * size * (len(txt) - 1)
        size = max(10, int(size * width / w))
    return ImageFont.truetype(FONT, size), size

def draw_line(d, txt, f, size, x, y, track, fill):
    bb = f.getbbox("H")
    for c in txt:
        d.text((x, y - bb[1]), c, font=f, fill=fill)
        x += f.getlength(c) + track * size
    return bb[3] - bb[1]

icon = Image.new("RGBA", (N, N), (0, 0, 0, 0))
sh = Image.new("L", (N, N), 0)
ImageDraw.Draw(sh).rounded_rectangle([100 * S, 118 * S, 924 * S, 942 * S], radius=186 * S, fill=120)
icon.paste((0, 0, 0, 255), (0, 0), sh.filter(ImageFilter.GaussianBlur(14 * S)))
d = ImageDraw.Draw(icon)
d.rounded_rectangle([100 * S, 100 * S, 924 * S, 924 * S], radius=186 * S, fill=(8, 9, 10, 255))

X1, X2, GAP, TRACK = 196 * S, 828 * S, 34 * S, 0.035
f1, s1 = fit("KILL", X2 - X1, TRACK)
f2, s2 = fit("HOUSE", X2 - X1, TRACK)
h1, h2 = (f.getbbox("H")[3] - f.getbbox("H")[1] for f in (f1, f2))
y = (N - (h1 + GAP + h2)) // 2
draw_line(d, "KILL", f1, s1, X1, y, TRACK, WHITE)
draw_line(d, "HOUSE", f2, s2, X1, y + h1 + GAP, TRACK, OSB)

out = icon.resize((1024, 1024), Image.LANCZOS)
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
out.save(os.path.join(root, "assets", "icon", "icon-1024.png"))
out.resize((256, 256), Image.LANCZOS).save(os.path.join(root, "site", "img", "icon.png"))
tmp = tempfile.mkdtemp()
iset = os.path.join(tmp, "AppIcon.iconset"); os.mkdir(iset)
for sz in (16, 32, 128, 256, 512):
    out.resize((sz, sz), Image.LANCZOS).save(os.path.join(iset, f"icon_{sz}x{sz}.png"))
    out.resize((sz * 2, sz * 2), Image.LANCZOS).save(os.path.join(iset, f"icon_{sz}x{sz}@2x.png"))
subprocess.run(["iconutil", "-c", "icns", iset, "-o", os.path.join(root, "assets", "icon", "AppIcon.icns")], check=True)
shutil.rmtree(tmp)
print("ok")
