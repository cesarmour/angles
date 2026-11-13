"""Ameaça ("invasor") a partir de um soldado do Sketchfab com esqueleto genérico -> threat.glb
Feito para o "Forest Soldier (outdated)" de buh (CC-BY): boneco em pose A, 59 ossos chamados "Bone.001" etc., sem arma e
sem animação. Sai um arquivo com as mesmas convenções do assets/models/threat.glb feito pelo conv_operator.py (modo
"threat"), para o jogo usar sem mudar o código:

  - clipes "Idle", "Aim", "Shoot" (rajada de três), "Walk", "WalkAim" e "Fall1" a "Fall4" (Mixamo, pelo mixamo_lib),
    e depois deles os clipes extras de tools3d/extra_clips.py ("WalkBack", "SneakL", "SneakR", "Run", "Flinch", "Hit", "Toss");
  - ossos com os nomes do Mixamo ("mixamorig:LeftArm" etc.): o jogo procura LeftArm, LeftForeArm, LeftHand e os três
    do lado direito para a pose de rendição (src/operator.rs, hands_setup) e conta com o eixo Y do osso ao longo dele;
  - nó "Rifle" preso à mão direita (o jogo esconde esse nó quando a ameaça se rende);
  - de frente para +Z no arquivo (-Y no Blender), pés na origem, 1 unidade = 1 m, mesma altura do boneco antigo.

  blender -b --python tools3d/conv_invader.py -- <soldado.glb> <threat.glb de referência> <saida.glb> [pasta dos FBX do Mixamo]

O soldado não traz arma: o fuzil vem do arquivo de referência (qualquer threat.glb feito por este script ou pelo
conv_operator.py: precisa ter o nó "Rifle" e o clipe "Aim"). Do mesmo arquivo saem também a altura do boneco e o jeito
de segurar a arma: onde ficam os dois punhos em relação ao fuzil, para onde cada mão está virada e como os dedos fecham.

O que muda em relação ao original: ossos renomeados; osso do quadril reposicionado e ossos de ponta do pé criados (só
para receber as animações); malha reduzida; texturas em 1024; fuzil e clipes acrescentados.
"""
import bpy, sys, os, re, json, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mixamo_lib
import numpy as np
from math import radians
from mathutils import Vector, Matrix, Quaternion
from mathutils.kdtree import KDTree

a = sys.argv[sys.argv.index('--') + 1:]
src, ref, out = a[0], a[1], a[2]
MXDIR = mixamo_lib.folder(a[3] if len(a) > 3 else None)

# altura do boneco em repouso (pose A, de pé) como fração da altura do boneco de referência na pose de espera: na espera
# com o fuzil os joelhos dobram um pouco e o boneco fica mais baixo que em repouso
REST_K = 1.016
TEX = 1024
# boca do cano na pose de mira, no espaço do boneco: é onde o jogo desenha o clarão do disparo (src/combat.rs, enemy_muzzle:
# 0,69 m à frente, 0,064 m à direita dele, 1,38 m do chão). O clipe de mira é ajustado para a boca cair exatamente aí.
AIM_MUZZLE = (-0.064, -0.69, 1.38)
# na mira do Mixamo a cabeça baixa para a alça de mira e, com este capacete, o rosto some; fica quase reta (0 = como no Mixamo, 1 = reta)
AIM_HEAD_LEVEL = 0.8
# ajuste de cada punho em relação ao lugar do punho do boneco de referência, em metros, com a arma na mira
# (x = para a esquerda dele, y = para trás, z = para cima). As mãos deste modelo são mais curtas e mais grossas (luvas):
# com o punho no mesmo lugar, a palma entrava no guarda-mão e a mão direita sumia dentro da empunhadura.
GRIP_OFF = {'Right': (-0.025, 0.0, -0.01), 'Left': (0.0, 0.0, -0.035)}
# triângulos no máximo, por malha do soldado (pelo nome); o resto fica como está
TRIS = (('S 1_1.001', 3500), ('S 1_1', 26000))
# direção de repouso alinhada com a do Mixamo só nos braços e nas pernas (o boneco repousa em A, o Mixamo em T).
# Tronco, pescoço, cabeça e ombros ficam como o modelo foi feito: alinhar o pescoço inclinado dele ao do Mixamo jogava a
# cabeça para trás, e alinhar a clavícula caída levantava os ombros.
mixamo_lib.CHAIN = {k: v for k, v in mixamo_lib.CHAIN.items() if re.search(r'Arm|Hand|Leg|Foot', k)}

sc = bpy.context.scene
def update(): bpy.context.view_layer.update()
def posed_points(objs):
    dg = bpy.context.evaluated_depsgraph_get(); pts = []
    for o in objs:
        e = o.evaluated_get(dg); me = e.to_mesh()
        co = np.empty(len(me.vertices) * 3, dtype=np.float64); me.vertices.foreach_get('co', co)
        pts.append((np.array(e.matrix_world) @ np.hstack([co.reshape(-1, 3), np.ones((len(me.vertices), 1))]).T).T[:, :3])
        e.to_mesh_clear()
    return np.vstack(pts)
def hand_frame(W, I, M, P):
    # eixos da mão, pelos nós dos dedos: Y = do punho para o nó do dedo médio; Z = perpendicular à palma
    y = (M - W).normalized(); n = (I - W).cross(P - W).normalized()
    x = y.cross(n).normalized(); z = x.cross(y)
    return Matrix((x, y, z)).transposed()
FINGER = ('Thumb', 'Index', 'Middle', 'Ring', 'Pinky')

# ====================== referência: fuzil, altura e empunhadura do threat.glb atual ======================
bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene
bpy.ops.import_scene.gltf(filepath=ref)
rarm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
gun = bpy.data.objects.get('Rifle')
if gun is None: raise SystemExit('O arquivo de referência precisa ter o nó "Rifle": %s' % ref)
def ref_clip(name):
    for o in bpy.data.objects:
        if o.animation_data:
            for tr in o.animation_data.nla_tracks: tr.mute = True
            o.animation_data.action = None
    act = next((tr.strips[0].action for tr in rarm.animation_data.nla_tracks if tr.name == name and tr.strips), None)
    if act is None: raise SystemExit('O arquivo de referência não tem o clipe "%s"' % name)
    rarm.animation_data.action = act
    sc.frame_set(1); update()
def rbone(name): return next(pb for pb in rarm.pose.bones if re.fullmatch(r'mixamorig:' + name + r'(_\d+)?', pb.name))
def rhas(name): return any(re.fullmatch(r'mixamorig:' + name + r'(_\d+)?', pb.name) for pb in rarm.pose.bones)
def rhead(name): return rarm.matrix_world @ rbone(name).head
ref_meshes = [o for o in bpy.data.objects if o.type == 'MESH' and not o.name.startswith('Icosphere')]   # a esfera é o desenho de osso que o importador cria
ref_clip('Idle')
p_ = posed_points(ref_meshes); REF_H = float(p_[:, 2].max() - p_[:, 2].min())
ref_clip('Aim')
G0 = gun.matrix_world.copy(); G0i = G0.inverted()
# se a referência já é um arquivo feito por este script, os punhos dela já estão no lugar ajustado
if rarm.name.startswith('Invader'): GRIP_OFF = {'Right': (0, 0, 0), 'Left': (0, 0, 0)}
A0, B0 = rhead('RightHand') + Vector(GRIP_OFF['Right']), rhead('LeftHand') + Vector(GRIP_OFF['Left'])   # punhos, com a arma na mira
S_REF, D_REF = {}, {}
for s in ('Left', 'Right'):
    S_REF[s] = hand_frame(rhead(s + 'Hand'), rhead(s + 'HandIndex1'), rhead(s + 'HandMiddle1'), rhead(s + 'HandPinky1'))
    for fg in FINGER:
        for i in (1, 2, 3):
            pb = rbone('%sHand%s%d' % (s, fg, i)); nxt = '%sHand%s%d' % (s, fg, i + 1)
            end = rhead(nxt) if rhas(nxt) else rarm.matrix_world @ pb.tail
            D_REF[s, fg, i] = (end - rhead('%sHand%s%d' % (s, fg, i))).normalized()
# eixo do cano (da coronha para a boca) e o "para cima" da arma (oposto ao carregador), como no conv_operator
gun_parts = [gun] + list(gun.children_recursive)
gp, mp = [], []
for o in gun_parts:
    if o.type != 'MESH': continue
    w_ = [tuple(o.matrix_world @ v.co) for v in o.data.vertices]; gp += w_
    if 'Magazine' in o.name or (o.parent and 'Magazine' in o.parent.name): mp += w_
gp = np.array(gp); gc = gp.mean(0)
D0 = np.linalg.svd(gp - gc, full_matrices=False)[2][0]
if D0.dot(np.array(B0 - A0)) < 0: D0 = -D0
U0 = gc - np.array(mp).mean(0) if mp else np.array([0.0, 0.0, 1.0])
U0 = U0 - U0.dot(D0) * D0; U0 /= np.linalg.norm(U0)
along = (gp - gc) @ D0
MUZZLE0 = Vector((gc + D0 * along.max()).tolist())
EJECT0 = Vector((gc + D0 * (along.min() + 0.33 * (along.max() - along.min())) + U0 * 0.03).tolist())
D0, U0 = Vector(D0.tolist()), Vector(U0.tolist())
print("REFERENCIA altura na espera %.3f m; fuzil %.2f m; punhos a %.3f m um do outro; boca na mira: %.3f à frente, %.3f para a esquerda dele, altura %.3f" % (
    REF_H, along.max() - along.min(), (B0 - A0).length, -MUZZLE0.y, MUZZLE0.x, MUZZLE0.z))
# fica só o fuzil, solto, no lugar em que estava na mira
gun.parent = None; gun.matrix_world = G0
for o in gun_parts: o.animation_data_clear()
for o in list(bpy.data.objects):
    if o not in gun_parts: bpy.data.objects.remove(o)
for act in list(bpy.data.actions): bpy.data.actions.remove(act)
for _ in range(3):
    for coll in (bpy.data.meshes, bpy.data.armatures, bpy.data.materials, bpy.data.images):
        for d in list(coll):
            if d.users == 0: coll.remove(d)
update()

# ====================== soldado: limpeza, escala e esqueleto ======================
objs0 = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=src)
new = [o for o in bpy.data.objects if o not in objs0]
arm = next(o for o in new if o.type == 'ARMATURE')
meshes = [o for o in new if o.type == 'MESH' and o.find_armature() is arm]
update()
mw = arm.matrix_world.copy(); arm.parent = None; arm.matrix_world = mw
for o in new:
    if o is not arm and o not in meshes: bpy.data.objects.remove(o)       # a esfera solta do Sketchfab e os nós vazios
for o in [arm] + meshes: o.animation_data_clear()
for pb in arm.pose.bones:
    pb.rotation_mode = 'QUATERNION'
    pb.location = (0, 0, 0); pb.rotation_quaternion = (1, 0, 0, 0); pb.scale = (1, 1, 1)
update()
p_ = posed_points(meshes); lo, hi = p_[:, 2].min(), p_[:, 2].max()
k = REF_H * REST_K / (hi - lo)
arm.matrix_world = Matrix.Translation((0, 0, -lo * k)) @ Matrix.Scale(k, 4) @ arm.matrix_world
update()
# a escala e a posição entram na malha e no esqueleto: o objeto do esqueleto fica na origem, no chão, sem giro nem escala
# (o mixamo_lib mede a altura do quadril a partir da origem do objeto)
bpy.ops.object.select_all(action='DESELECT')
for o in meshes:
    mw = o.matrix_world.copy(); o.parent = None; o.matrix_world = mw
update()
for o in [arm] + meshes: o.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
for o in meshes: o.parent = arm; o.matrix_parent_inverse = Matrix.Identity(4)
update()
p_ = posed_points(meshes)
assert abs(p_[:, 2].min()) < 1e-3 and abs(p_[:, 2].max() - REF_H * REST_K) < 1e-3, "a malha saiu do lugar ao aplicar a escala"
print("SOLDADO %.3f m no arquivo -> %.3f m em repouso (escala %.4f); malhas: %s" % (hi - lo, REF_H * REST_K, k, [o.name for o in meshes]))

# nomes do Mixamo, pelo número do osso no arquivo ("Bone", "Bone.001" ... "Bone.057"; o lado esquerdo dele é +X)
NAMES = {0: 'Hips', 1: 'Pelvis', 2: 'Spine', 3: 'Spine2', 4: 'Neck', 5: 'Head',
         7: 'LeftUpLeg', 8: 'LeftLeg', 9: 'LeftFoot', 10: 'RightUpLeg', 11: 'RightLeg', 12: 'RightFoot',
         6: 'LeftShoulder', 13: 'LeftArm', 14: 'LeftForeArm', 15: 'LeftHand', 35: 'RightShoulder', 36: 'RightArm', 37: 'RightForeArm', 38: 'RightHand'}
for s, base in (('Left', 16), ('Right', 39)):
    for i in range(3): NAMES[base + i] = '%sHandThumb%d' % (s, i + 1)
    for j, fg in enumerate(FINGER[1:]):
        NAMES[base + 3 + j] = '%sHand%s0' % (s, fg)                      # osso da palma (metacarpo)
        for i in range(3): NAMES[base + 7 + 4 * i + j] = '%sHand%s%d' % (s, fg, i + 1)
for b in arm.data.bones:
    m_ = re.match(r'Bone(?:\.(\d+))?_', b.name)
    n = NAMES.get(int(m_.group(1) or 0)) if m_ else 'Root'
    if n is None: raise SystemExit("Osso sem nome previsto: %r (o esqueleto não é o do Forest Soldier)" % b.name)
    b.name = 'mixamorig:' + n
B = lambda n: 'mixamorig:' + n
missing = [n for n in NAMES.values() if B(n) not in arm.data.bones]
if missing: raise SystemExit("Faltam ossos no esqueleto: %s" % missing)

def mixamo_rest():
    # no boneco do Mixamo em repouso: altura do quadril em relação à altura da junta da coxa, e a direção de cada pé
    # (do tornozelo para a ponta)
    o0 = set(bpy.data.objects); a0 = set(bpy.data.actions)
    d0 = {c: set(getattr(bpy.data, c)) for c in ('meshes', 'materials', 'armatures', 'images')}
    bpy.ops.import_scene.fbx(filepath=os.path.join(MXDIR, 'rifle_idle.fbx'))
    newo = [o for o in bpy.data.objects if o not in o0]
    s_ = next(o for o in newo if o.type == 'ARMATURE'); update()
    h_ = {b.name.split(':')[-1]: s_.matrix_world @ b.head_local for b in s_.data.bones}
    z = {n: v.z - s_.matrix_world.translation.z for n, v in h_.items()}
    feet = {s: (h_[s + 'ToeBase'] - h_[s + 'Foot']).normalized() for s in ('Left', 'Right')}
    for o in newo: bpy.data.objects.remove(o)
    for x in list(bpy.data.actions):
        if x not in a0: bpy.data.actions.remove(x)
    for c, before in d0.items():
        coll = getattr(bpy.data, c)
        for d in list(coll):
            if d not in before: coll.remove(d)
    return z['Hips'] / z['LeftUpLeg'], feet
HIP_RATIO, FOOT_DIR = mixamo_rest()

# ajustes no esqueleto (em repouso, sem mexer na malha):
#  - quadril: no arquivo o osso que carrega tronco e pernas nasce 34 cm atrás do corpo; girar em torno dali jogaria o
#    boneco para os lados. Vai para o meio das coxas, na altura em que o Mixamo põe o quadril (a animação desce e sobe o
#    quadril na proporção dessa altura; com a proporção das pernas, os pés ficam no chão).
#  - mão: aponta para o nó do dedo médio (o jogo usa o eixo do osso como direção da mão).
#  - ponta do pé: osso novo, sem malha, na mesma direção em que o Mixamo tem a ponta do pé. Com ele o pé fica como o
#    modelo foi feito (sola no chão) sempre que o pé do Mixamo está em repouso; sem ele o pé herdava o ajuste da canela.
W = arm.matrix_world.copy(); Wi = W.inverted()
bpy.ops.object.select_all(action='DESELECT')
bpy.context.view_layer.objects.active = arm; arm.select_set(True)
bpy.ops.object.mode_set(mode='EDIT')
eb = arm.data.edit_bones
for b in eb: b.use_connect = False
wl, wr = W @ eb[B('LeftUpLeg')].head, W @ eb[B('RightUpLeg')].head
hp = Vector(((wl.x + wr.x) / 2, (wl.y + wr.y) / 2, (wl.z + wr.z) / 2 * HIP_RATIO))
eb[B('Hips')].head = Wi @ hp; eb[B('Hips')].tail = Wi @ (hp + Vector((0, 0, 0.1)))
for s in ('Left', 'Right'):
    eb[B(s + 'Hand')].tail = eb[B(s + 'HandMiddle1')].head.copy()
    ft = eb[B(s + 'Foot')]; an = W @ ft.head
    toe = eb.new(B(s + 'ToeBase')); toe.parent = ft; toe.use_deform = False
    th = an + FOOT_DIR[s] * (an.z * 0.72 / max(-FOOT_DIR[s].z, 0.2))
    toe.head = Wi @ th; toe.tail = Wi @ (th + Vector((0, -0.06, 0)))
bpy.ops.object.mode_set(mode='OBJECT')
update()
print("QUADRIL a %.3f m do chão (coxas a %.3f; proporção do Mixamo %.3f)" % (hp.z, (wl.z + wr.z) / 2, HIP_RATIO))

# vértices sem osso (o arquivo tem alguns): na tela do jogo iriam parar na origem. Ganham os pesos do vizinho mais próximo.
bone_names = {b.name for b in arm.data.bones}
for o in meshes:
    gi = {g.index for g in o.vertex_groups if g.name in bone_names}
    ok, bad = [], []
    for v in o.data.vertices:
        (ok if sum(g.weight for g in v.groups if g.group in gi) > 1e-4 else bad).append(v.index)
    if bad and ok:
        kd = KDTree(len(ok))
        for j, i in enumerate(ok): kd.insert(o.data.vertices[i].co, j)
        kd.balance()
        for i in bad:
            _, j, _ = kd.find(o.data.vertices[i].co)
            for g in o.data.vertices[ok[j]].groups:
                if g.group in gi: o.vertex_groups[g.group].add([i], g.weight, 'REPLACE')
    elif bad:
        g = o.vertex_groups.get(B('Head')) or o.vertex_groups.new(name=B('Head'))
        g.add(bad, 1.0, 'REPLACE')
    print("PESOS %-12s %d vértices, %d sem osso (corrigidos)" % (o.name, len(o.data.vertices), len(bad)))

# ====================== pose: ferramentas ======================
def bone(name): return arm.pose.bones[B(name)]
def subtree(pb):
    yield pb
    for c in pb.children: yield from subtree(c)
def world(pb):
    update()
    return arm.matrix_world @ pb.matrix
def set_world(pb, M):
    pb.matrix = arm.matrix_world.inverted() @ M
    update()
def turn_about(pb, q):
    # gira o osso em torno da própria junta
    M = world(pb); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ q.to_matrix().to_4x4() @ Matrix.Translation(-h) @ M)
def set_rot(pb, R3):
    # põe o osso numa orientação do mundo, sem tirá-lo do lugar
    loc, _, scl = world(pb).decompose()
    set_world(pb, Matrix.LocRotScale(loc, R3.to_quaternion(), scl))
def rotx(pb, deg): turn_about(pb, Quaternion((1, 0, 0), radians(deg)))   # eixo lateral do mundo (negativo = ponta para cima)
def key(bones, frames):
    for pb in bones:
        for f in frames:
            for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=f)
def drop_curves(action, bones):
    names = {pb.name for pb in bones}
    for fc in list(action.fcurves):
        if fc.data_path.startswith('pose.bones["') and fc.data_path.split('"')[1] in names: action.fcurves.remove(fc)
def pose_snapshot(): return {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in arm.pose.bones}
def pose_restore(snap):
    for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = snap[pb.name]
    update()
REST = pose_snapshot()
def rest_head(n): return arm.matrix_world @ arm.data.bones[B(n)].head_local
def rest_rot(n): return (arm.matrix_world @ arm.data.bones[B(n)].matrix_local).to_quaternion().to_matrix()

# ====================== empunhadura ======================
# Cada mão fica virada, em relação ao fuzil, como a mão do boneco de referência (comparando os eixos da mão tirados dos nós
# dos dedos, que valem para os dois esqueletos), e cada falange aponta para onde a falange dele aponta.
HAND = {s: bone(s + 'Hand') for s in ('Left', 'Right')}
CONV = {s: hand_frame(rest_head(s + 'Hand'), rest_head(s + 'HandIndex1'), rest_head(s + 'HandMiddle1'), rest_head(s + 'HandPinky1')).inverted() @ rest_rot(s + 'Hand') for s in ('Left', 'Right')}
FINGERS = [pb for s in ('Left', 'Right') for pb in subtree(HAND[s]) if pb is not HAND[s]]
for s in ('Left', 'Right'):
    set_rot(HAND[s], S_REF[s] @ CONV[s])
    for fg in FINGER:
        for i in (1, 2, 3):
            pb = bone('%sHand%s%d' % (s, fg, i)); M = world(pb)
            cur = (world(pb.children[0]).translation - M.translation) if pb.children else M.to_3x3() @ Vector((0, 1, 0))
            turn_about(pb, cur.normalized().rotation_difference(D_REF[s, fg, i]))
FING0 = {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in FINGERS}
# a mão direita na posição que ocupa na referência: é ali que o fuzil se prende a ela
loc, rot, scl = world(HAND['Right']).decompose()
HR0 = Matrix.LocRotScale(A0, rot, scl); HR0i = HR0.inverted()
set_world(HAND['Right'], HR0)
gun.parent = arm; gun.parent_type = 'BONE'; gun.parent_bone = HAND['Right'].name
update(); gun.matrix_world = G0; update()
pose_restore(REST)

def gun_now():
    # deslocamento rígido do fuzil em relação ao lugar dele na referência (onde a mão direita o leva)
    return world(HAND['Right']) @ HR0i
def gun_from_wrists(wR, wL, fwd=None):
    # onde a arma fica para os dois punhos caírem nos pontos em que a referência a segura: a linha entre os punhos dá a
    # direção, o carregador fica para baixo; com `fwd`, o cano é levado exatamente para essa direção (mira)
    R = (B0 - A0).rotation_difference(wL - wR)
    e = (wL - wR).normalized()
    u1 = R @ U0; u1 = u1 - u1.dot(e) * e
    z = Vector((0, 0, 1)); z = z - z.dot(e) * e
    if u1.length > 1e-5 and z.length > 1e-5:
        ang = u1.angle(z)
        if u1.cross(z).dot(e) < 0: ang = -ang
        R = Quaternion(e, ang) @ R
    if fwd is not None:
        R = (R @ D0).rotation_difference(Vector(fwd).normalized()) @ R
        d = (R @ D0).normalized(); u1 = R @ U0; z = Vector((0, 0, 1)); z = z - z.dot(d) * d
        if z.length > 1e-5:
            ang = u1.angle(z)
            if u1.cross(z).dot(d) < 0: ang = -ang
            R = Quaternion(d, ang) @ R
    return Matrix.Translation(wR) @ R.to_matrix().to_4x4() @ Matrix.Translation(-A0)
def reach(s, Wt):
    # dobra o braço (ombro e cotovelo) até o punho chegar ao ponto Wt; o cotovelo continua dobrando para o mesmo lado
    ua, fa, hb = bone(s + 'Arm'), bone(s + 'ForeArm'), HAND[s]
    S, E0, W0 = world(ua).translation.copy(), world(fa).translation.copy(), world(hb).translation.copy()
    L1, L2 = (E0 - S).length, (W0 - E0).length
    v = Wt - S; d = min(max(v.length, abs(L1 - L2) + 1e-4), (L1 + L2) * 0.999); u = v.normalized()
    a_ = (L1 * L1 - L2 * L2 + d * d) / (2 * d); h = max(L1 * L1 - a_ * a_, 0.0) ** 0.5
    pole = (E0 - S) - (E0 - S).dot(u) * u
    if pole.length < 1e-5: pole = Vector((0, 0, -1)) - Vector((0, 0, -1)).dot(u) * u
    E = S + u * a_ + pole.normalized() * h; Wt = S + u * d
    turn_about(ua, (E0 - S).rotation_difference(E - S))
    E1, W1 = world(fa).translation.copy(), world(hb).translation.copy()
    turn_about(fa, (W1 - E1).rotation_difference(Wt - E1))
def fix_hands(T, right=False):
    # mão direita na empunhadura (com `right`, o braço direito também é levado até ela); braço esquerdo dobrado até o
    # punho chegar ao guarda-mão; dedos fechados como na referência
    R = T.to_quaternion().to_matrix()
    if right: reach('Right', T @ A0)
    set_rot(HAND['Right'], R @ S_REF['Right'] @ CONV['Right'])
    reach('Left', T @ B0)
    set_rot(HAND['Left'], R @ S_REF['Left'] @ CONV['Left'])
    for pb in FINGERS: pb.location, pb.rotation_quaternion, pb.scale = FING0[pb.name]
    update()
GRIPPED = [bone(s + n) for s in ('Right', 'Left') for n in ('Arm', 'ForeArm', 'Hand')] + [bone('Head')] + FINGERS
def grip_clip(act, fwd=None, muzzle=None, head_level=0.0):
    # passa o clipe inteiro acertando as mãos na arma (as mãos do Mixamo seguram um fuzil de outro tamanho).
    # muzzle: onde a boca do cano tem de ficar no primeiro quadro; a arma inteira é deslocada o mesmo tanto no clipe todo
    arm.animation_data.action = act
    drop_curves(act, FINGERS)
    last = {}; shift = None
    hd = bone('Head'); hrest = rest_rot('Head')
    for f in range(1, int(act.frame_range[1]) + 1):
        sc.frame_set(f); update()
        T = gun_from_wrists(world(HAND['Right']).translation.copy(), world(HAND['Left']).translation.copy(), fwd)
        if muzzle is not None:
            if shift is None:
                shift = Vector(muzzle) - T @ MUZZLE0
                print("MIRA arma deslocada (%.3f, %.3f, %.3f) para a boca cair no ponto que o jogo usa" % shift[:])
            T = Matrix.Translation(shift) @ T
        fix_hands(T, right=muzzle is not None)
        if head_level:
            # para onde a cabeça olha (em repouso ela olha para -Y); tira parte da inclinação para baixo ou para cima
            look = (world(hd).to_quaternion().to_matrix() @ hrest.inverted()) @ Vector((0, -1, 0))
            flat = Vector((look.x, look.y, 0)).normalized()
            turn_about(hd, Quaternion().slerp(look.rotation_difference(flat), head_level))
        for pb in GRIPPED:
            q = pb.rotation_quaternion
            if pb.name in last and last[pb.name].dot(q) < 0: pb.rotation_quaternion = -q
            last[pb.name] = pb.rotation_quaternion.copy()
        key(GRIPPED, (f,))
    sc.frame_set(1); update()
    d = (gun_now().to_3x3() @ D0).normalized()
    gap = (world(HAND['Left']).translation - gun_now() @ B0).length
    print("EMPUNHADURA %s: cano aponta para (%.2f, %.2f, %.2f); punho esquerdo a %.3f m do guarda-mão" % (act.name, d.x, d.y, d.z, gap))
def closed_fingers(act):
    # clipes em que as mãos não são acertadas na arma (quedas): os dedos ficam fechados do começo ao fim
    arm.animation_data.action = act
    drop_curves(act, FINGERS)
    for f in (1, int(act.frame_range[1])):
        sc.frame_set(f)
        for pb in FINGERS: pb.location, pb.rotation_quaternion, pb.scale = FING0[pb.name]
        key(FINGERS, (f,))
    sc.frame_set(1); update()

# ====================== clipes do Mixamo ======================
tmap = {n: bone(n) for n in ('Hips', 'Spine', 'Spine2', 'Neck', 'Head') + tuple(s + n for s in ('Left', 'Right')
        for n in ('Shoulder', 'Arm', 'ForeArm', 'Hand', 'HandMiddle1', 'UpLeg', 'Leg', 'Foot', 'ToeBase'))}
arm.animation_data_create()
def mx(name, file, **kw):
    act, n = mixamo_lib.retarget(os.path.join(MXDIR, file + '.fbx'), arm, tmap, name, **kw)
    print("MIXAMO %s: %d quadros" % (name, n)); return act
aim = mx('Aim', 'rifle_aim')
grip_clip(aim, (0, -1, 0), muzzle=AIM_MUZZLE, head_level=AIM_HEAD_LEVEL)
# onde ficam a boca do cano e a janela de ejeção na mira (o jogo põe ali o clarão e solta a cápsula: src/combat.rs, enemy_muzzle)
for nm, p0 in (('BOCA', MUZZLE0), ('EJECAO', EJECT0)):
    w_ = gun_now() @ p0
    print("%s na mira: %.3f à frente, %.3f para a esquerda dele, altura %.3f" % (nm, -w_.y, w_.x, w_.z))
aim_pose = pose_snapshot()
# disparo: rajada curta de três tiros (um a cada três quadros) e volta para a mira; feito aqui, como no conv_operator.
# Fuzil apoiado no ombro: quem absorve é o tronco (vai para trás) e a boca sobe pouco; a mão esquerda segue a arma.
shoot = bpy.data.actions.new('Shoot'); arm.animation_data.action = shoot
def kick(kk):
    pose_restore(aim_pose)
    rotx(bone('Spine'), -1.4 * kk); rotx(bone('Spine2'), -2.6 * kk)
    rotx(bone('Head'), 2.6 * kk)
    for s_ in ('Left', 'Right'):
        rotx(bone(s_ + 'Shoulder'), -1.6 * kk)
        rotx(bone(s_ + 'Arm'), -2.4 * kk)
    fix_hands(gun_now())
for frame, kk in ((1, 0.0), (2, 1.0), (3, 0.5), (4, 0.3), (5, 1.05), (6, 0.55), (7, 0.35), (8, 1.1), (9, 0.6), (11, 0.22), (13, -0.08), (16, 0.0)):
    kick(kk)
    key(list(arm.pose.bones), (frame,))
idle = mx('Idle', 'rifle_idle')
walk = mx('Walk', 'rifle_walk', in_place=True)
grip_clip(idle); grip_clip(walk)
walk_aim = walk.copy(); walk_aim.name = 'WalkAim'
CLIPS = ['Idle', 'Aim', 'Shoot', 'Walk', 'WalkAim']
# quedas: o corpo anda menos que no clipe original, para caber no cômodo. A linha FIMQUEDA dá onde a cabeça vai parar,
# no espaço do jogo (x = para a esquerda dele, z = para a frente): é o que está em FALL_REACH, em src/operator.rs.
for v, f in mixamo_lib.FALLS:
    act = mx(v, f, travel=0.6)
    closed_fingers(act)
    arm.animation_data.action = act; sc.frame_set(int(act.frame_range[1]))
    hp_, he = world(bone('Hips')).translation, world(bone('Head')).translation
    print("FIMQUEDA %s quadril (x %.2f, z %.2f, altura %.2f) cabeça (x %.2f, z %.2f, altura %.2f)" % (v, hp_.x, -hp_.y, hp_.z, he.x, -he.y, he.z))
    CLIPS.append(v)
# clipes extras (andar para trás, passo lateral, corrida, proteger o rosto, tiro levado, arremesso): tools3d/extra_clips.py
import extra_clips
from types import SimpleNamespace
CLIPS += extra_clips.build(SimpleNamespace(arm=arm, sc=sc, mxdir=MXDIR, tmap=tmap, mx=mx, world=world, set_world=set_world, key=key,
    snapshot=pose_snapshot, restore=pose_restore, grip_clip=grip_clip, gun_from_wrists=gun_from_wrists, fix_hands=fix_hands, gun_now=gun_now,
    gripped=GRIPPED, fingers=FINGERS, grip_dist=(B0 - A0).length, aim_pose=aim_pose, hip_height=rest_head('Hips').z))
arm.animation_data.action = idle
sc.frame_set(1); update()
p_ = posed_points(meshes)
print("ALTURA na espera %.3f m (referência %.3f); pés em z %.3f" % (p_[:, 2].max() - p_[:, 2].min(), REF_H, p_[:, 2].min()))

# ====================== malha ======================
def tri_count(o): return sum(len(p.vertices) - 2 for p in o.data.polygons)
total = 0
for o in meshes + [g for g in gun_parts if g.type == 'MESH']:
    t = next((n for key_, n in TRIS if o.name.startswith(key_)), None) if o in meshes else None
    tris = tri_count(o)
    if t and tris > t * 1.1:
        if o.data.users > 1: o.data = o.data.copy()
        d = o.modifiers.new('dec', 'DECIMATE'); d.ratio = t / tris
        with bpy.context.temp_override(object=o, active_object=o, selected_objects=[o]):
            while o.modifiers[0].name != 'dec': bpy.ops.object.modifier_move_up(modifier='dec')
            bpy.ops.object.modifier_apply(modifier='dec')
    print("MALHA %-16s %6d -> %6d" % (o.name, tris, tri_count(o)))
    total += tri_count(o)
print("TRIS", total)

# ====================== materiais ======================
# texturas em 1024; nada transparente (o jogo desenha as ameaças opacas)
soldier_mats = {m for o in meshes for m in o.data.materials if m}
for m in soldier_mats:
    if not m.use_nodes: continue
    p = next((n for n in m.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None)
    if not p: continue
    nt = m.node_tree
    for name in ('Transmission Weight', 'Alpha'):
        if name in p.inputs:
            for l in list(p.inputs[name].links): nt.links.remove(l)
    if 'Transmission Weight' in p.inputs: p.inputs['Transmission Weight'].default_value = 0.0
    p.inputs['Alpha'].default_value = 1.0
    m.blend_method = 'OPAQUE'
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image and n.image.size[0] > TEX:
            n.image.scale(TEX, TEX * n.image.size[1] // n.image.size[0])
    print("MATERIAL %s cor %s texturas %s" % (m.name, [round(c, 3) for c in p.inputs['Base Color'].default_value], [n.image.name for n in nt.nodes if n.type == 'TEX_IMAGE' and n.image]))

# ====================== exportação ======================
# nomes limpos (o esqueleto vem com o nome em cirílico mal codificado)
arm.name = 'Invader'; arm.data.name = 'Invader'
for o, n in zip(sorted(meshes, key=lambda o: o.name), ('invader_visor', 'invader_body', 'invader_gear')): o.name = n; o.data.name = n
# cada clipe numa faixa com o próprio nome, na mesma ordem do arquivo antigo
arm.animation_data.action = None
pose_restore(REST)
for tr in list(arm.animation_data.nla_tracks): arm.animation_data.nla_tracks.remove(tr)
for name in CLIPS:
    tr = arm.animation_data.nla_tracks.new(); tr.name = name; tr.strips.new(name, 1, bpy.data.actions[name])
sc.frame_start = 1; sc.frame_end = int(max(bpy.data.actions[n].frame_range[1] for n in CLIPS))
sc.render.fps = mixamo_lib.FPS
sc.frame_set(1)
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=82, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True, export_morph=False)
# pós-exportação: materiais do soldado sem transparência
raw = open(out, 'rb').read()
ln = struct.unpack_from('<I', raw, 12)[0]
g = json.loads(raw[20:20 + ln]); rest = raw[20 + ln:]
names = {m.name for m in soldier_mats}
for m in g['materials']:
    if m['name'] in names: m.pop('alphaMode', None); m.pop('alphaCutoff', None)
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest)
print("EXPORTADO", out, os.path.getsize(out))
