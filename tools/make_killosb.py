"""Gera a textura de OSB da kill house (assets/tex/killosb_*.jpg): chapa de lascas de madeira prensadas.

Uma chapa de 1,22 m por repetição, 2048 px (0,6 mm por pixel), com relevo das lascas e a junta escura na borda.
Uso: python3 tools/make_killosb.py
"""
import math, os, random
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

N = 2048
MM = N / 1220.0                      # pixels por milímetro
random.seed(11)
rng = np.random.default_rng(11)

PALETTE = [(236, 212, 160), (224, 192, 134), (210, 172, 112), (192, 150, 92), (244, 226, 184), (228, 200, 150), (202, 164, 108)]
DARK = [(150, 108, 66), (128, 90, 54), (172, 128, 80)]

col = Image.new("RGB", (N, N), (200, 162, 106))
hgt = Image.new("L", (N, N), 120)
dc, dh = ImageDraw.Draw(col), ImageDraw.Draw(hgt)

def strand(cx, cy, ln, wd, ang, rgb, h):
    ca, sa = math.cos(ang), math.sin(ang)
    # lasca: quadrilátero comprido, com as pontas um pouco tortas
    k1, k2 = random.uniform(-0.35, 0.35) * wd, random.uniform(-0.35, 0.35) * wd
    pts = [(-ln / 2, -wd / 2 + k1), (ln / 2, -wd / 2 + k2), (ln / 2 + random.uniform(-0.1, 0.1) * ln, wd / 2 + k2), (-ln / 2 + random.uniform(-0.1, 0.1) * ln, wd / 2 + k1)]
    world = [(cx + x * ca - y * sa, cy + x * sa + y * ca) for x, y in pts]
    edge = tuple(int(c * 0.80) for c in rgb)
    # fibras ao comprido da lasca: três ou quatro riscos de tom um pouco diferente
    fibres = []
    for _ in range(random.randint(2, 5)):
        off = random.uniform(-0.4, 0.4) * wd
        tone = random.uniform(0.86, 1.08)
        a, b = (-ln * 0.46, off), (ln * 0.46, off + random.uniform(-0.08, 0.08) * wd)
        fibres.append(([(cx + x * ca - y * sa, cy + x * sa + y * ca) for x, y in (a, b)], tuple(min(255, int(c * tone)) for c in rgb)))
    xs, ys = [p[0] for p in world], [p[1] for p in world]
    for ox in (-N, 0, N):
        if max(xs) + ox < 0 or min(xs) + ox > N: continue
        for oy in (-N, 0, N):
            if max(ys) + oy < 0 or min(ys) + oy > N: continue
            poly = [(x + ox, y + oy) for x, y in world]
            dc.polygon(poly, fill=rgb, outline=edge)
            dh.polygon(poly, fill=h)
            for line, tone in fibres:
                dc.line([(x + ox, y + oy) for x, y in line], fill=tone, width=max(1, int(wd * 0.12)))

# camadas: lascas grandes embaixo, miúdas por cima; a maioria deitada ao comprido da chapa (em pé na parede)
for layer, (count, lmin, lmax, wmin, wmax) in enumerate([(2600, 70, 170, 12, 34), (3200, 50, 130, 8, 26), (2600, 25, 80, 4, 14)]):
    for i in range(count):
        ang = random.gauss(math.pi / 2, 0.75) if random.random() < 0.8 else random.uniform(0, math.pi)
        rgb = random.choice(DARK) if random.random() < 0.04 else random.choice(PALETTE)
        f = random.uniform(0.9, 1.06)
        rgb = tuple(min(255, int(c * f)) for c in rgb)
        strand(random.uniform(0, N), random.uniform(0, N), random.uniform(lmin, lmax) * MM, random.uniform(wmin, wmax) * MM, ang, rgb, random.randint(60, 200))

a = np.asarray(col).astype(np.float32) / 255.0
# grão miúdo e manchas largas de tom (periódicas, para a emenda não aparecer)
y, x = np.mgrid[0:N, 0:N] / N
blot = np.zeros((N, N), np.float32)
for _ in range(10):
    fx, fy = rng.integers(0, 3), rng.integers(0, 3)
    if fx == 0 and fy == 0: fx = 1
    blot += np.sin(2 * np.pi * (fx * x + fy * y) + rng.uniform(0, 6.28)) / (1 + fx + fy)
blot /= np.abs(blot).max()
a *= (1.0 + 0.035 * blot[..., None])
a *= (1.0 + rng.normal(0, 0.022, (N, N, 1)).astype(np.float32))

h = np.asarray(hgt.filter(ImageFilter.GaussianBlur(1.1))).astype(np.float32) / 255.0
# sombra leve na borda das lascas de cima (oclusão): onde a altura cai, escurece
gx, gy = np.roll(h, -1, 1) - np.roll(h, 1, 1), np.roll(h, -1, 0) - np.roll(h, 1, 0)
a *= (1.0 - np.clip(np.sqrt(gx * gx + gy * gy) * 1.6, 0, 0.22))[..., None]

# contraste mais baixo: de longe a chapa é um bege quase uniforme, as lascas aparecem de perto
m = a.mean(axis=(0, 1), keepdims=True)
a = m + (a - m) * 0.72

# junta entre chapas: fresta escura de 2 mm na borda vertical e na horizontal
for c in (x, y):
    d = np.minimum(c, 1 - c) * 1220.0
    a *= (1 - 0.72 * np.clip(1 - d / 1.6, 0, 1))[..., None]

out = Image.fromarray((np.clip(a, 0, 1) * 255).astype(np.uint8))
tex = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "tex")
out.save(os.path.join(tex, "killosb_diff.jpg"), quality=88)
# relevo: normal a partir da altura das lascas
s = 1.0
n = np.dstack([-gx * s, gy * s, np.ones_like(h)])
n /= np.linalg.norm(n, axis=2, keepdims=True)
Image.fromarray(((n * 0.5 + 0.5) * 255).astype(np.uint8)).save(os.path.join(tex, "killosb_nor.jpg"), quality=90)
# R = oclusão, G = aspereza, B = metal
Image.new("RGB", (64, 64), (255, 225, 0)).save(os.path.join(tex, "killosb_arm.jpg"), quality=95)
print("ok", a.mean(axis=(0, 1)))
