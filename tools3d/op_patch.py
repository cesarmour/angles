"""Gera a textura das insígnias do operador no mesmo arranjo da original (escudo do ombro à esquerda, faixa das costas à
direita; 2048x512, invertida na vertical). Sem texto: o escudo leva só o leque de fatiar a porta e a faixa vira velcro liso.
python3 tools3d/op_patch.py <saida.png>"""
import sys
from PIL import Image, ImageDraw, ImageFilter, ImageOps
out = sys.argv[1]
W, H = 2048, 512
im = Image.new('RGB', (W, H), (9, 10, 12))
d = ImageDraw.Draw(im)
bone, cyan, cloth = (226, 222, 208), (86, 200, 220), (15, 16, 19)
# faixa das costas: painel de velcro vazio, com a costura da borda
x0, y0, x1, y1 = 540, 44, 1996, 486
d.rectangle([x0, y0, x1, y1], fill=cloth, outline=(34, 36, 41), width=10)
for y in range(y0 + 22, y1 - 14, 9):
    d.line([(x0 + 22, y), (x1 - 22, y)], fill=(19, 20, 24), width=3)
# escudo do ombro: o leque que o operador abre numa porta, entre os dois batentes
cx, top, bot, hw = 280, 110, 440, 176
pts = [(cx - hw + 34, top), (cx + hw - 34, top), (cx + hw, top + 44), (cx + hw, bot - 96), (cx, bot), (cx - hw, bot - 96), (cx - hw, top + 44)]
d.polygon(pts, fill=cloth)
d.line(pts + [pts[0]], fill=bone, width=9, joint='curve')
py = top + 205
d.pieslice([cx - 150, py - 150, cx + 150, py + 150], 215, 325, fill=cyan)
d.line([(cx - 132, py), (cx - 36, py)], fill=bone, width=11)
d.line([(cx + 36, py), (cx + 132, py)], fill=bone, width=11)
im = im.filter(ImageFilter.GaussianBlur(0.8))
ImageOps.flip(im).save(out)
print("patch", out)
