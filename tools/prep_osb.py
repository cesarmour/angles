"""Prepara a textura de OSB das paredes da kill house a partir da "Oriented Strand Board" do Poly Haven (CC0, Dimitrios Savva).

Baixe os arquivos jpg de 2K (diff, nor_gl, arm) de https://polyhaven.com/a/oriented_strand_board para uma pasta, com os
nomes osb_diff.jpg, osb_nor_gl.jpg e osb_arm.jpg. Uso: python3 tools/prep_osb.py <pasta> assets/tex
Escurece a junta a cada chapa de 1,22 m, tira o brilho de verniz e grava killosb_diff/nor/arm.jpg.
"""
import sys, numpy as np
from PIL import Image
src, tex = sys.argv[1], sys.argv[2]
d = np.asarray(Image.open(f'{src}/osb_diff.jpg').convert('RGB')).astype(np.float32) / 255
N = d.shape[0]
print('media srgb', d.mean(axis=(0, 1)))
# a textura cobre 2,51 m: usada como duas chapas de 1,22 m por repetição; junta escura de 2 mm a cada chapa
y, x = np.mgrid[0:N, 0:N] / N
for c in (x, y):
    t = (c * 2.0) % 1.0
    dist = np.minimum(t, 1 - t) * 1220.0
    d *= (1 - 0.7 * np.clip(1 - dist / 1.6, 0, 1))[..., None]
Image.fromarray((np.clip(d, 0, 1) * 255).astype(np.uint8)).save(f'{tex}/killosb_diff.jpg', quality=90)
Image.open(f'{src}/osb_nor_gl.jpg').convert('RGB').save(f'{tex}/killosb_nor.jpg', quality=90)
a = np.asarray(Image.open(f'{src}/osb_arm.jpg').convert('RGB')).astype(np.float32) / 255
print('aspereza media', a[..., 1].mean())
a[..., 1] = np.clip(0.72 + a[..., 1] * 0.28, 0, 1)   # chapa crua, sem verniz
a[..., 2] = 0
Image.fromarray((a * 255).astype(np.uint8)).save(f'{tex}/killosb_arm.jpg', quality=90)
Image.open(f'{tex}/killosb_diff.jpg').crop((0, 0, 1100, 1100)).resize((700, 700)).save(f'{src}/prev.jpg', quality=88)
