"""FSB Operator (Mateusz Woliński / jeandiz, CC-BY) -> assets/models/operator.glb
Uniforme e equipamento em preto, insígnias russas trocadas por um emblema sem texto, malha e texturas reduzidas.
blender -b --python tools3d/conv_operator.py -- <fsb_operator.glb> <op_patch.png> <saida.glb>"""
import bpy, sys, os
import numpy as np
a = sys.argv[sys.argv.index('--') + 1:]
src, patch, out = a[0], a[1], a[2]
# quarto argumento: "hostage" = refém (sem arma, capacete e equipamento; roupa clara, a cor de "não atire"; mãos para o alto)
#                   "threat"  = ameaça (roupa cáqui; sem insígnia; pistola no lugar do fuzil; clipes Idle, Aim, Shoot e quedas);
#                               pede um quinto argumento: o arquivo da pistola (assets/models/pistola_colt.glb)
MODE = a[3] if len(a) > 3 else 'operator'
HOSTAGE, THREAT = MODE == 'hostage', MODE == 'threat'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for o in list(bpy.data.objects):
    if o.name.startswith('Icosphere'): bpy.data.objects.remove(o)
# o óculos de visão noturna sai de todas as versões (pedido do dono do projeto)
drop = ('Krinkov', 'Null.001', 'Magazine', 'pouch.002', 'helmet', 'night_vis_goggles', 'fsb_patch') if HOSTAGE else ('fsb_patch', 'night_vis_goggles') if THREAT else ('night_vis_goggles',)
for o in [o for o in bpy.data.objects if o.type == 'MESH']:
    if o.data.materials and o.data.materials[0] and o.data.materials[0].name in drop:
        bpy.data.objects.remove(o)

# ---------- poses feitas aqui (o modelo só traz um clipe: em pé, fuzil em pronto baixo) ----------
import re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pose_lib
from math import sqrt, radians
from mathutils import Vector, Matrix
arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
sc = bpy.context.scene
def bone(name): return next(pb for pb in arm.pose.bones if re.fullmatch(r'mixamorig:' + name + r'_\d+', pb.name))
def subtree(pb):
    yield pb
    for c in pb.children: yield from subtree(c)
def world(pb):
    bpy.context.view_layer.update()
    return arm.matrix_world @ pb.matrix
def set_world(pb, M):
    pb.matrix = arm.matrix_world.inverted() @ M
    bpy.context.view_layer.update()
def point(pb, d):
    # gira o osso em torno da própria cabeça até ele apontar para a direção d (em coordenadas do mundo)
    M = world(pb); cur = (M.to_3x3() @ Vector((0, 1, 0))).normalized()
    R = cur.rotation_difference(Vector(d).normalized()).to_matrix().to_4x4(); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ R @ Matrix.Translation(-h) @ M)
def twist(pb, deg):
    # gira o osso em torno do eixo vertical do mundo
    M = world(pb); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ Matrix.Rotation(radians(deg), 4, 'Z') @ Matrix.Translation(-h) @ M)
def drop_curves(action, bones):
    names = {pb.name for pb in bones}
    for fc in list(action.fcurves):
        if fc.data_path.startswith('pose.bones["') and fc.data_path.split('"')[1] in names: action.fcurves.remove(fc)
def key(bones, frames):
    for pb in bones:
        for f in frames:
            for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=f)
def two_bone(side, target, pole):
    # leva o punho ao alvo dobrando o cotovelo para o lado de `pole`; devolve onde o punho chegou
    up_b, fore_b, hand_b = bone(side + 'Arm'), bone(side + 'ForeArm'), bone(side + 'Hand')
    S = world(up_b).translation.copy(); E0 = world(fore_b).translation.copy(); W0 = world(hand_b).translation.copy()
    L1, L2 = (E0 - S).length, (W0 - E0).length
    v = target - S; d = min(v.length, (L1 + L2) * 0.995); u = v.normalized()
    a_ = (L1 * L1 - L2 * L2 + d * d) / (2 * d); h = sqrt(max(L1 * L1 - a_ * a_, 0.0))
    pp = (pole - pole.dot(u) * u).normalized()
    E = S + u * a_ + pp * h; W = S + u * d
    point(up_b, E - S); point(fore_b, W - world(fore_b).translation)
    print("BRACO", side, "alcance pedido %.3f de %.3f" % (v.length, L1 + L2))
    return world(hand_b).translation.copy()

sc.frame_set(1)
idle = arm.animation_data.action
if HOSTAGE:
    bpy.context.view_layer.update()
    legs = bpy.data.materials['uniform'].copy(); legs.name = 'uniform_legs'
    for o in [o for o in bpy.data.objects if o.type == 'MESH']:
        if o.data.materials and o.data.materials[0] and o.data.materials[0].name == 'uniform':
            zs = [(o.matrix_world @ Vector(c)).z for c in o.bound_box]
            if (min(zs) + max(zs)) / 2 < 0.93 and max(zs) < 1.05:
                if o.data.users > 1: o.data = o.data.copy()
                o.data.materials[0] = legs
LAST = int(idle.frame_range[1])
if HOSTAGE:
    # mãos para o alto: sem isto ele fica segurando um fuzil que não existe
    arms = [pb for s in ('Left', 'Right') for pb in subtree(bone(s + 'Shoulder'))]
    drop_curves(idle, arms)
    for s, sx in (('Right', -1.0), ('Left', 1.0)):
        point(bone(s + 'Arm'), (sx * 0.86, -0.14, 0.30))
        point(bone(s + 'ForeArm'), (sx * 0.10, -0.12, 1.0))
        point(bone(s + 'Hand'), (sx * 0.02, -0.06, 1.0))
    key(arms, (1, LAST))
if THREAT:
    import numpy as _np
    PISTOL = a[4]   # assets/models/pistola_colt.glb: a mesma pistola que o jogador usa em primeira pessoa
    def bone_parent(o, pb):
        # o objeto deixa de ter animação própria e passa a acompanhar o osso
        mw = o.matrix_world.copy(); o.animation_data_clear()
        o.parent = arm; o.parent_type = 'BONE'; o.parent_bone = pb.name
        bpy.context.view_layer.update(); o.matrix_world = mw; bpy.context.view_layer.update()
    def kill(o):
        for c in list(o.children): kill(c)
        bpy.data.objects.remove(o)
    def rotx(pb, deg):
        # gira o osso em torno do eixo lateral do mundo (negativo = ponta para cima)
        M = world(pb); h = M.translation.copy()
        set_world(pb, Matrix.Translation(h) @ Matrix.Rotation(radians(deg), 4, 'X') @ Matrix.Translation(-h) @ M)
    def pose_snapshot(): return {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in arm.pose.bones}
    def pose_restore(snap):
        for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = snap[pb.name]
        bpy.context.view_layer.update()
    def hands_to(right, left, pitch):
        # leva os dois punhos aos alvos (relativos a cada ombro) e inclina as mãos (e a pistola) em `pitch` graus
        for s, off, pole, d in (('Right', right, Vector((-0.8, 0.3, -1.0)), (0.06, -1.0, 0.0)), ('Left', left, Vector((0.8, 0.3, -1.0)), (-0.42, -1.0, 0.05))):
            S = world(bone(s + 'Arm')).translation.copy()
            two_bone(s, S + Vector(off), pole)
            point(bone(s + 'Hand'), d)
            if pitch: rotx(bone(s + 'Hand'), pitch)
    # sai o fuzil (a ameaça passa a usar a pistola); carregadores e bolsos do colete acompanham o tronco
    kill(bpy.data.objects['Gun_99'])
    for o in bpy.data.objects:
        if o.type == 'EMPTY' and (o.name.startswith('pouch') or o.name.startswith('Magazine.0')): bone_parent(o, bone('Spine1'))
    idle.name = 'Idle'
    aim = idle.copy(); aim.name = 'Aim'
    drop_curves(aim, list(arm.pose.bones))
    arm.animation_data.action = aim
    sc.frame_set(1); bpy.context.view_layer.update()
    # mira a duas mãos: braços estendidos à frente, mãos juntas na altura do queixo
    AIM_R, AIM_L = (0.155, -0.33, 0.085), (-0.165, -0.325, 0.07)
    hands_to(AIM_R, AIM_L, 0)
    # ---- pistola: só a arma (sem as mãos de primeira pessoa), na pose parada do arquivo, encaixada na mão direita
    objs0, acts0 = set(bpy.data.objects), set(bpy.data.actions)
    bpy.ops.import_scene.gltf(filepath=PISTOL)
    newo = [o for o in bpy.data.objects if o not in objs0]
    fp = next(o for o in newo if o.type == 'ARMATURE')
    still = [x for x in bpy.data.actions if x not in acts0 and x.name.startswith('Idle')]
    if still: fp.animation_data.action = still[0]
    sc.frame_set(1); bpy.context.view_layer.update()
    dg = bpy.context.evaluated_depsgraph_get()
    pistol = bpy.data.objects.new('Pistol', None); sc.collection.objects.link(pistol)
    pts = []
    for o in newo:
        if o.type == 'MESH' and o.data.materials and o.data.materials[0] and o.data.materials[0].name.startswith('Colt'):
            me = bpy.data.meshes.new_from_object(o.evaluated_get(dg))
            part = bpy.data.objects.new('Pistol_part', me); sc.collection.objects.link(part)
            part.matrix_world = o.matrix_world.copy(); part.parent = pistol
            pts += [tuple(o.matrix_world @ v.co) for v in me.vertices]
    for o in newo: bpy.data.objects.remove(o)
    for x in list(bpy.data.actions):
        if x not in acts0: bpy.data.actions.remove(x)
    bpy.context.view_layer.update()
    pts = _np.array(pts)
    hi = pts[pts[:, 2] > _np.median(pts[:, 2])]                 # metade de cima = ferrolho e cano
    d = _np.linalg.svd(hi - hi.mean(0), full_matrices=False)[2][0]
    if d[1] > 0: d = -d                                          # no arquivo a arma aponta para -Y
    up = _np.array([0.0, 0.0, 1.0]); up = up - up.dot(d) * d; up /= _np.linalg.norm(up)
    A = _np.array([d, up, _np.cross(d, up)]).T
    F, U = _np.array([0.0, -1.0, 0.0]), _np.array([0.0, 0.0, 1.0])
    Rw = Matrix((_np.array([F, U, _np.cross(F, U)]).T @ A.T).tolist()).to_4x4()
    h = pts @ up
    grip = Vector(pts[h < _np.quantile(h, 0.35)].mean(0).tolist())   # punho da pistola = parte de baixo
    length = float((pts @ d).max() - (pts @ d).min())
    SC = 0.21 / length                                           # comprimento real de uma .45: 21 cm
    wrist = world(bone('RightHand')).translation.copy()
    palm = wrist + Vector((0.012, -0.075, -0.012))
    pistol.matrix_world = Matrix.Translation(palm) @ Rw @ Matrix.Scale(SC, 4) @ Matrix.Translation(-grip)
    bpy.context.view_layer.update()
    bone_parent(pistol, bone('RightHand'))
    print("PISTOLA comprimento no arquivo %.2f, escala %.4f" % (length, SC))
    key(list(arm.pose.bones), (1, 12))
    aim_pose = pose_snapshot()
    # ---- disparo: coice curto (punhos sobem, cano levanta) e volta para a mira
    shoot = bpy.data.actions.new('Shoot'); arm.animation_data.action = shoot
    key(list(arm.pose.bones), (1,))
    hands_to((AIM_R[0], AIM_R[1] + 0.03, AIM_R[2] + 0.05), (AIM_L[0], AIM_L[1] + 0.03, AIM_L[2] + 0.05), -17)
    key(list(arm.pose.bones), (3,))
    pose_restore(aim_pose)
    hands_to((AIM_R[0], AIM_R[1] + 0.01, AIM_R[2] + 0.015), (AIM_L[0], AIM_L[1] + 0.01, AIM_L[2] + 0.015), -5)
    key(list(arm.pose.bones), (6,))
    pose_restore(aim_pose)
    key(list(arm.pose.bones), (10,))
    # ---- quedas, partindo da pose de mira (joelhos dobram e o corpo desaba, em vez de tombar rígido)
    MX = {'hips': 'Hips', 'spine0': 'Spine', 'spine1': 'Spine1', 'spine2': 'Spine2', 'neck': 'Neck', 'head': 'Head',
          'thigh_l': 'LeftUpLeg', 'thigh_r': 'RightUpLeg', 'calf_l': 'LeftLeg', 'calf_r': 'RightLeg', 'arm_l': 'LeftArm', 'arm_r': 'RightArm', 'fore_l': 'LeftForeArm', 'fore_r': 'RightForeArm'}
    def restore(): pose_restore(aim_pose)
    for v in pose_lib.VARIANTS:
        arm.animation_data.action = bpy.data.actions.new(v)
        pose_lib.make_fall(lambda k: bone(MX[k]), world, set_world, point, restore, lambda f: key(list(arm.pose.bones), (f,)), v)
    # ---- espera: o clipe original segurava um fuzil; os braços passam a ficar com a pistola baixa, a duas mãos
    arms = [pb for s in ('Left', 'Right') for pb in subtree(bone(s + 'Arm'))]
    drop_curves(idle, arms)
    arm.animation_data.action = idle
    sc.frame_set(1); bpy.context.view_layer.update()
    hands_to((0.14, -0.20, -0.30), (-0.15, -0.195, -0.31), 48)
    key(arms, (1, LAST))
    sc.frame_set(1)

# (piso, ganho) aplicados à luminância da textura: preto para o operador, claro para o refém
LIGHT = {'uniform': (0.62, 0.38), 'boots': (0.45, 0.4), 'gloves': (0.6, 0.35), 'mask': (0.6, 0.35)}
# refém em roupa civil comum (branco puro estourava na luz e parecia um fantasma): cor por peça, multiplicada pela textura clara
CIVIL = {'uniform': (0.42, 0.52, 0.66), 'uniform_legs': (0.13, 0.16, 0.24), 'mask': (0.34, 0.30, 0.25), 'gloves': (0.78, 0.60, 0.50), 'boots': (0.20, 0.15, 0.11)}
BLACK = {'uniform': (0.050, 0.17), 'helmet': (0.045, 0.16), 'pouch.002': (0.045, 0.15), 'boots': (0.035, 0.12), 'gloves': (0.04, 0.14), 'mask': (0.04, 0.12),
         # lentes claras do óculos de visão noturna pareciam dois olhos brancos vistas de frente
         'night_vis_goggles': (0.03, 0.2)}
if HOSTAGE: BLACK = LIGHT
# ameaça: cáqui, por canal (R, G, B) = piso + ganho x luminância; equipamento escuro para contrastar
TAN = {'uniform': ((0.36, 0.30, 0.20), (0.62, 0.52, 0.36)), 'helmet': ((0.20, 0.17, 0.12), (0.45, 0.4, 0.3)), 'pouch.002': ((0.22, 0.19, 0.13), (0.4, 0.35, 0.26)),
       'boots': ((0.16, 0.12, 0.08), (0.3, 0.24, 0.17)), 'gloves': ((0.12, 0.10, 0.08), (0.22, 0.2, 0.17)), 'mask': ((0.10, 0.09, 0.08), (0.16, 0.15, 0.14))}
if THREAT: BLACK = TAN
SIZE = {'uniform': 1024, 'helmet': 1024, 'face': 1024, 'Krinkov': 1024, 'fsb_patch': 1024}
def base_node(m):
    p = next((n for n in m.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None)
    if not p: return None, None
    def walk(sock, depth=0):
        for l in sock.links:
            n = l.from_node
            if n.type == 'TEX_IMAGE' and n.image: return n
            if depth < 4:
                for i in n.inputs:
                    r = walk(i, depth + 1)
                    if r: return r
        return None
    return p, walk(p.inputs['Base Color'])
for m in bpy.data.materials:
    if not m.use_nodes: continue
    p, bn = base_node(m)
    if not p: continue
    nt = m.node_tree
    # sem transmissão nem alfa: o jogo desenha tudo opaco
    for name in ('Transmission Weight', 'Alpha'):
        if name in p.inputs:
            for l in list(p.inputs[name].links): nt.links.remove(l)
    if 'Transmission Weight' in p.inputs: p.inputs['Transmission Weight'].default_value = 0.0
    p.inputs['Alpha'].default_value = 1.0
    m.blend_method = 'OPAQUE'
    if bn:
        for l in list(p.inputs['Base Color'].links): nt.links.remove(l)
        nt.links.new(bn.outputs[0], p.inputs['Base Color'])
    # olhos: a córnea é uma casca por cima da íris, transparente pelo canal alfa da textura; sem isso o olho fica branco
    if m.name == 'eyes' and bn:
        nt.links.new(bn.outputs['Alpha'], p.inputs['Alpha'])
        m.blend_method = 'BLEND'
    if m.name == 'fsb_patch' and bn:
        bn.image = bpy.data.images.load(patch)
        m.name = 'unit_patch'
        for n in list(nt.nodes):
            if n.type == 'TEX_IMAGE' and n.name != bn.name: nt.nodes.remove(n)
        for l in list(p.inputs['Normal'].links): nt.links.remove(l)
        p.inputs['Roughness'].default_value = 0.9; p.inputs['Metallic'].default_value = 0.0
        continue
    if m.name in BLACK and bn:
        lo, k = BLACK[m.name]
        img = bn.image
        w, h = img.size
        px = np.empty(w * h * 4, dtype=np.float32); img.pixels.foreach_get(px); px = px.reshape(-1, 4)
        lum = px[:, 0] * 0.2126 + px[:, 1] * 0.7152 + px[:, 2] * 0.0722
        if THREAT:
            for ch in range(3): px[:, ch] = np.minimum(lo[ch] + k[ch] * lum, 1.0)
        else:
            v = lo + k * lum
            tint = (1.0, 1.0, 1.0) if HOSTAGE else (0.96, 1.0, 1.07)
            for ch in range(3): px[:, ch] = np.minimum(v * tint[ch], 1.0)
        img.pixels.foreach_set(px.reshape(-1)); img.update()
        print("COR", m.name, img.size[:])
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image:
            s = SIZE.get(m.name, 512)
            if n.image.size[0] > s: n.image.scale(s, s * n.image.size[1] // n.image.size[0])

TARGET = {'Krinkov': 4200, 'Magazine': 700, 'pouch.002': 800, 'helmet': 4500, 'boots': 1800}
# a ameaça aparece várias vezes ao mesmo tempo no mapa: malha mais leve que a do operador do menu
if THREAT: TARGET = {'Colt': 1400, 'Magazine': 380, 'pouch.002': 420, 'helmet': 2400, 'boots': 900, 'eyes': 500, 'face': 1200}
total = 0
for o in [o for o in bpy.data.objects if o.type == 'MESH']:
    me = o.data
    tris = sum(len(p.vertices) - 2 for p in me.polygons)
    mat = me.materials[0].name if me.materials and me.materials[0] else ''
    t = TARGET.get(mat)
    if t and tris > t * 1.2:
        if me.users > 1: o.data = me.copy()
        d = o.modifiers.new('dec', 'DECIMATE'); d.ratio = t / tris
        with bpy.context.temp_override(object=o, active_object=o, selected_objects=[o]):
            while o.modifiers[0].name != 'dec': bpy.ops.object.modifier_move_up(modifier='dec')
            bpy.ops.object.modifier_apply(modifier='dec')
        tris = sum(len(p.vertices) - 2 for p in o.data.polygons)
    total += tris
print("TRIS", total)
sc = bpy.context.scene
ends = [a.frame_range[1] for a in bpy.data.actions]
sc.frame_start = 1; sc.frame_end = int(max(ends))
mode = 'ACTIVE_ACTIONS'
if THREAT:
    # dois clipes: cada objeto animado entra na faixa "Idle"; o esqueleto ganha também a faixa "Aim"
    mode = 'NLA_TRACKS'
    for o in bpy.data.objects:
        ad = o.animation_data
        if not ad or not ad.action: continue
        act = ad.action; ad.action = None
        for tr in list(ad.nla_tracks): ad.nla_tracks.remove(tr)
        tr = ad.nla_tracks.new(); tr.name = 'Idle'; tr.strips.new('Idle', 1, act)
        if o is arm or o.type == 'ARMATURE':
            tr = ad.nla_tracks.new(); tr.name = 'Aim'; tr.strips.new('Aim', 1, bpy.data.actions['Aim'])
            for v in ('Shoot',) + tuple(pose_lib.VARIANTS):
                tr = ad.nla_tracks.new(); tr.name = v; tr.strips.new(v, 1, bpy.data.actions[v])
else:
    for act in bpy.data.actions: act.name = 'Idle'
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode=mode, export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=82, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True)
# pós-exportação: a transparência dos olhos vira recorte (MASK), que o jogo desenha sem precisar ordenar; tecidos sem brilho
import json, struct
raw = open(out, 'rb').read()
ln = struct.unpack_from('<I', raw, 12)[0]
g = json.loads(raw[20:20 + ln]); rest = raw[20 + ln:]
for m in g['materials']:
    if m['name'] == 'eyes':
        m['alphaMode'] = 'MASK'; m['alphaCutoff'] = 0.5
    if HOSTAGE and m['name'] in CIVIL:
        m.setdefault('pbrMetallicRoughness', {})['baseColorFactor'] = list(CIVIL[m['name']]) + [1.0]
    if m['name'] in ('uniform', 'uniform_legs', 'pouch.002', 'gloves', 'mask', 'boots', 'unit_patch'):
        pbr = m.setdefault('pbrMetallicRoughness', {})
        pbr.pop('metallicRoughnessTexture', None); pbr['metallicFactor'] = 0.0; pbr['roughnessFactor'] = 0.95
        m.pop('extensions', None)
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest)
print("EXPORTADO", out, os.path.getsize(out))
