"""FSB Operator (Mateusz Woliński / jeandiz, CC-BY) -> assets/models/operator.glb
Uniforme e equipamento em preto, insígnias russas trocadas por um emblema sem texto, malha e texturas reduzidas.
blender -b --python tools3d/conv_operator.py -- <fsb_operator.glb> <op_patch.png> <saida.glb>"""
import bpy, sys, os
import numpy as np
a = sys.argv[sys.argv.index('--') + 1:]
src, patch, out = a[0], a[1], a[2]
# quarto argumento: "hostage" = refém (sem arma, capacete e equipamento; roupa clara, a cor de "não atire"; mãos para o alto)
#                   "threat"  = ameaça (roupa cáqui; sem insígnia; fica com o fuzil do modelo, as duas mãos ajustadas a ele;
#                               clipes Idle, Aim, Shoot em rajada, Walk e quedas);
#                               quinto argumento: o arquivo da pistola (assets/models/pistola_colt.glb), usado só no modo "player"
MODE = a[3] if len(a) > 3 else 'operator'
HOSTAGE, THREAT = MODE == 'hostage', MODE == 'threat'
# "player" = o operador do jogador na vista de cima: preto como o do menu, com a pistola e as poses da ameaça
ARMED = MODE in ('threat', 'player')
# operador principal (menu e corpo do jogador): jaqueta cáqui, colete e equipamento pretos, óculos de proteção no capacete
# (visual alternativo, desligado: o dono do projeto quer o operador todo de preto; ANGLES_TAN=1 liga para experimentar)
MAIN = MODE in ('operator', 'player') and os.environ.get('ANGLES_TAN') == '1'
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for o in list(bpy.data.objects):
    if o.name.startswith('Icosphere'): bpy.data.objects.remove(o)
# o óculos de visão noturna sai de todas as versões (pedido do dono do projeto)
drop = ('Krinkov', 'Null.001', 'Magazine', 'pouch.002', 'helmet', 'night_vis_goggles', 'fsb_patch') if HOSTAGE else ('fsb_patch', 'night_vis_goggles') if THREAT else ('night_vis_goggles',)
for o in [o for o in bpy.data.objects if o.type == 'MESH']:
    if o.data.materials and o.data.materials[0] and o.data.materials[0].name in drop:
        bpy.data.objects.remove(o)

# ---------- olhos: o original tem a pálpebra quase fechada e a íris avermelhada (cara de sonolento) ----------
def open_eyes(upper, lower, gaze=0.0, radius=0.024):
    """Ajusta as pálpebras e a direção do olhar. No original a pálpebra de cima cobre meia íris (cara de sonolento).
    upper > 0 levanta a pálpebra de cima; lower > 0 abaixa a de baixo (negativo = sobe: olhar semicerrado, firme);
    gaze = graus que os olhos giram para o lado (negativo = para a direita dele).
    Mede tudo na pose de verdade (em pé, cabeça para a frente), onde "para cima" é o eixo Z do mundo, e traz o deslocamento
    de volta para as coordenadas da malha (que ficam num espaço próprio, deitado)."""
    import numpy as np_
    from math import sin, cos, radians
    def by_mat(name): return next((o for o in bpy.data.objects if o.type == 'MESH' and o.data.materials and o.data.materials[0] and o.data.materials[0].name == name), None)
    face, eyes = by_mat('face'), by_mat('eyes')
    if not face or not eyes: return
    bpy.context.scene.frame_set(1)
    dg = bpy.context.evaluated_depsgraph_get()
    def posed(o):
        e = o.evaluated_get(dg); me = e.to_mesh()
        w = np_.array([tuple(e.matrix_world @ v.co) for v in me.vertices]); e.to_mesh_clear(); return w
    def back_map(o, w):
        # a peça acompanha a cabeça como um bloco: mapa linear (malha -> mundo) por mínimos quadrados, invertido
        rest = np_.array([tuple(v.co) for v in o.data.vertices])
        A = np_.linalg.lstsq(np_.hstack([rest, np_.ones((len(rest), 1))]), w, rcond=None)[0][:3].T
        return np_.linalg.inv(A)
    fw, ew = posed(face), posed(eyes)
    fback, eback = back_map(face, fw), back_map(eyes, ew)
    mid = ew[:, 0].mean(); moved = 0
    th = radians(gaze); R = np_.array([[cos(th), -sin(th), 0.0], [sin(th), cos(th), 0.0], [0.0, 0.0, 1.0]])
    for sel in (ew[:, 0] < mid, ew[:, 0] >= mid):
        c = np_.array([ew[sel, 0].mean(), ew[sel, 1].min(), ew[sel, 2].mean()])   # centro do olho, na frente (ele olha para -Y)
        dx, dz = fw[:, 0] - c[0], fw[:, 2] - c[2]
        d = np_.sqrt(dx * dx + dz * dz)
        near = (d < radius) & (np_.abs(fw[:, 1] - c[1]) < 0.022)
        for i in np_.nonzero(near)[0]:
            k = (1 - d[i] / radius) ** 0.6
            lift = dz[i] * k * (upper if dz[i] > 0 else lower)
            face.data.vertices[i].co = face.data.vertices[i].co + V_(fback @ np_.array([0.0, 0.0, lift]))
            moved += 1
        if gaze:
            ball = ew[sel].mean(0)                                                 # centro do globo ocular
            for i in np_.nonzero(sel)[0]:
                w2 = ball + R @ (ew[i] - ball)
                eyes.data.vertices[i].co = eyes.data.vertices[i].co + V_(eback @ (w2 - ew[i]))
    print("OLHOS vértices de pálpebra movidos", moved, "olhar", gaze)
from mathutils import Vector as V_
# retrato do menu: olhar semicerrado e firme, voltado para a câmera (a cabeça fica virada por cima do ombro);
# nos bonecos do jogo: olho aberto, olhando para a frente
if MODE == 'operator': open_eyes(0.42, -0.30, gaze=-17)
else: open_eyes(0.52, 0.10)

# ---------- poses feitas aqui (o modelo só traz um clipe: em pé, fuzil em pronto baixo) ----------
import re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
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

sc.frame_set(1)
idle = arm.animation_data.action
LAST = int(idle.frame_range[1])
import bmesh
def mat(name, col, rough, metal=0.0):
    m = bpy.data.materials.new(name); m.use_nodes = True
    b = m.node_tree.nodes['Principled BSDF']; b.inputs['Base Color'].default_value = (*col, 1); b.inputs['Roughness'].default_value = rough; b.inputs['Metallic'].default_value = metal
    return m
def arc_band(name, cx, cy, r, z, height, depth, a0, a1, m, tilt=0.0, lens=False, r2=None, blend=(50.0, 95.0), shape=(3.2, 0.35)):
    """Faixa curva em volta da cabeça: de a0 a a1 graus (0 = frente, positivo = lado esquerdo dele), raio r, na altura z."""
    from math import sin, cos
    bm = bmesh.new(); n = 24; rings = []
    for i in range(n + 1):
        a_ = radians(a0 + (a1 - a0) * i / n)
        ox, oy = sin(a_), -cos(a_)
        ht = hb = height / 2
        rr = r
        if r2 is not None:
            # o raio cresce de r (na frente, rente ao rosto) até r2 (nos lados e atrás, por fora do capacete)
            side = abs(((a0 + (a1 - a0) * i / n) + 180.0) % 360.0 - 180.0)
            s_ = min(max((side - blend[0]) / (blend[1] - blend[0]), 0.0), 1.0)
            rr = r + (r2 - r) * (s_ * s_ * (3 - 2 * s_))
        if lens:
            # contorno de lente: cantos arredondados, borda de cima mais reta que a de baixo
            # shape = (expoente: quanto maior, mais quadrado; quanto da borda de cima fica reta)
            u = abs(2 * i / n - 1)
            f = max((1 - u ** shape[0]) ** (1 / shape[0]), 0.12)
            ht, hb = height / 2 * (shape[1] + (1 - shape[1]) * f), height / 2 * f
        rings.append([bm.verts.new((cx + ox * (rr + dr), cy + oy * (rr + dr), z + dz)) for dr, dz in ((0, -hb), (depth, -hb), (depth, ht), (0, ht))])
    for i in range(n):
        for k_ in range(4):
            bm.faces.new((rings[i][k_], rings[i + 1][k_], rings[i + 1][(k_ + 1) % 4], rings[i][(k_ + 1) % 4]))
    bm.faces.new(rings[0][::-1]); bm.faces.new(rings[-1])
    me = bpy.data.meshes.new(name); bm.to_mesh(me); bm.free()
    ob = bpy.data.objects.new(name, me); sc.collection.objects.link(ob); me.materials.append(m)
    if tilt:
        piv = Vector((cx, cy, z))
        ob.matrix_world = Matrix.Translation(piv) @ Matrix.Rotation(radians(tilt), 4, 'X') @ Matrix.Translation(-piv)
    return ob
def box(name, centre, size, m):
    """Caixa alinhada aos eixos (x, y, z), para hastes e detalhes."""
    bm = bmesh.new(); bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts: v.co = Vector((centre[0] + v.co.x * size[0], centre[1] + v.co.y * size[1], centre[2] + v.co.z * size[2]))
    me = bpy.data.meshes.new(name); bm.to_mesh(me); bm.free()
    ob = bpy.data.objects.new(name, me); sc.collection.objects.link(ob); me.materials.append(m)
    return ob
def on_head(parts):
    # prende as peças ao osso da cabeça, sem mudar onde elas estão
    bpy.context.view_layer.update()
    head_b = next(pb for pb in arm.pose.bones if re.fullmatch(r'mixamorig:Head_\d+', pb.name))
    for ob in parts:
        mw = ob.matrix_world.copy(); ob.parent = arm; ob.parent_type = 'BONE'; ob.parent_bone = head_b.name
        bpy.context.view_layer.update(); ob.matrix_world = mw
    bpy.context.view_layer.update()
def by_material(name):
    return next(o for o in bpy.data.objects if o.type == 'MESH' and o.data.materials and o.data.materials[0] and o.data.materials[0].name == name)
def posed_verts(o):
    dg = bpy.context.evaluated_depsgraph_get()
    e = o.evaluated_get(dg); me = e.to_mesh()
    w = [e.matrix_world @ v.co for v in me.vertices]; e.to_mesh_clear()
    return w

if MODE in ('operator', 'player', 'threat'):
    # óculos balísticos envolventes (referência do dono do projeto): duas lentes escuras e brilhantes que acompanham a curva
    # do rosto até as têmporas, barra fina por cima, sem aro embaixo, ponte estreita no nariz e hastes finas.
    # No operador do jogador e nas ameaças (os olhos do modelo nunca ficaram bons).
    ev = posed_verts(by_material('eyes'))
    ex = (min(v.x for v in ev) + max(v.x for v in ev)) / 2; ez = sum(v.z for v in ev) / len(ev); efront = min(v.y for v in ev)
    lens_m2 = mat('shades_lens', (0.010, 0.011, 0.013), 0.12, 0.25)   # escura, com brilho discreto (muito metálica virava espelho claro sob a luz do menu)
    frame_m2 = mat('shades_frame', (0.010, 0.010, 0.012), 0.45)
    RB = 0.090                                            # raio da curva: acompanha o rosto
    yf = efront - 0.012                                   # ponto mais à frente dos óculos, 12 mm à frente dos olhos
    yc, zc_ = yf + RB, ez + 0.001
    WR = (2.4, 0.78)                                      # lente em gota: borda de cima quase reta, de baixo arredondada
    END = 54.0                                            # até onde a lente dá a volta (graus a partir da frente)
    from math import sin as _sin, cos as _cos
    parts = []
    for s in (1.0, -1.0):
        b0, b1 = (3.6, END) if s > 0 else (-END, -3.6)
        parts.append(arc_band('shades_lens', ex, yc, RB, zc_ - 0.002, 0.036, 0.0022, b0, b1, lens_m2, lens=True, shape=WR))
        # haste fina: da ponta da lente para trás, por dentro do capacete
        hx = ex + s * RB * _sin(radians(END)); hyy = yc - RB * _cos(radians(END))
        parts.append(box('shades_arm', (hx + s * 0.0005, hyy + 0.047, zc_ + 0.011), (0.0030, 0.094, 0.0065), frame_m2))
    # barra de cima (segura as duas lentes) e ponte do nariz
    parts.append(arc_band('shades_rim', ex, yc, RB + 0.0004, zc_ + 0.0135, 0.0055, 0.0042, -END - 1.0, END + 1.0, frame_m2))
    parts.append(arc_band('shades_bridge', ex, yc, RB + 0.0002, zc_ + 0.006, 0.012, 0.0036, -3.8, 3.8, frame_m2))
    # leve inclinação para baixo, em torno do centro da frente
    piv = Vector((ex, yf, zc_))
    for ob in parts:
        ob.matrix_world = Matrix.Translation(piv) @ Matrix.Rotation(radians(-6), 4, 'X') @ Matrix.Translation(-piv) @ ob.matrix_world
    on_head(parts)
    print("OCULOS envolventes em z=%.3f" % ez)

if HOSTAGE or MAIN:
    # as peças de roupa usam todas o mesmo material; aqui a calça (e, no visual alternativo, a jaqueta) ganham material
    # próprio para poderem ter cor diferente do colete
    bpy.context.view_layer.update()
    legs = bpy.data.materials['uniform'].copy(); legs.name = 'uniform_legs'
    jacket = bpy.data.materials['uniform'].copy(); jacket.name = 'uniform_jacket'
    cloth = []
    for o in [o for o in bpy.data.objects if o.type == 'MESH']:
        if o.data.materials and o.data.materials[0] and o.data.materials[0].name == 'uniform':
            ws = [o.matrix_world @ Vector(c) for c in o.bound_box]
            cloth.append((o, min(w.z for w in ws), max(w.z for w in ws), max(w.x for w in ws) - min(w.x for w in ws)))
    widest = max((c for c in cloth if (c[1] + c[2]) / 2 > 1.0), key=lambda c: c[3])[0]   # a jaqueta é a peça que inclui as mangas
    for o, z0, z1, wx in cloth:
        if o.data.users > 1: o.data = o.data.copy()
        if (z0 + z1) / 2 < 0.93 and z1 < 1.05: o.data.materials[0] = legs
        elif MAIN and o is widest: o.data.materials[0] = jacket
if MAIN:
    # (visual alternativo) óculos de proteção apoiados na frente do capacete, presos à cabeça
    hv = posed_verts(by_material('helmet'))
    cx = (min(v.x for v in hv) + max(v.x for v in hv)) / 2; cy = (min(v.y for v in hv) + max(v.y for v in hv)) / 2
    top = max(v.z for v in hv); rad = (max(v.x for v in hv) - min(v.x for v in hv)) / 2
    frame_m, lens_m = mat('helmet_goggle_frame', (0.012, 0.012, 0.014), 0.7), mat('helmet_goggle_lens', (0.02, 0.03, 0.04), 0.12, 0.6)
    zg = top - 0.105; rg = rad * 1.0
    on_head([arc_band('goggle_strap', cx, cy, rg, zg, 0.024, 0.005, 56, 304, frame_m, tilt=-14),
             arc_band('goggle_frame', cx, cy, rg, zg, 0.058, 0.016, -56, 56, frame_m, tilt=-14),
             arc_band('goggle_lens', cx, cy, rg + 0.0155, zg, 0.040, 0.003, -48, 48, lens_m, tilt=-14)])
if MODE == 'operator':
    # retrato do menu: tronco de lado para a câmera e o rosto virado por cima do ombro direito, olhando para ela
    turn = [bone('Spine2'), bone('Neck'), bone('Head')]
    drop_curves(idle, turn)
    twist(bone('Spine2'), -14); twist(bone('Neck'), -30); twist(bone('Head'), -30)
    key(turn, (1, LAST))
if HOSTAGE:
    # mãos para o alto: sem isto ele fica segurando um fuzil que não existe
    arms = [pb for s in ('Left', 'Right') for pb in subtree(bone(s + 'Shoulder'))]
    drop_curves(idle, arms)
    for s, sx in (('Right', -1.0), ('Left', 1.0)):
        point(bone(s + 'Arm'), (sx * 0.86, -0.14, 0.30))
        point(bone(s + 'ForeArm'), (sx * 0.10, -0.12, 1.0))
        point(bone(s + 'Hand'), (sx * 0.02, -0.06, 1.0))
    key(arms, (1, LAST))
if ARMED:
    import numpy as _np
    GRIP_FWD, GRIP_OFF = 0.078, (0.008, 0.0, -0.02)   # punho da pistola: à frente do pulso (ao longo da mão) e ajuste fino no mundo
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
    # a ameaça fica com o fuzil do modelo, preso à mão direita (as duas mãos são ajustadas a ele quadro a quadro, mais abaixo);
    # o corpo do jogador troca o fuzil pela pistola. Carregadores e bolsos do colete acompanham o tronco.
    RIFLE = THREAT
    if RIFLE:
        sc.frame_set(1); bpy.context.view_layer.update()
        gun = bpy.data.objects['Gun_99']; gun.name = 'Rifle'
        G0 = gun.matrix_world.copy(); G0i = G0.inverted()
        HAND = {s_: bone(s_ + 'Hand') for s_ in ('Left', 'Right')}
        # como cada mão segura o fuzil na pose original do modelo (punho e orientação, em coordenadas da arma) e os dedos fechados
        HR0, HL0 = G0i @ world(HAND['Right']), G0i @ world(HAND['Left'])
        A0, B0 = world(HAND['Right']).translation.copy(), world(HAND['Left']).translation.copy()
        FINGERS = [pb for s_ in ('Left', 'Right') for pb in subtree(HAND[s_]) if pb is not HAND[s_]]
        FING0 = {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in FINGERS}
        # eixo do cano (da coronha para a boca) e o "para cima" da arma (oposto ao carregador), no mundo, na pose original
        dg_ = bpy.context.evaluated_depsgraph_get()
        gp, mp = [], []
        def _collect(o):
            if o.type == 'MESH':
                w_ = [tuple(o.matrix_world @ v.co) for v in o.data.vertices]
                gp.extend(w_)
                if 'Magazine' in o.name or (o.parent and 'Magazine' in o.parent.name): mp.extend(w_)
            for c in o.children: _collect(c)
        _collect(gun)
        gp = _np.array(gp); gc = gp.mean(0)
        D0 = _np.linalg.svd(gp - gc, full_matrices=False)[2][0]
        if D0.dot(_np.array(B0 - A0)) < 0: D0 = -D0
        U0 = gc - _np.array(mp).mean(0) if mp else _np.array([0.0, 0.0, 1.0])
        U0 = U0 - U0.dot(D0) * D0; U0 /= _np.linalg.norm(U0)
        D0, U0 = Vector(D0.tolist()), Vector(U0.tolist())
        along = (gp - gc) @ _np.array(D0)
        print("FUZIL comprimento %.2f m; boca a %.2f m e coronha a %.2f m do punho direito; eixo (%.2f %.2f %.2f) cima (%.2f %.2f %.2f)" % (
            along.max() - along.min(), along.max() - (_np.array(A0) - gc).dot(_np.array(D0)), (_np.array(A0) - gc).dot(_np.array(D0)) - along.min(), *D0, *U0))
        MUZZLE0 = Vector((gc + _np.array(D0) * along.max()).tolist())
        EJECT0 = Vector((gc + _np.array(D0) * (along.min() + 0.33 * (along.max() - along.min())) + _np.array(U0) * 0.03).tolist())
        bone_parent(gun, HAND['Right'])
        def gun_from_wrists(wR, wL, fwd=None):
            # onde a arma fica para os dois punhos caírem nos pontos em que o modelo a segura: a linha entre os punhos dá a
            # direção, o carregador fica para baixo; com `fwd`, o cano é levado exatamente para essa direção (mira)
            R = (B0 - A0).rotation_difference(wL - wR)
            e = (wL - wR).normalized()
            u1 = R @ U0; u1 = u1 - u1.dot(e) * e
            z = Vector((0, 0, 1)); z = z - z.dot(e) * e
            if u1.length > 1e-5 and z.length > 1e-5:
                ang = u1.angle(z)
                if u1.cross(z).dot(e) < 0: ang = -ang
                from mathutils import Quaternion as _Q
                R = _Q(e, ang) @ R
            if fwd is not None:
                R = (R @ D0).rotation_difference(Vector(fwd).normalized()) @ R
                # depois de apontar, tira de novo a inclinação lateral
                d = (R @ D0).normalized(); u1 = R @ U0; z = Vector((0, 0, 1)); z = z - z.dot(d) * d
                if z.length > 1e-5:
                    ang = u1.angle(z)
                    if u1.cross(z).dot(d) < 0: ang = -ang
                    from mathutils import Quaternion as _Q
                    R = _Q(d, ang) @ R
            return Matrix.Translation(wR) @ R.to_matrix().to_4x4() @ Matrix.Translation(-A0) @ G0
        def turn_about(pb, q):
            M = world(pb); h = M.translation.copy()
            set_world(pb, Matrix.Translation(h) @ q.to_matrix().to_4x4() @ Matrix.Translation(-h) @ M)
        def fix_hands(G):
            # mão direita na empunhadura; braço esquerdo dobrado até o punho chegar ao guarda-mão; dedos fechados como no modelo
            M = G @ HR0; M.translation = world(HAND['Right']).translation; set_world(HAND['Right'], M)
            ua, fa, hb = bone('LeftArm'), bone('LeftForeArm'), HAND['Left']
            S, E0, W0 = world(ua).translation.copy(), world(fa).translation.copy(), world(hb).translation.copy()
            W = (G @ HL0).translation
            L1, L2 = (E0 - S).length, (W0 - E0).length
            v = W - S; d = min(max(v.length, abs(L1 - L2) + 1e-4), (L1 + L2) * 0.999); u = v.normalized()
            a_ = (L1 * L1 - L2 * L2 + d * d) / (2 * d); h = max(L1 * L1 - a_ * a_, 0.0) ** 0.5
            pole = (E0 - S) - (E0 - S).dot(u) * u
            if pole.length < 1e-5: pole = Vector((0, 0, -1)) - Vector((0, 0, -1)).dot(u) * u
            E = S + u * a_ + pole.normalized() * h; W = S + u * d
            turn_about(ua, (E0 - S).rotation_difference(E - S))
            E1, W1 = world(fa).translation.copy(), world(hb).translation.copy()
            turn_about(fa, (W1 - E1).rotation_difference(W - E1))
            M = G @ HL0; M.translation = world(hb).translation; set_world(hb, M)
            for pb in FINGERS: pb.location, pb.rotation_quaternion, pb.scale = FING0[pb.name]
            bpy.context.view_layer.update()
        GRIPPED = [HAND['Right'], bone('LeftArm'), bone('LeftForeArm'), HAND['Left']] + FINGERS
        def gun_now(): return world(HAND['Right']) @ HR0.inverted()
        def grip_clip(act, fwd=None):
            # passa o clipe inteiro acertando as mãos na arma (as mãos do Mixamo seguram um fuzil de outro tamanho)
            arm.animation_data.action = act
            last = {}
            for f in range(1, int(act.frame_range[1]) + 1):
                sc.frame_set(f); bpy.context.view_layer.update()
                fix_hands(gun_from_wrists(world(HAND['Right']).translation.copy(), world(HAND['Left']).translation.copy(), fwd))
                for pb in GRIPPED:
                    q = pb.rotation_quaternion
                    if pb.name in last and last[pb.name].dot(q) < 0: pb.rotation_quaternion = -q
                    last[pb.name] = pb.rotation_quaternion.copy()
                key(GRIPPED, (f,))
            sc.frame_set(1); bpy.context.view_layer.update()
            d = (gun_now().to_3x3() @ (G0i.to_3x3() @ D0)).normalized()
            print("EMPUNHADURA %s: cano aponta para (%.2f, %.2f, %.2f)" % (act.name, d.x, d.y, d.z))
    else:
        kill(bpy.data.objects['Gun_99'])
    for o in bpy.data.objects:
        if o.type == 'EMPTY' and (o.name.startswith('pouch') or o.name.startswith('Magazine.0')): bone_parent(o, bone('Spine1'))
        # insígnias (só existem no operador do jogador): a das costas acompanha o tronco, a do ombro acompanha o braço
        if o.type == 'EMPTY' and o.name.startswith('Plane.001'): bone_parent(o, bone('RightArm'))
        elif o.type == 'EMPTY' and o.name.startswith('Plane'): bone_parent(o, bone('Spine2'))
    # ---- clipes do Mixamo (espera, mira, caminhada e quedas) aplicados a este esqueleto; o clipe original (fuzil) sai
    import mixamo_lib
    MXDIR = mixamo_lib.folder(a[5] if len(a) > 5 else None)
    tmap = {}
    for pb in arm.pose.bones:
        m_ = re.fullmatch(r'mixamorig:(\w+?)_\d+', pb.name)
        if m_: tmap[m_.group(1)] = pb
    sc.frame_set(1)
    HIP0 = world(bone('Hips')).translation.copy()   # neste esqueleto a altura do quadril só existe na pose (o repouso deixa o quadril no chão)
    arm.animation_data.action = None
    bpy.data.actions.remove(idle)
    def mx(name, file, **kw):
        act, n = mixamo_lib.retarget(os.path.join(MXDIR, file + '.fbx'), arm, tmap, name, hip_base=HIP0, **kw)
        print("MIXAMO %s: %d quadros" % (name, n)); return act
    aim = mx('Aim', 'rifle_aim' if RIFLE else 'pistol_idle_aim')
    if RIFLE:
        grip_clip(aim, (0, -1, 0))
        # onde ficam a boca do cano e a janela de ejeção na mira (o jogo põe ali o clarão e solta a cápsula)
        for nm, p0 in (('BOCA', MUZZLE0), ('EJECAO', EJECT0)):
            w_ = gun_now() @ G0i @ p0
            print("%s na mira: %.3f à frente, %.3f para a esquerda dele, altura %.3f" % (nm, -w_.y, w_.x, w_.z))
    arm.animation_data.action = aim
    sc.frame_set(1); bpy.context.view_layer.update()
    if not RIFLE:
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
        Mh = world(bone('RightHand'))
        wrist = Mh.translation.copy(); hd = (Mh.to_3x3() @ Vector((0, 1, 0))).normalized()
        palm = wrist + hd * GRIP_FWD + Vector(GRIP_OFF)
        pistol.matrix_world = Matrix.Translation(palm) @ Rw @ Matrix.Scale(SC, 4) @ Matrix.Translation(-grip)
        bpy.context.view_layer.update()
        bone_parent(pistol, bone('RightHand'))
        print("PISTOLA comprimento no arquivo %.2f, escala %.4f; mão aponta para (%.2f, %.2f, %.2f)" % (length, SC, hd.x, hd.y, hd.z))
    aim_pose = pose_snapshot()
    # ---- disparo: coice curto (braços e mãos sobem, cano levanta) e volta para a mira; feito aqui, porque o clipe de tiro
    # do Mixamo gira o corpo de lado e o boneco do jogo precisa continuar de frente para o operador
    shoot = bpy.data.actions.new('Shoot'); arm.animation_data.action = shoot
    def kick(k):
        # k = quanto do coice (1 = pico; negativo = a arma passa um pouco do ponto ao voltar). O tronco e os ombros recuam,
        # os braços sobem, os punhos quebram para cima e a cabeça compensa para os olhos ficarem no alvo.
        pose_restore(aim_pose)
        if RIFLE:
            # fuzil apoiado no ombro: quem absorve é o tronco (vai para trás) e a boca sobe pouco; a mão esquerda segue a arma
            rotx(bone('Spine'), -0.8 * k); rotx(bone('Spine1'), -1.3 * k); rotx(bone('Spine2'), -1.9 * k)
            rotx(bone('Head'), 2.6 * k)
            for s_ in ('Left', 'Right'):
                rotx(bone(s_ + 'Shoulder'), -1.6 * k)
                rotx(bone(s_ + 'Arm'), -2.4 * k)
            fix_hands(gun_now())
            return
        rotx(bone('Spine1'), -1.5 * k); rotx(bone('Spine2'), -2.5 * k)
        rotx(bone('Head'), 3.0 * k)
        for s_ in ('Left', 'Right'):
            rotx(bone(s_ + 'Shoulder'), -2.0 * k)
            rotx(bone(s_ + 'Arm'), -7.0 * k)
            rotx(bone(s_ + 'ForeArm'), -6.0 * k)
            rotx(bone(s_ + 'Hand'), -19.0 * k)
    # pistola: um coice; fuzil: rajada curta de três tiros (um a cada três quadros) e volta para a mira
    KICKS = ((1, 0.0), (2, 1.0), (3, 0.5), (4, 0.3), (5, 1.05), (6, 0.55), (7, 0.35), (8, 1.1), (9, 0.6), (11, 0.22), (13, -0.08), (16, 0.0)) if RIFLE else \
            ((1, 0.0), (2, 0.72), (3, 1.0), (5, 0.38), (7, -0.13), (10, 0.04), (14, 0.0))
    for frame, k in KICKS:
        kick(k)
        key(list(arm.pose.bones), (frame,))
    # ---- espera com a pistola baixa e caminhada (o mesmo ciclo serve com a arma baixa e apontada)
    idle = mx('Idle', 'rifle_idle' if RIFLE else 'pistol_idle_low')
    walk = mx('Walk', 'rifle_walk' if RIFLE else 'pistol_walk', in_place=True)
    if RIFLE: grip_clip(idle); grip_clip(walk)
    walk_aim = walk.copy(); walk_aim.name = 'WalkAim'
    CLIPS = ['Aim', 'Shoot', 'Walk', 'WalkAim']
    if MODE == 'threat':
        # ---- quedas (só a ameaça cai): o corpo anda menos que no clipe original, para caber no cômodo
        for v, f in mixamo_lib.FALLS:
            act = mx(v, f, travel=0.6)
            arm.animation_data.action = act; sc.frame_set(int(act.frame_range[1]))
            hp, he = world(bone('Hips')).translation, world(bone('Head')).translation
            print("FIMQUEDA %s quadril (%.2f, %.2f, z %.2f) cabeça (%.2f, %.2f, z %.2f)" % (v, hp.x, hp.y, hp.z, he.x, he.y, he.z))
            CLIPS.append(v)
        # ---- clipes extras (andar para trás, passo lateral, corrida, proteger o rosto, tiro levado, arremesso), os mesmos
        # do outro boneco de ameaça (conv_invader.py): tools3d/extra_clips.py. Entram depois dos que já existiam.
        import extra_clips
        from types import SimpleNamespace
        CLIPS += extra_clips.build(SimpleNamespace(arm=arm, sc=sc, mxdir=MXDIR, tmap=tmap, mx=mx, world=world, set_world=set_world, key=key,
            snapshot=pose_snapshot, restore=pose_restore, grip_clip=grip_clip, gun_from_wrists=gun_from_wrists, fix_hands=fix_hands, gun_now=gun_now,
            gripped=GRIPPED, fingers=FINGERS, grip_dist=(B0 - A0).length, aim_pose=aim_pose, hip_height=HIP0.z))
    arm.animation_data.action = idle
    sc.frame_set(1)

# (piso, ganho) aplicados à luminância da textura: preto para o operador, claro para o refém
LIGHT = {'uniform': (0.62, 0.38), 'boots': (0.45, 0.4), 'gloves': (0.6, 0.35), 'mask': (0.6, 0.35)}
# refém em roupa civil comum (branco puro estourava na luz e parecia um fantasma): cor por peça, multiplicada pela textura clara
CIVIL = {'uniform': (0.42, 0.52, 0.66), 'uniform_legs': (0.13, 0.16, 0.24), 'mask': (0.34, 0.30, 0.25), 'gloves': (0.78, 0.60, 0.50), 'boots': (0.20, 0.15, 0.11)}
BLACK = {'uniform': (0.050, 0.17), 'helmet': (0.045, 0.16), 'pouch.002': (0.045, 0.15), 'boots': (0.035, 0.12), 'gloves': (0.04, 0.14), 'mask': (0.04, 0.12),
         # lentes claras do óculos de visão noturna pareciam dois olhos brancos vistas de frente
         'night_vis_goggles': (0.03, 0.2)}
if HOSTAGE: BLACK = LIGHT
# operador principal: o tecido fica claro e neutro na textura; a cor de cada peça (jaqueta, colete, calça) entra como fator do material
MAINCOL = {'uniform': (0.075, 0.08, 0.09), 'uniform_jacket': (0.64, 0.52, 0.29), 'uniform_legs': (0.19, 0.19, 0.16)}
if MAIN: BLACK = dict(BLACK, uniform=(0.52, 0.48))
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
    if m.name == 'eyes' and bn:
        # íris avermelhada -> castanha; branco do olho um pouco mais claro
        img = bn.image; w, h = img.size
        px = np.empty(w * h * 4, dtype=np.float32); img.pixels.foreach_get(px); px = px.reshape(-1, 4)
        mx, mn = px[:, :3].max(1), px[:, :3].min(1)
        lum = px[:, 0] * 0.2126 + px[:, 1] * 0.7152 + px[:, 2] * 0.0722
        col = (mx - mn) > 0.12
        for ch, kk in enumerate((0.95, 0.64, 0.38)): px[col, ch] = np.minimum((0.10 + lum[col] * 1.5) * kk, 1.0)
        for ch in range(3): px[~col, ch] = np.minimum(px[~col, ch] * 1.12 + 0.03, 1.0)
        img.pixels.foreach_set(px.reshape(-1)); img.update()
        print("IRIS", int(col.sum()), "de", len(col))
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
            tint = (1.0, 1.0, 1.0) if HOSTAGE or (MAIN and m.name == 'uniform') else (0.96, 1.0, 1.07)
            for ch in range(3): px[:, ch] = np.minimum(v * tint[ch], 1.0)
        img.pixels.foreach_set(px.reshape(-1)); img.update()
        print("COR", m.name, img.size[:])
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image:
            s = SIZE.get(m.name, 512)
            if n.image.size[0] > s: n.image.scale(s, s * n.image.size[1] // n.image.size[0])

TARGET = {'Krinkov': 4200, 'Magazine': 700, 'pouch.002': 800, 'helmet': 4500, 'boots': 1800}
# a ameaça aparece várias vezes ao mesmo tempo no mapa: malha mais leve que a do operador do menu
if ARMED: TARGET = {'Colt': 1400, 'Magazine': 380, 'pouch.002': 420, 'helmet': 2400, 'boots': 900}
# o fuzil da ameaça aparece de perto na mira: fica com a mesma malha do operador do menu (reduzir mais desmancha o cano e o carregador)
if THREAT: TARGET.update({'Krinkov': 4200, 'Magazine': 700})
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
if ARMED:
    # cada objeto animado entra na faixa "Idle"; o esqueleto ganha também uma faixa por clipe
    mode = 'NLA_TRACKS'
    for o in bpy.data.objects:
        ad = o.animation_data
        if not ad or not ad.action: continue
        act = ad.action; ad.action = None
        for tr in list(ad.nla_tracks): ad.nla_tracks.remove(tr)
        tr = ad.nla_tracks.new(); tr.name = 'Idle'; tr.strips.new('Idle', 1, act)
        if o is arm or o.type == 'ARMATURE':
            tr = ad.nla_tracks.new(); tr.name = 'Aim'; tr.strips.new('Aim', 1, bpy.data.actions['Aim'])
            for v in CLIPS[1:]:
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
        # olho menos espelhado: com o brilho original a luz do estúdio lavava a íris e o olho ficava cinza
        m.setdefault('pbrMetallicRoughness', {})['roughnessFactor'] = 0.38; m.pop('extensions', None)
    if HOSTAGE and m['name'] in CIVIL:
        m.setdefault('pbrMetallicRoughness', {})['baseColorFactor'] = list(CIVIL[m['name']]) + [1.0]
    if MAIN and m['name'] in MAINCOL:
        m.setdefault('pbrMetallicRoughness', {})['baseColorFactor'] = list(MAINCOL[m['name']]) + [1.0]
    if m['name'] in ('uniform', 'uniform_legs', 'uniform_jacket', 'pouch.002', 'gloves', 'mask', 'boots', 'unit_patch'):
        pbr = m.setdefault('pbrMetallicRoughness', {})
        pbr.pop('metallicRoughnessTexture', None); pbr['metallicFactor'] = 0.0; pbr['roughnessFactor'] = 0.95
        m.pop('extensions', None)
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest)
print("EXPORTADO", out, os.path.getsize(out))
