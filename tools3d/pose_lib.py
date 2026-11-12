"""Clipes de queda feitos por poses-chave, usados pelos dois bonecos (ameaça e refém).

O boneco não tomba rígido como uma placa: os joelhos dobram, o quadril desce e o corpo desaba. Há quatro variações
(o jogo sorteia uma a cada queda): para a frente, para a esquerda, para a direita e sentando para trás. Todas terminam
ocupando pouco chão, para não atravessar a parede que estiver perto.

Cada script de conversão passa as próprias funções de osso (os dois esqueletos têm nomes diferentes):
  B(chave)            -> osso de pose; chaves: hips, spine0..2, neck, head, thigh_l/r, calf_l/r, arm_l/r, fore_l/r
  world(osso)         -> matriz do osso no mundo
  set_world(osso, M)  -> põe o osso nessa matriz
  point(osso, d)      -> aponta o osso para a direção d (mundo)
  restore()           -> volta à pose de partida
  key_all(quadro)     -> grava chave de todos os ossos
O boneco olha para -Y; o lado esquerdo dele é +X; Z é para cima.
"""
from math import radians
from mathutils import Matrix, Vector

VARIANTS = ('Fall1', 'Fall2', 'Fall3', 'Fall4')

# Pose de referência em pé, de onde cada variação parte.
STAND = dict(drop=0.0, pitch=0.0, roll=0.0, spine=[(0, 0, 1), (0, 0, 1), (0, 0, 1)], neck=(0, 0, 1), head=(0, -0.1, 1),
             thigh=(0.08, 0.0, -1.0), calf=(0.0, 0.0, -1.0), arm=(0.25, 0.0, -0.97), fore=(0.15, -0.2, -0.95), lat=(0.0, 0.0, 0.0))

def _side(sd):
    # desaba de lado: sd = +1 para a esquerda dele, -1 para a direita
    return dict(drop=0.80, pitch=12, roll=50 * sd, spine=[(0.45 * sd, -0.2, 0.87), (0.72 * sd, -0.25, 0.64), (0.92 * sd, -0.28, 0.27)],
                neck=(0.95 * sd, -0.25, -0.1), head=(0.85 * sd, -0.3, -0.42), thigh=(0.16, -0.80, -0.50), calf=(-0.15 * sd, 0.97, -0.05),
                arm=(0.2, -0.45, -0.85), fore=(0.12, -0.55, -0.8), lat=(0.25 * sd, 0.45 * sd, 0.5 * sd))

# Pose final de cada variação. drop = quanto o quadril desce, como fração da altura dele em pé;
# lat = deslocamento lateral somado a coxas, braços e antebraços (quando o corpo cai de lado).
FINAL = {
    # de joelhos, tronco dobrado para a frente
    'Fall1': dict(drop=0.77, pitch=31, roll=9, spine=[(0.05, -0.58, 0.81), (0.08, -0.88, 0.46), (0.11, -0.99, 0.08)], neck=(0.12, -0.94, -0.3), head=(0.14, -0.74, -0.66),
                  thigh=(0.16, -0.83, -0.54), calf=(0.0, 1.0, -0.02), arm=(0.22, -0.66, -0.72), fore=(0.12, -0.84, -0.52), lat=(0.0, 0.0, 0.0)),
    'Fall2': _side(1.0),
    'Fall3': _side(-1.0),
    # senta para trás, joelhos para cima, tronco e cabeça caídos para trás
    'Fall4': dict(drop=0.80, pitch=-28, roll=6, spine=[(0, 0.3, 0.95), (0, 0.5, 0.87), (0.05, 0.62, 0.78)], neck=(0.05, 0.5, 0.86), head=(0.08, 0.75, 0.6),
                  thigh=(0.22, -0.72, 0.62), calf=(0.0, -0.25, -0.97), arm=(0.35, 0.1, -0.93), fore=(0.3, -0.25, -0.9), lat=(0.0, 0.0, 0.0)),
}
# (quadro a 24 qps, quanto da pose final já foi atingido): o joelho cede rápido e o corpo assenta no fim
TIMING = ((5, 0.30), (10, 0.72), (15, 0.96), (19, 1.0))


def _mix(a, b, w):
    if isinstance(a, (int, float)): return a + (b - a) * w
    if isinstance(a, list): return [_mix(x, y, w) for x, y in zip(a, b)]
    return tuple(x + (y - x) * w for x, y in zip(a, b))


def make_fall(B, world, set_world, point, restore, key_all, variant='Fall1'):
    """Grava as chaves de uma variação de queda na ação ativa; devolve o último quadro."""
    restore()
    hip = B('hips')
    h0 = world(hip).translation.z
    final = FINAL[variant]
    key_all(1)
    for frame, w in TIMING:
        st = {k: _mix(STAND[k], final[k], w) for k in STAND}
        restore()
        M = world(hip); h = M.translation.copy()
        R = Matrix.Rotation(radians(st['roll']), 4, 'Y') @ Matrix.Rotation(radians(st['pitch']), 4, 'X')
        set_world(hip, Matrix.Translation(Vector((h.x, h.y, h.z - st['drop'] * h0))) @ R @ Matrix.Translation(-h) @ M)
        for name, d in zip(('spine0', 'spine1', 'spine2'), st['spine']): point(B(name), d)
        point(B('neck'), st['neck']); point(B('head'), st['head'])
        lt, la, lf = st['lat']
        for s, sx in (('l', 1.0), ('r', -1.0)):
            t, a, f = st['thigh'], st['arm'], st['fore']
            point(B('thigh_' + s), (sx * t[0] + lt, t[1], t[2]))
            point(B('calf_' + s), st['calf'])
            point(B('arm_' + s), (sx * a[0] + la, a[1], a[2]))
            point(B('fore_' + s), (sx * f[0] + lf, f[1], f[2]))
        key_all(frame)
    return TIMING[-1][0]
