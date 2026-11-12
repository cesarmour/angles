# Mech original do projeto ("Walker"): modelado e animado inteiramente por este script, sem partir de nenhum modelo de terceiros.
#   blender -b --python tools3d/make_mech.py -- <saida.glb> [pasta_de_previas]
# Desenho: tanque bípede de corpo baixo e comprido, inclinado para a frente; focinho em cunha com visor estreito e metralhadora
# dupla no queixo; pernas de joelho invertido com pé de três garras; canhão longo num pilone à direita; casulo de seis mísseis à
# esquerda; bloco de motor com aletas e cauda de contrapeso. É um desenho próprio do gênero, não a cópia de um mech existente.
# Terceira versão (o dono do projeto achou a anterior "muito poligonal"): bordas chanfradas e sombreamento suave, cilindros
# redondos, casco e focinho afilados, detalhe mecânico (pistões, mangueiras, parafusos, grelhas, faróis, lançadores de fumaça,
# número pintado) e textura de metal pintado com desgaste (texturas CC0 do Poly Haven que o jogo já usa, retingidas aqui).
# O modelo olha para -Y (no jogo, +Z). Peças rígidas presas a nós vazios (juntas); cada clipe anima todos os nós:
# Idle (espera, canhão erguido), Aim (canhão nivelado), Shoot (coice), Walk (passo no lugar) e Fall1 (desaba para a frente).
import bpy, bmesh, sys, os, tempfile
import numpy as np
from math import radians, sin, cos, pi, atan2
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index('--') + 1:]
out = a[0]
prev = a[1] if len(a) > 1 else None
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
sc.render.fps = 30
TEX = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'assets', 'tex')
TMP = tempfile.mkdtemp(prefix='mech_tex_')

# ------------------------------------------------------------------ materiais
def tinted(src, name, sat, mul, size=1024):
    """Retinge uma textura de cor: mistura com o cinza (sat = quanto da cor original fica) e multiplica por `mul`."""
    img = bpy.data.images.load(os.path.join(TEX, src)); img.scale(size, size)
    px = np.array(img.pixels[:], dtype=np.float32).reshape(-1, 4)
    g = px[:, :3] @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    for k in range(3): px[:, k] = np.clip((g + (px[:, k] - g) * sat) * mul[k], 0.0, 1.0)
    img.pixels = px.reshape(-1).tolist()
    path = os.path.join(TMP, name + '.jpg'); img.filepath_raw = path; img.file_format = 'JPEG'; img.save()
    return bpy.data.images.load(path)
def plain_tex(src, size=1024):
    img = bpy.data.images.load(os.path.join(TEX, src)); img.scale(size, size)
    path = os.path.join(TMP, src); img.filepath_raw = path; img.file_format = 'JPEG'; img.save()
    im2 = bpy.data.images.load(path); im2.colorspace_settings.name = 'Non-Color'
    return im2
def texmat(name, diff, nor, arm):
    m = bpy.data.materials.new(name); m.use_nodes = True
    nt = m.node_tree; b = nt.nodes['Principled BSDF']
    t = nt.nodes.new('ShaderNodeTexImage'); t.image = diff; nt.links.new(t.outputs['Color'], b.inputs['Base Color'])
    r = nt.nodes.new('ShaderNodeTexImage'); r.image = arm
    sep = nt.nodes.new('ShaderNodeSeparateColor'); nt.links.new(r.outputs['Color'], sep.inputs['Color'])
    nt.links.new(sep.outputs['Green'], b.inputs['Roughness']); nt.links.new(sep.outputs['Blue'], b.inputs['Metallic'])
    n = nt.nodes.new('ShaderNodeTexImage'); n.image = nor
    nm = nt.nodes.new('ShaderNodeNormalMap'); nm.inputs['Strength'].default_value = 1.0
    nt.links.new(n.outputs['Color'], nm.inputs['Color']); nt.links.new(nm.outputs['Normal'], b.inputs['Normal'])
    return m
def mat(name, col, rough, metal=0.0, emit=None):
    m = bpy.data.materials.new(name); m.use_nodes = True
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*col, 1); b.inputs['Roughness'].default_value = rough; b.inputs['Metallic'].default_value = metal
    if emit: b.inputs['Emission Color'].default_value = (*emit[0], 1); b.inputs['Emission Strength'].default_value = emit[1]
    return m
# casco: tinta escura fosca com riscos e pontos de ferrugem; placas: chapa pintada com emendas, um tom mais clara
HULL = texmat('mech_hull', tinted('green_metal_rust_diff.jpg', 'hull_diff', 0.22, (0.50, 0.52, 0.56)), plain_tex('green_metal_rust_nor.jpg'), plain_tex('green_metal_rust_arm.jpg'))
PLATE = texmat('mech_plate', tinted('blue_metal_plate_diff.jpg', 'plate_diff', 0.12, (1.15, 1.16, 1.20)), plain_tex('blue_metal_plate_nor.jpg'), plain_tex('blue_metal_plate_arm.jpg'))
STEEL = mat('mech_steel', (0.085, 0.088, 0.096), 0.42, 0.55)
ROD = mat('mech_rod', (0.50, 0.51, 0.53), 0.22, 0.9)
GUN = mat('mech_gun', (0.014, 0.015, 0.018), 0.48, 0.35)
RUBBER = mat('mech_rubber', (0.012, 0.012, 0.013), 0.9, 0.0)
MARK = mat('mech_mark', (0.62, 0.62, 0.58), 0.75, 0.0)
VISOR = mat('mech_visor', (0.8, 0.08, 0.02), 0.3, 0.0, ((1.0, 0.10, 0.02), 4.0))
LAMP = mat('mech_lamp', (0.30, 0.26, 0.16), 0.25, 0.0, ((1.0, 0.80, 0.45), 0.5))
LENS = mat('mech_lens', (0.01, 0.02, 0.03), 0.06, 0.4)

# ------------------------------------------------------------------ juntas e formas
WORLD, REST = {}, {}
def joint(name, parent, pos):
    o = bpy.data.objects.new(name, None); sc.collection.objects.link(o)
    o.rotation_mode = 'XYZ'; o.empty_display_size = 0.05
    WORLD[name] = Vector(pos)
    if parent: o.parent = parent; o.location = Vector(pos) - WORLD[parent.name]
    else: o.location = Vector(pos)
    REST[name] = o.location.copy()
    return o
_seed = [0]
def _proj_uv(tmp, scale=1.0, cyl_axis=None, radius=0.0):
    """UV por projeção: plana pelo eixo dominante de cada face; nas laterais de um cilindro, desenrolada em volta do eixo Z."""
    uv = tmp.loops.layers.uv.verify()
    _seed[0] += 1
    off = Vector(((_seed[0] * 0.37) % 1.0, (_seed[0] * 0.61) % 1.0))
    tmp.normal_update()
    for f in tmp.faces:
        n = f.normal
        if cyl_axis and abs(n.z) < 0.6:
            c = f.calc_center_median(); ac = atan2(c.y, c.x)
            for l in f.loops:
                an = atan2(l.vert.co.y, l.vert.co.x)
                if an - ac > pi: an -= 2 * pi
                elif an - ac < -pi: an += 2 * pi
                l[uv].uv = Vector((an * radius, l.vert.co.z)) * scale + off
            continue
        ax = max(range(3), key=lambda k: abs(n[k]))
        i, j = [(1, 2), (0, 2), (0, 1)][ax]
        for l in f.loops: l[uv].uv = Vector((l.vert.co[i], l.vert.co[j])) * scale + off
def _emit(bm, tmp, c, rx=0.0, rz=0.0, ry=0.0):
    R = Matrix.Rotation(radians(rz), 3, 'Z') @ Matrix.Rotation(radians(ry), 3, 'Y') @ Matrix.Rotation(radians(rx), 3, 'X')
    for v in tmp.verts: v.co = Vector(c) + R @ v.co
    me = bpy.data.meshes.new('tmp'); tmp.to_mesh(me); tmp.free()
    bm.from_mesh(me); bpy.data.meshes.remove(me)
def box(bm, c, s, rx=0.0, rz=0.0, ry=0.0, bev=None, tf=None, tt=None, tb=None):
    """Caixa chanfrada. tf = afila a face da frente (-Y): (escala em x, escala em z, deslocamento em z);
    tt = afila o topo: (escala em x, escala em y); tb = afila a face de trás (+Y): (escala em x, escala em z, deslocamento em z)."""
    tmp = bmesh.new()
    vs = bmesh.ops.create_cube(tmp, size=1.0)['verts']
    for v in vs: v.co = Vector((v.co.x * s[0], v.co.y * s[1], v.co.z * s[2]))
    for v in vs:
        if tf and v.co.y < 0: v.co.x *= tf[0]; v.co.z = v.co.z * tf[1] + (tf[2] if len(tf) > 2 else 0.0)
        if tb and v.co.y > 0: v.co.x *= tb[0]; v.co.z = v.co.z * tb[1] + (tb[2] if len(tb) > 2 else 0.0)
        if tt and v.co.z > 0: v.co.x *= tt[0]; v.co.y *= tt[1]
    w = bev if bev is not None else min(0.016, 0.16 * min(s))
    if w > 0: bmesh.ops.bevel(tmp, geom=list(tmp.edges), offset=w, segments=2, affect='EDGES', profile=0.65, clamp_overlap=True)
    _proj_uv(tmp)
    _emit(bm, tmp, c, rx, rz, ry)
def cyl(bm, c, r, depth, axis='Y', r2=None, segs=24, rx=0.0, rz=0.0, bev=None):
    tmp = bmesh.new()
    bmesh.ops.create_cone(tmp, cap_ends=True, cap_tris=False, segments=segs, radius1=r, radius2=r if r2 is None else r2, depth=depth)
    w = bev if bev is not None else min(0.008, 0.2 * r)
    if w > 0:
        rim = [e for e in tmp.edges if len(e.link_faces) == 2 and any(len(f.verts) > 4 for f in e.link_faces)]
        if rim: bmesh.ops.bevel(tmp, geom=rim, offset=w, segments=2, affect='EDGES', profile=0.6, clamp_overlap=True)
    _proj_uv(tmp, cyl_axis=True, radius=r)
    base = {'Z': (0, 0), 'Y': (90, 0), 'X': (0, 90)}[axis]
    R0 = Matrix.Rotation(radians(base[1]), 3, 'Y') @ Matrix.Rotation(radians(base[0]), 3, 'X')
    for v in tmp.verts: v.co = R0 @ v.co
    _emit(bm, tmp, c, rx, rz)
def ball(bm, c, r, sx=1.0, sy=1.0, sz=1.0):
    tmp = bmesh.new()
    bmesh.ops.create_uvsphere(tmp, u_segments=20, v_segments=12, radius=r)
    for v in tmp.verts: v.co = Vector((v.co.x * sx, v.co.y * sy, v.co.z * sz))
    _proj_uv(tmp)
    _emit(bm, tmp, c)
def tube(bm, pts, r, segs=10):
    """Mangueira: tubo ao longo de uma linha de pontos (com as pontas fechadas)."""
    tmp = bmesh.new(); rings = []
    P = [Vector(p) for p in pts]
    for i, p in enumerate(P):
        t = (P[min(i + 1, len(P) - 1)] - P[max(i - 1, 0)]).normalized()
        u = t.cross(Vector((0, 0, 1)));
        if u.length < 1e-3: u = t.cross(Vector((1, 0, 0)))
        u.normalize(); w = t.cross(u).normalized()
        rings.append([tmp.verts.new(p + (u * cos(2 * pi * k / segs) + w * sin(2 * pi * k / segs)) * r) for k in range(segs)])
    for i in range(len(rings) - 1):
        for k in range(segs): tmp.faces.new((rings[i][k], rings[i][(k + 1) % segs], rings[i + 1][(k + 1) % segs], rings[i + 1][k]))
    tmp.faces.new(rings[0][::-1]); tmp.faces.new(rings[-1])
    _proj_uv(tmp)
    _emit(bm, tmp, (0, 0, 0))
def bolts(bm, pts, r=0.011, h=0.012, axis='X'):
    for p in pts: cyl(bm, p, r, h, axis, segs=8, bev=0.002)
def text(bm, body, c, size, read=(1, 0, 0), up=(0, 1, 0)):
    """Texto pintado: letras planas, um fio de cabelo acima da superfície. `read` = para onde as letras correm; `up` = o alto delas."""
    cu = bpy.data.curves.new('txt', 'FONT'); cu.body = body; cu.size = size; cu.align_x = 'CENTER'; cu.align_y = 'CENTER'
    ob = bpy.data.objects.new('txt', cu); sc.collection.objects.link(ob); bpy.context.view_layer.update()
    me = bpy.data.meshes.new_from_object(ob.evaluated_get(bpy.context.evaluated_depsgraph_get()))
    tmp = bmesh.new(); tmp.from_mesh(me); bpy.data.meshes.remove(me); bpy.data.objects.remove(ob); bpy.data.curves.remove(cu)
    tmp.loops.layers.uv.verify()
    r, u = Vector(read).normalized(), Vector(up).normalized()
    R = Matrix((r, u, r.cross(u))).transposed()
    for v in tmp.verts: v.co = Vector(c) + R @ v.co
    me2 = bpy.data.meshes.new('tmp'); tmp.to_mesh(me2); tmp.free(); bm.from_mesh(me2); bpy.data.meshes.remove(me2)
def pit(c, ly, lz, deg=None):
    """Ponto na superfície de uma peça inclinada: desloca (ly para trás, lz para cima) no referencial inclinado da peça."""
    R = Matrix.Rotation(radians(PITCH if deg is None else deg), 3, 'X')
    return tuple(Vector(c) + R @ Vector((0, ly, lz)))
def part(name, j, m, build, smooth=38.0):
    bm = bmesh.new(); bm.loops.layers.uv.verify(); build(bm)
    for v in bm.verts: v.co -= WORLD[j.name]
    # faces com mais de quatro lados viram triângulos (o espaço tangente do mapa de normais só existe para triângulos e quadriláteros)
    ng = [f for f in bm.faces if len(f.verts) > 4]
    if ng: bmesh.ops.triangulate(bm, faces=ng)
    me = bpy.data.meshes.new(name); bm.to_mesh(me); bm.free(); me.materials.append(m)
    for p in me.polygons: p.use_smooth = True
    me.set_sharp_from_angle(angle=radians(smooth))
    o = bpy.data.objects.new(name, me); sc.collection.objects.link(o); o.parent = j
    return o

PITCH = 10.0   # o corpo fica inclinado para a frente (focinho mais baixo que a traseira)
root = joint('Mech', None, (0, 0, 0))
hips = joint('Hips', root, (0, 0.05, 1.18))
torso = joint('Torso', hips, (0, 0.05, 1.24))
head = joint('Head', torso, (0, -0.58, 1.43))
cannon = joint('Cannon', torso, (-0.56, 0.10, 1.76))
legs = {}
for side, sx in (('L', 1.0), ('R', -1.0)):
    th = joint('Thigh' + side, hips, (sx * 0.31, 0.05, 1.15))     # quadril
    sh = joint('Shin' + side, th, (sx * 0.31, -0.28, 0.78))       # joelho (à frente do quadril)
    ft = joint('Foot' + side, sh, (sx * 0.31, 0.16, 0.40))        # tornozelo (atrás do joelho: é o "joelho invertido")
    legs[side] = (th, sh, ft, sx)
JOINTS = [root, hips, torso, head, cannon] + [o for s in legs.values() for o in s[:3]]

# ------------------------------------------------------------------ quadril
part('pelvis', hips, HULL, lambda bm: (box(bm, (0, 0.05, 1.16), (0.44, 0.34, 0.19), tf=(0.8, 0.8), tb=(0.8, 0.8)), box(bm, (0, 0.05, 1.05), (0.20, 0.24, 0.10), tt=(1.4, 1.2))))
def b_pelvis_steel(bm):
    cyl(bm, (0, 0.05, 1.15), 0.085, 0.80, 'X')                         # eixo das pernas
    cyl(bm, (0, 0.05, 1.255), 0.18, 0.05, 'Z', segs=32)                # anel de giro do corpo
    cyl(bm, (0, 0.05, 1.225), 0.21, 0.02, 'Z', segs=32)
    bolts(bm, [(0.195 * cos(k * pi / 6), 0.05 + 0.195 * sin(k * pi / 6), 1.24) for k in range(12)], 0.008, 0.012, 'Z')
part('pelvis_steel', hips, STEEL, b_pelvis_steel)

# ------------------------------------------------------------------ corpo
def b_hull(bm):
    box(bm, (0, -0.08, 1.50), (0.66, 1.12, 0.40), rx=PITCH, bev=0.03, tf=(0.80, 0.78, -0.02), tt=(0.90, 1.0))   # casco afilado para a frente
    box(bm, (0, 0.60, 1.66), (0.60, 0.42, 0.44), rx=PITCH, bev=0.035, tb=(0.82, 0.74, 0.02), tt=(0.92, 0.96))   # bloco do motor
    box(bm, (0, -0.02, 1.285), (0.40, 0.76, 0.10), rx=PITCH, bev=0.02, tf=(0.8, 1.0), tb=(0.8, 1.0))            # barriga
    box(bm, (0, 0.98, 1.56), (0.17, 0.42, 0.15), rx=-16, tb=(0.7, 0.7))                                         # cauda de contrapeso
    box(bm, (-0.45, 0.10, 1.70), (0.17, 0.32, 0.24), bev=0.025, tt=(0.8, 0.85))                                 # pilone do canhão
    box(bm, (0.39, 0.10, 1.675), (0.13, 0.24, 0.15), bev=0.02)                                                  # braço do casulo
part('hull', torso, HULL, b_hull)
def b_hull_plates(bm):
    box(bm, (0, 0.04, 1.748), (0.48, 0.74, 0.06), rx=PITCH, bev=0.012, tf=(0.82, 1.0))                          # convés
    for sx in (1.0, -1.0):
        box(bm, (sx * 0.362, -0.02, 1.515), (0.045, 0.84, 0.30), rx=PITCH, bev=0.012, tf=(1.0, 0.7), tb=(1.0, 0.8))   # saias laterais
        box(bm, (sx * 0.305, -0.555, 1.50), (0.11, 0.26, 0.28), rx=PITCH, rz=sx * -28, bev=0.014, tf=(1.0, 0.75))     # ombreiras junto ao pescoço
        box(bm, (sx * 0.30, 0.66, 1.70), (0.04, 0.34, 0.34), rx=PITCH, bev=0.01, tb=(1.0, 0.7))                 # chapas do motor
    for k in range(3):                                                                                          # espinhas do dorso
        box(bm, (0, -0.22 + 0.24 * k, 1.81 + 0.045 * k), (0.03, 0.17, 0.11), rx=PITCH - 14, bev=0.008, tt=(1.0, 0.55))
    box(bm, (0, 1.20, 1.50), (0.25, 0.10, 0.21), rx=-16, bev=0.014)                                             # massa da cauda
    cyl(bm, (0.0, -0.20, 1.80), 0.115, 0.03, 'Z', segs=28, rx=PITCH)                                            # tampa da escotilha
part('hull_plates', torso, PLATE, b_hull_plates)
def b_engine(bm):
    for k in range(7): box(bm, (0, 0.47 + 0.045 * k, 1.898 + 0.008 * k), (0.48, 0.012, 0.075), rx=PITCH, bev=0.003)   # aletas do radiador
    for sx in (1.0, -1.0):
        cyl(bm, (sx * 0.20, 0.84, 1.84), 0.032, 0.22, 'Z', rx=-38)                                              # escapamentos curtos, inclinados para trás
        cyl(bm, (sx * 0.20, 0.905, 1.925), 0.038, 0.04, 'Z', rx=-38)
        cyl(bm, (sx * 0.31, 0.60, 1.44), 0.012, 0.02, 'X', segs=10)
    cyl(bm, (0, 0.60, 1.44), 0.125, 0.60, 'X', segs=28)                                                         # tanque sob o motor
    for dx in (-0.2, 0.0, 0.2): cyl(bm, (dx, 0.60, 1.44), 0.131, 0.025, 'X', segs=28, bev=0.003)                # cintas do tanque
    cyl(bm, (0.0, -0.20, 1.822), 0.03, 0.03, 'Z', segs=12, rx=PITCH)                                            # trava da escotilha
    for sx in (1.0, -1.0):                                                                                      # lançadores de fumaça
        for k in range(3): cyl(bm, (sx * (0.20 + 0.055 * k), 0.36, 1.86), 0.022, 0.11, 'Y', rx=-28 + PITCH, segs=12)
    box(bm, (0, -0.60, 1.335), (0.10, 0.04, 0.05), bev=0.008)                                                   # gancho de reboque
    box(bm, (0, 1.265, 1.47), (0.10, 0.04, 0.05), rx=-16, bev=0.008)
    # parafusos das saias
    for sx in (1.0, -1.0): bolts(bm, [pit((sx * 0.388, -0.02, 1.515), -0.33 + 0.22 * k, 0.105) for k in range(4)] + [pit((sx * 0.388, -0.02, 1.515), -0.20 + 0.18 * k, -0.105) for k in range(4)], 0.010, 0.012, 'X')
part('engine', torso, STEEL, b_engine)
def b_hoses(bm):
    for sx in (1.0, -1.0):
        tube(bm, [(sx * 0.24, 0.42, 1.40), (sx * 0.30, 0.30, 1.33), (sx * 0.33, 0.16, 1.30), (sx * 0.30, 0.06, 1.28)], 0.016)   # do motor ao quadril
        tube(bm, [(sx * 0.14, 0.44, 1.92), (sx * 0.16, 0.34, 1.93), (sx * 0.14, 0.22, 1.86)], 0.012)
    tube(bm, [(-0.34, 0.30, 1.80), (-0.42, 0.34, 1.86), (-0.50, 0.30, 1.86)], 0.018)                            # cabo do canhão
part('hoses', torso, RUBBER, b_hoses)
def b_lamps(bm):
    for sx in (1.0, -1.0): cyl(bm, (sx * 0.265, -0.588, 1.629), 0.019, 0.008, 'Y', rx=PITCH, segs=16, bev=0.001)
part('lamps', torso, LAMP, b_lamps)
part('lamp_housings', torso, STEEL, lambda bm: [cyl(bm, (sx * 0.265, -0.55, 1.635), 0.034, 0.075, 'Y', rx=PITCH, segs=16) for sx in (1.0, -1.0)])
# casulo de mísseis à esquerda (o lado esquerdo dele é +X)
part('pod', torso, PLATE, lambda bm: (box(bm, (0.50, 0.06, 1.79), (0.28, 0.46, 0.25), bev=0.022), box(bm, (0.50, 0.30, 1.79), (0.24, 0.04, 0.21), bev=0.01)))
def b_tubes(bm):
    for i in range(3):
        for j in range(2):
            cyl(bm, (0.42 + 0.08 * i, -0.165, 1.735 + 0.11 * j), 0.031, 0.03, 'Y', segs=16, bev=0.002)
    box(bm, (0.50, -0.172, 1.79), (0.275, 0.012, 0.245), bev=0.004)
part('pod_tubes', torso, GUN, b_tubes)
part('pod_rim', torso, STEEL, lambda bm: [cyl(bm, (0.42 + 0.08 * i, -0.176, 1.735 + 0.11 * j), 0.036, 0.012, 'Y', segs=16, bev=0.002) for i in range(3) for j in range(2)])
def b_marks(bm):
    for sx in (1.0, -1.0):
        # visto de fora, o lado esquerdo (+X) lê-se de frente para trás (+Y) e o direito, de trás para a frente
        rd = Vector(pit((0, 0, 0), 1.0, 0.0)) * sx
        text(bm, 'W-09', pit((sx * 0.3866, -0.02, 1.515), 0.10, 0.0), 0.115, read=rd, up=pit((0, 0, 0), 0.0, 1.0))
        box(bm, pit((sx * 0.3862, -0.02, 1.515), -0.27, 0.085), (0.004, 0.16, 0.02), rx=PITCH, bev=0)
    text(bm, '09', (0.50, 0.06, 1.9165), 0.13)
part('marks', torso, MARK, b_marks, smooth=1.0)

# ------------------------------------------------------------------ focinho
def b_head(bm):
    box(bm, (0, -0.84, 1.37), (0.40, 0.56, 0.25), rx=PITCH + 4, bev=0.03, tf=(0.66, 0.60, -0.02), tt=(0.84, 1.0))
    box(bm, (0, -1.13, 1.295), (0.25, 0.20, 0.14), rx=PITCH + 4, bev=0.025, tf=(0.70, 0.62, -0.01))
    cyl(bm, (0, -0.62, 1.31), 0.115, 0.20, 'Y', segs=24)                                                         # pescoço
    box(bm, (0, -0.86, 1.215), (0.19, 0.18, 0.07), bev=0.014)                                                    # berço das metralhadoras
part('head_snout', head, HULL, b_head)
def b_head_plates(bm):
    box(bm, (0, -0.79, 1.517), (0.30, 0.42, 0.035), rx=PITCH + 4, bev=0.01, tf=(0.7, 1.0))
    for sx in (1.0, -1.0): box(bm, (sx * 0.197, -0.84, 1.365), (0.03, 0.42, 0.16), rx=PITCH + 4, rz=sx * 4.5, bev=0.008, tf=(1.0, 0.55))
    cyl(bm, (0, -0.62, 1.31), 0.125, 0.04, 'Y', segs=24)
part('head_plates', head, PLATE, b_head_plates)
# visor: fenda estreita na frente do focinho
part('head_visor', head, VISOR, lambda bm: box(bm, (0, -1.088, 1.386), (0.185, 0.03, 0.014), rx=PITCH + 4, bev=0.003, tf=(0.85, 1.0)))
def b_head_guns(bm):
    for dx in (0.055, -0.055):
        cyl(bm, (dx, -1.04, 1.205), 0.017, 0.38, 'Y', segs=12)
        cyl(bm, (dx, -1.215, 1.205), 0.024, 0.05, 'Y', segs=12)
        cyl(bm, (dx, -0.93, 1.205), 0.027, 0.12, 'Y', segs=12)
part('head_guns', head, GUN, b_head_guns)
def b_head_steel(bm):
    cyl(bm, (0.16, -0.66, 1.72), 0.006, 0.36, 'Z', segs=8)                                                       # antena
    cyl(bm, (0.16, -0.66, 1.545), 0.016, 0.03, 'Z', segs=10)
    for dx in (0.10, -0.10): cyl(bm, (dx, -1.215, 1.305), 0.024, 0.03, 'Y', rx=PITCH + 4, segs=14)               # sensores do focinho
    bolts(bm, [(sx * 0.214, -0.70 - 0.11 * k, 1.42 - 0.03 * k) for sx in (1.0, -1.0) for k in range(3)], 0.008, 0.012, 'X')
part('head_steel', head, STEEL, b_head_steel)
part('head_lenses', head, LENS, lambda bm: [cyl(bm, (dx, -1.232, 1.3015), 0.018, 0.006, 'Y', rx=PITCH + 4, segs=14, bev=0.001) for dx in (0.10, -0.10)])

# ------------------------------------------------------------------ canhão (à direita: lado -X)
def b_cannon(bm):
    box(bm, (-0.56, 0.22, 1.76), (0.20, 0.52, 0.22), bev=0.028, tb=(0.8, 0.8))                                   # culatra
    cyl(bm, (-0.56, -0.50, 1.76), 0.046, 0.98, 'Y')                                                              # cano
    cyl(bm, (-0.56, -0.10, 1.76), 0.074, 0.30, 'Y')                                                              # luva de recuo
    cyl(bm, (-0.56, -0.30, 1.76), 0.058, 0.10, 'Y')
    cyl(bm, (-0.56, -1.045, 1.76), 0.068, 0.15, 'Y')                                                             # freio de boca
    for k in range(3): cyl(bm, (-0.56, -1.00 - 0.045 * k, 1.76), 0.076, 0.016, 'Y', bev=0.002)
    cyl(bm, (-0.70, 0.24, 1.70), 0.105, 0.13, 'X', segs=28)                                                      # tambor de munição
part('cannon_body', cannon, GUN, b_cannon)
def b_cannon_steel(bm):
    box(bm, (-0.56, -0.50, 1.838), (0.03, 0.88, 0.024), bev=0.006)                                               # trilho sobre o cano
    for y in (-0.92, -0.52, -0.10): box(bm, (-0.56, y, 1.81), (0.026, 0.03, 0.05), bev=0.005)
    cyl(bm, (-0.70, 0.24, 1.70), 0.03, 0.15, 'X', segs=12)
    bolts(bm, [(-0.458, 0.06 + 0.11 * k, 1.80) for k in range(4)] + [(-0.458, 0.06 + 0.11 * k, 1.72) for k in range(4)], 0.009, 0.012, 'X')
    cyl(bm, (-0.56, 0.495, 1.76), 0.06, 0.04, 'Y')                                                               # tampa da culatra
part('cannon_steel', cannon, STEEL, b_cannon_steel)
part('cannon_sight', cannon, PLATE, lambda bm: box(bm, (-0.56, 0.24, 1.90), (0.085, 0.19, 0.065), bev=0.012, tf=(0.85, 0.85)))
part('cannon_lens', cannon, LENS, lambda bm: cyl(bm, (-0.56, 0.142, 1.90), 0.024, 0.008, 'Y', segs=14, bev=0.001))
part('cannon_feed', cannon, RUBBER, lambda bm: tube(bm, [(-0.70, 0.24, 1.80), (-0.69, 0.30, 1.86), (-0.64, 0.34, 1.875), (-0.58, 0.34, 1.87)], 0.022))

# ------------------------------------------------------------------ pernas
for side, (th, sh, ft, sx) in legs.items():
    x = sx * 0.31
    # coxa: do quadril para a frente e para baixo
    part('thigh_' + side, th, HULL, lambda bm, x=x: box(bm, (x, -0.115, 0.965), (0.19, 0.25, 0.54), rx=-41.5, bev=0.03, tt=(0.85, 0.9)))
    def b_thigh_plates(bm, x=x, sx=sx):
        box(bm, (x + sx * 0.112, -0.125, 0.975), (0.04, 0.30, 0.46), rx=-41.5, bev=0.012, tt=(1.0, 0.7))
        box(bm, (x, -0.262, 1.02), (0.20, 0.05, 0.36), rx=-41.5, bev=0.012, tt=(0.8, 1.0))
    part('thigh_guard_' + side, th, PLATE, b_thigh_plates)
    def b_thigh_steel(bm, x=x, sx=sx):
        cyl(bm, (x + sx * 0.125, 0.05, 1.15), 0.12, 0.06, 'X', segs=28)                                          # tampa do quadril
        bolts(bm, [(x + sx * 0.158, 0.05 + 0.085 * cos(k * pi / 3), 1.15 + 0.085 * sin(k * pi / 3)) for k in range(6)], 0.010, 0.012, 'X')
        cyl(bm, (x - sx * 0.115, -0.02, 1.02), 0.034, 0.30, 'Z', rx=-41.5, segs=16)                              # corpo do pistão da coxa
    part('thigh_steel_' + side, th, STEEL, b_thigh_steel)
    part('thigh_rod_' + side, th, ROD, lambda bm, x=x, sx=sx: cyl(bm, (x - sx * 0.115, -0.17, 0.85), 0.017, 0.24, 'Z', rx=-41.5, segs=12))
    # canela: do joelho para trás e para baixo
    part('knee_' + side, sh, STEEL, lambda bm, x=x: (cyl(bm, (x, -0.28, 0.78), 0.092, 0.25, 'X', segs=28), cyl(bm, (x, 0.02, 0.535), 0.03, 0.34, 'Z', rx=49, segs=16)))
    part('shin_' + side, sh, HULL, lambda bm, x=x: box(bm, (x, -0.06, 0.59), (0.14, 0.19, 0.60), rx=49, bev=0.028, tt=(0.9, 0.85)))
    part('shin_rod_' + side, sh, ROD, lambda bm, x=x, sx=sx: cyl(bm, (x - sx * 0.10, -0.02, 0.63), 0.016, 0.44, 'Z', rx=49, segs=12))
    part('knee_guard_' + side, sh, PLATE, lambda bm, x=x, sx=sx: (box(bm, (x, -0.365, 0.80), (0.20, 0.06, 0.24), rx=-12, bev=0.016, tt=(0.75, 1.0)), box(bm, (x + sx * 0.085, -0.06, 0.59), (0.03, 0.16, 0.46), rx=49, bev=0.008)))
    part('shin_hose_' + side, sh, RUBBER, lambda bm, x=x, sx=sx: tube(bm, [(x - sx * 0.07, -0.22, 0.80), (x - sx * 0.11, -0.10, 0.74), (x - sx * 0.10, 0.04, 0.60), (x - sx * 0.07, 0.12, 0.47)], 0.013))
    # pé: do tornozelo para a frente e para baixo, com três garras e um esporão atrás
    part('ankle_' + side, ft, STEEL, lambda bm, x=x: (cyl(bm, (x, 0.16, 0.40), 0.072, 0.21, 'X', segs=24), cyl(bm, (x, -0.20, 0.095), 0.045, 0.26, 'X', segs=16)))
    part('foot_' + side, ft, HULL, lambda bm, x=x: box(bm, (x, 0.02, 0.245), (0.14, 0.17, 0.44), rx=-42, bev=0.026, tt=(0.9, 0.85)))
    def b_toes(bm, x=x, sx=sx):
        box(bm, (x, -0.20, 0.052), (0.25, 0.22, 0.10), bev=0.02, tt=(0.85, 0.9))
        for dx, rz in ((0.0, 0.0), (0.115, -18.0), (-0.115, 18.0)): box(bm, (x + dx, -0.40, 0.042), (0.078, 0.27, 0.078), rx=-8, rz=rz, bev=0.014, tf=(0.5, 0.45, -0.012))
        box(bm, (x, 0.02, 0.052), (0.08, 0.25, 0.07), rx=14, bev=0.012, tb=(0.5, 0.5))                           # esporão
    part('toes_' + side, ft, PLATE, b_toes)
    part('foot_rod_' + side, ft, ROD, lambda bm, x=x, sx=sx: cyl(bm, (x + sx * 0.09, 0.03, 0.255), 0.014, 0.34, 'Z', rx=-42, segs=12))

# ------------------------------------------------------------------ poses e clipes
def sm(x): x = min(max(x, 0.0), 1.0); return x * x * (3 - 2 * x)
def lerp(a_, b_, t): return a_ + (b_ - a_) * t
def base(cannon_deg=-22.0):
    p = {o.name: [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)] for o in JOINTS}   # (rotação em graus, deslocamento em metros)
    p['Cannon'][0] = (cannon_deg, 0, 0)
    return p
def idle(f):
    t = (f - 1) / 60.0 * 2 * pi
    p = base(-16.0)
    p['Hips'][1] = (0, 0, 0.008 * sin(t))
    p['Torso'][0] = (1.2 * sin(t), 0, 3.0 * sin(t))
    p['Head'][0] = (2.0 * sin(t * 2), 0, 16.0 * sin(t + 0.9))
    p['Cannon'][0] = (-16.0 + 1.5 * sin(t), 0, 0)
    return p
def crouch(p, k):
    # agacha sobre as pernas de joelho invertido: coxa avança, canela recua, pé compensa
    p['Hips'][1] = (0, 0, -0.07 * k)
    for s in 'LR':
        p['Thigh' + s][0] = (-9 * k, 0, 0); p['Shin' + s][0] = (17 * k, 0, 0); p['Foot' + s][0] = (-8 * k, 0, 0)
def aim(f):
    t = (f - 1) / 30.0 * 2 * pi
    p = base(-2.0)                                            # canhão nivelado
    crouch(p, 1.0)
    p['Torso'][0] = (2.0 + 0.4 * sin(t), 0, 0)
    return p
def shoot(f):
    p = aim(1)
    k = sm((f - 1) / 2.0) if f <= 3 else 1.0 - sm((f - 3) / 12.0)       # coice rápido, volta lenta
    p['Cannon'][1] = (0, 0.20 * k, 0)
    p['Cannon'][0] = (-2.0 - 5.0 * k, 0, 0)
    p['Torso'][0] = (2.0 - 7.0 * k, 0, 0)
    p['Hips'][1] = (0, 0.05 * k, -0.07 - 0.02 * k)
    return p
# pernas: comprimentos e ângulos de repouso no plano (y, z); os ângulos crescem no sentido da rotação em torno de X
H0, K0, A0, T0 = (0.05, 1.15), (-0.28, 0.78), (0.16, 0.40), (-0.14, 0.07)
LA, LB, LC = ((K0[0] - H0[0]) ** 2 + (K0[1] - H0[1]) ** 2) ** 0.5, ((A0[0] - K0[0]) ** 2 + (A0[1] - K0[1]) ** 2) ** 0.5, ((T0[0] - A0[0]) ** 2 + (T0[1] - A0[1]) ** 2) ** 0.5
AT0, AS0, AF0 = atan2(K0[1] - H0[1], K0[0] - H0[0]), atan2(A0[1] - K0[1], A0[0] - K0[0]), atan2(T0[1] - A0[1], T0[0] - A0[0])
def _wrap(x): return (x + pi) % (2 * pi) - pi
def leg_ik(hip, toe, foot_ang):
    """Rotações (graus, em torno de X) de coxa, canela e pé para a ponta do pé ficar em `toe`, com o pé no ângulo dado."""
    from math import acos, degrees
    ay, az = toe[0] - LC * cos(foot_ang), toe[1] - LC * sin(foot_ang)        # tornozelo
    vy, vz = ay - hip[0], az - hip[1]
    d = min((vy * vy + vz * vz) ** 0.5, LA + LB - 0.005)
    base = atan2(vz, vy)
    g = acos(max(-1.0, min(1.0, (LA * LA + d * d - LB * LB) / (2 * LA * d))))
    cands = []
    for sg in (1.0, -1.0):
        at = base + sg * g
        cands.append((hip[0] + LA * cos(at), at))
    ky, at = min(cands)                                                      # joelho à frente (menor y)
    kz = hip[1] + LA * sin(at)
    ash = atan2(az - kz, ay - ky)
    return degrees(_wrap(at - AT0)), degrees(_wrap((ash - AS0) - (at - AT0))), degrees(_wrap((foot_ang - AF0) - (ash - AS0)))
STRIDE, DUTY, LIFT = 0.90, 0.60, 0.20      # passada para 1,5 m/s com um ciclo por segundo; cada pé fica 60% do ciclo no chão
def walk(f):
    ph = (f - 1) / 30.0
    t = ph * 2 * pi
    p = base(-8.0)
    # o corpo baixa quando os dois pés estão no chão, balança para o lado da perna de apoio e gira um pouco contra o passo
    dz, dx = -0.05 + 0.022 * cos(2 * t), 0.03 * sin(t)
    p['Hips'][1] = (dx, 0, dz)
    p['Hips'][0] = (0, -2.0 * sin(t), 0)
    p['Torso'][0] = (1.2 * sin(2 * t), 1.4 * sin(t), -3.0 * sin(t))
    p['Head'][0] = (-1.0 * sin(2 * t), 0, 2.5 * sin(t))
    p['Cannon'][0] = (-8.0 + 0.8 * sin(2 * t + 0.6), 0, 0)
    hip = (H0[0], H0[1] + dz)
    for s, off in (('L', 0.0), ('R', 0.5)):
        q = (ph + off) % 1.0
        if q < DUTY:                                           # apoio: o pé fica parado no chão enquanto o corpo passa por cima
            u = q / DUTY
            toe = (-0.10 - STRIDE / 2 + STRIDE * u, T0[1]); fa = AF0
        else:                                                  # balanço: o pé sobe, vem para a frente e pousa
            u = (q - DUTY) / (1 - DUTY)
            toe = (-0.10 + STRIDE / 2 - STRIDE * sm(u), T0[1] + LIFT * sin(pi * u)); fa = AF0 + radians(16.0 * sin(2 * pi * u))
        th, kn, ft = leg_ik(hip, toe, fa)
        p['Thigh' + s][0] = (th, 0, 0); p['Shin' + s][0] = (kn, 0, 0); p['Foot' + s][0] = (ft, 0, 0)
    return p
def fall(f):
    # as pernas dobram, a barriga bate no chão, o corpo tomba de lado e o focinho e o canhão arrastam
    keys = [  # (quadro, quadril dz, quadril dy, torso rx, torso ry, coxa, canela, pé, canhão, cabeça)
        (1, -0.07, 0.0, 2, 0, -9, 17, -8, -8, 0), (10, -0.16, 0.02, -7, 3, -18, 30, -10, -20, 6), (24, -0.48, -0.06, 8, 8, -34, 52, -6, 6, -8),
        (38, -0.74, -0.16, 15, 13, -44, 62, 4, 20, -14), (50, -0.80, -0.20, 17, 15, -46, 64, 8, 24, -16), (56, -0.78, -0.19, 16, 14, -45, 63, 7, 22, -15), (66, -0.80, -0.20, 17, 15, -46, 64, 8, 24, -16),
    ]
    k0, k1 = keys[0], keys[-1]
    for i in range(len(keys) - 1):
        if keys[i][0] <= f <= keys[i + 1][0]: k0, k1 = keys[i], keys[i + 1]; break
    t = sm((f - k0[0]) / max(k1[0] - k0[0], 1))
    v = [lerp(k0[j], k1[j], t) for j in range(1, 10)]
    p = base(v[7])
    p['Hips'][1] = (0, v[1], v[0])
    p['Torso'][0] = (v[2], v[3], 0)
    p['Head'][0] = (v[8], 0, 0)
    for n, s in enumerate('LR'):
        p['Thigh' + s][0] = (v[4] - 5 * n, 0, 5 * (1 - 2 * n)); p['Shin' + s][0] = (v[5], 0, 0); p['Foot' + s][0] = (v[6], 0, 0)
    return p
CLIPS = [('Idle', idle, 61), ('Aim', aim, 31), ('Shoot', shoot, 16), ('Walk', walk, 31), ('Fall1', fall, 66)]

def apply(p):
    for o in JOINTS:
        r, l = p[o.name]
        o.rotation_euler = [radians(x) for x in r]; o.location = REST[o.name] + Vector(l)
ACT = {}
for name, fn, n in CLIPS:
    for o in JOINTS:
        o.animation_data_create()
        act = bpy.data.actions.new('%s_%s' % (name, o.name)); o.animation_data.action = act; ACT[(name, o.name)] = act
    for f in range(1, n + 1):
        apply(fn(f))
        for o in JOINTS: o.keyframe_insert('rotation_euler', frame=f); o.keyframe_insert('location', frame=f)
    for o in JOINTS: o.animation_data.action = None

# ------------------------------------------------------------------ prévias (opcional)
if prev:
    os.makedirs(prev, exist_ok=True)
    cd = bpy.data.cameras.new('c'); cd.lens = 50
    cam = bpy.data.objects.new('c', cd); sc.collection.objects.link(cam); sc.camera = cam
    lights = []
    for i, (pos, e) in enumerate([((3, -4, 5), 3.2), ((-4, -2, 3), 1.6), ((0, 5, 4), 1.4)]):
        ld = bpy.data.lights.new('l%d' % i, 'SUN'); ld.energy = e
        lo = bpy.data.objects.new('l%d' % i, ld); sc.collection.objects.link(lo); lights.append(lo)
        lo.rotation_euler = (Vector((0, 0, 0)) - Vector(pos)).to_track_quat('-Z', 'Y').to_euler()
    w = bpy.data.worlds.new('w'); sc.world = w; w.use_nodes = True
    w.node_tree.nodes['Background'].inputs[0].default_value = (0.36, 0.38, 0.41, 1)
    sc.render.engine = 'BLENDER_EEVEE_NEXT'; sc.render.resolution_x = 900; sc.render.resolution_y = 900
    sc.render.image_settings.file_format = 'JPEG'; sc.render.image_settings.quality = 85; sc.view_settings.view_transform = 'Standard'
    shots = [('front', (2.9, -4.3, 1.9), 'Idle', 1), ('side', (5.2, -0.1, 1.4), 'Aim', 1), ('back', (-2.8, 4.4, 2.2), 'Idle', 20), ('walk', (5.4, 0.0, 1.2), 'Walk', 1), ('walk2', (5.4, 0.0, 1.2), 'Walk', 6), ('walk3', (5.4, 0.0, 1.2), 'Walk', 11), ('walk4', (5.4, 0.0, 1.2), 'Walk', 16),
             ('shoot', (-4.2, -3.0, 1.7), 'Shoot', 3), ('fall', (3.6, -3.8, 2.0), 'Fall1', 66), ('close', (1.9, -2.5, 1.9), 'Aim', 1), ('rear', (-1.8, 2.6, 2.0), 'Aim', 1), ('left', (2.6, 0.2, 1.6), 'Aim', 1), ('head', (0.9, -2.2, 1.5), 'Idle', 1)]
    by_name = {n_: fn_ for n_, fn_, _ in CLIPS}
    for tag, cp, clip, fr in shots:
        apply(by_name[clip](fr)); bpy.context.view_layer.update()
        cam.location = cp; cam.rotation_euler = (Vector((0, 0, 1.0)) - Vector(cp)).to_track_quat('-Z', 'Y').to_euler()
        sc.render.filepath = os.path.join(prev, 'mech_%s.jpg' % tag); bpy.ops.render.render(write_still=True)
    for o in [cam] + lights: bpy.data.objects.remove(o)

# ------------------------------------------------------------------ exportação: uma faixa por clipe em cada junta
apply(idle(1))
for o in JOINTS:
    ad = o.animation_data
    for name, _, _ in CLIPS:
        tr = ad.nla_tracks.new(); tr.name = name; tr.strips.new(name, 1, ACT[(name, o.name)])
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_materials='EXPORT',
    export_cameras=False, export_lights=False, export_yup=True, export_tangents=True, export_image_format='JPEG', export_jpeg_quality=85)
print("MECH exportado em", out, "com", len(JOINTS), "juntas e", len(CLIPS), "clipes")
