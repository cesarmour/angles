"""Refém a partir da "Carla Rigged 001" (Renderpeople, CC BY 4.0) -> hostage.glb
Pessoa escaneada (roupa social), uma malha só de 20 mil triângulos, UM material com a textura inteira já pintada, esqueleto
de 89 ossos com nomes próprios ("upperarm_l", "lowerarm_twist_l"...), em pose A, sem animação. Sai um arquivo com as mesmas
convenções do assets/models/hostage.glb feito pelo conv_hostage.py, para o jogo usar sem mudar o código:

  - clipes, nesta ordem e com estas durações: "Idle" (mãos para o alto, 250 quadros), "Fall1" a "Fall4" (quedas do
    Mixamo, pelo mixamo_lib) e "Relax" (braços abaixados, 250 quadros);
  - 1,70 m de altura em repouso, pés na origem, de frente para +Z no arquivo (-Y no Blender), 1 unidade = 1 m;
  - textura em 1024, sem brilho (rugosidade alta, nada metálico).

  blender -b --python tools3d/conv_carla.py -- <carla.glb> <saida.glb> [pasta dos FBX do Mixamo]

Diferenças para a refém antiga que o jogo percebe: o material é um só ("Carla_Mat"), então o sorteio de cor de blusa,
calça e cabelo (src/operator.rs, hostage_look) não acha nada para pintar e todas as reféns saem iguais. Os ossos não têm
os nomes do Mixamo; o jogo só procura esses nomes nas ameaças (hands_setup), não na refém.

O que muda em relação ao original: poses e clipes feitos aqui; escala (o arquivo vem em centímetros); textura reduzida.
A malha fica como veio.

Sobre os ombros: gente escaneada deforma mal quando o braço sobe, porque a pele do ombro inteira gira junto com o osso do
braço. O esqueleto traz ossos de torção no meio do braço e do antebraço ("upperarm_twist", "lowerarm_twist"); depois de
montar cada clipe, o giro do braço em torno do próprio eixo é repartido entre o osso do braço (metade de cima, perto do
ombro) e o osso de torção (metade de baixo), e o giro do punho é repartido com a torção do antebraço (ver spread_twist).
"""
import bpy, sys, os, re, json, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mixamo_lib
import numpy as np
from math import radians, sin, pi, atan2
from mathutils import Vector, Matrix, Quaternion

a = sys.argv[sys.argv.index('--') + 1:]
src, out = a[0], a[1]

HEIGHT = 1.70
TEX = 1024
ROUGH = 0.95
FRAMES = 250          # espera e braços abaixados: 250 quadros a 30 por segundo, em laço (igual à refém antiga)
BLEND = 14            # quadros em que os braços saem das mãos para o alto e entram na queda
# quanto do giro do braço em torno do próprio eixo fica no osso do braço (metade de cima, junto do ombro); o resto vai
# para o osso de torção (metade de baixo). 1 = tudo no braço (o ombro torce feito pano); 0 = o ombro não gira nada.
UPPER_KEEP = 0.45
FORE_KEEP = 0.5       # o mesmo para o antebraço (giro que o próprio osso do antebraço traz, nas quedas do Mixamo)
WRIST_SHARE = 0.55    # fração do giro do punho que a metade de baixo do antebraço acompanha
# mãos para o alto (ela olha para -Y; o braço esquerdo dela fica em +X; sx = +1 no esquerdo, -1 no direito)
SHOULDER_LIFT = 10.0                       # a clavícula sobe junto com o braço, em graus
UP_ARM = (0.86, -0.16, -0.30)              # do ombro para o cotovelo (x multiplicado por sx)
UP_FORE = (0.12, -0.20, 1.0)               # do cotovelo para o punho
UP_HAND = (0.08, -0.10, 1.0)               # do punho para os dedos
UP_PALM = (-0.18, -1.0, 0.0)               # para onde a palma olha: para a frente, um pouco para dentro
# braços abaixados
DOWN_ARM = (0.21, 0.03, -1.0)
DOWN_FORE = (0.10, -0.24, -1.0)
DOWN_HAND = (0.06, -0.12, -1.0)
DOWN_PALM = (-1.0, -0.25, 0.0)             # palma para a coxa
# direção de repouso alinhada com a do Mixamo só nos braços e nas pernas (ela repousa em A, o Mixamo em T). Tronco,
# pescoço, ombros e pés ficam como foram escaneados: o pé dela está no salto, e alinhá-lo ao pé chato do Mixamo
# afundaria o calcanhar no chão.
mixamo_lib.CHAIN = {k: v for k, v in mixamo_lib.CHAIN.items() if re.search(r'Arm|Hand|UpLeg|Leg', k) and 'Shoulder' not in k and not k.endswith('Foot')}

bpy.ops.wm.read_factory_settings(use_empty=True)
# ossos criados a partir da posição dos nós do arquivo (como no conv_hostage): com a opção padrão os ossos de ponta
# ("head_end", "thumb_end"...) nascem na origem
bpy.ops.import_scene.gltf(filepath=src, guess_original_bind_pose=False)
sc = bpy.context.scene
def update(): bpy.context.view_layer.update()
arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
meshes = [o for o in bpy.data.objects if o.type == 'MESH' and o.find_armature() is arm]
if len(meshes) != 1: raise SystemExit("Esperava uma malha só presa ao esqueleto; achei %d" % len(meshes))
body = meshes[0]

# ---------- limpeza e escala ----------
update()
mw = arm.matrix_world.copy(); arm.parent = None; arm.matrix_world = mw
for o in list(bpy.data.objects):
    if o is not arm and o is not body: bpy.data.objects.remove(o)       # a esfera solta do Sketchfab e os nós vazios
for o in (arm, body):
    o.animation_data_clear()
if body.data.shape_keys: body.shape_key_clear()
for act in list(bpy.data.actions): bpy.data.actions.remove(act)
for pb in arm.pose.bones:
    pb.rotation_mode = 'QUATERNION'
    pb.location = (0, 0, 0); pb.rotation_quaternion = (1, 0, 0, 0); pb.scale = (1, 1, 1)
sc.frame_set(1); update()
def posed_points():
    dg = bpy.context.evaluated_depsgraph_get()
    e = body.evaluated_get(dg); me = e.to_mesh()
    co = np.empty(len(me.vertices) * 3, dtype=np.float64); me.vertices.foreach_get('co', co)
    p = (np.array(e.matrix_world) @ np.hstack([co.reshape(-1, 3), np.ones((len(me.vertices), 1))]).T).T[:, :3]
    e.to_mesh_clear()
    return p
p_ = posed_points(); lo, hi = p_[:, 2].min(), p_[:, 2].max()
k = HEIGHT / (hi - lo)
arm.matrix_world = Matrix.Translation((0, 0, -lo * k)) @ Matrix.Scale(k, 4) @ arm.matrix_world
update()
# a escala e a posição entram na malha e no esqueleto: o objeto do esqueleto fica na origem, no chão, sem giro nem escala
# (o mixamo_lib mede a altura do quadril a partir da origem do objeto)
mw = body.matrix_world.copy(); body.parent = None; body.matrix_world = mw
update()
bpy.ops.object.select_all(action='DESELECT')
for o in (arm, body): o.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
body.parent = arm; body.matrix_parent_inverse = Matrix.Identity(4)
update()
p_ = posed_points()
assert abs(p_[:, 2].min()) < 1e-3 and abs(p_[:, 2].max() - HEIGHT) < 1e-3, "a malha saiu do lugar ao aplicar a escala"
print("CARLA %.3f no arquivo -> %.3f m em repouso (escala %.5f); olha para %s" % (hi - lo, HEIGHT, k, "-Y" if p_[p_[:, 2] > HEIGHT * 0.93][:, 1].mean() < 0 else "+Y"))

# vértices sem osso iriam parar na origem na tela do jogo: aqui só confere (este arquivo não tem nenhum)
bone_names = {b.name for b in arm.data.bones}
gi = {g.index for g in body.vertex_groups if g.name in bone_names}
loose = sum(1 for v in body.data.vertices if sum(g.weight for g in v.groups if g.group in gi) < 1e-4)
print("PESOS %d vértices, %d sem osso" % (len(body.data.vertices), loose))
if loose: raise SystemExit("Há vértices sem osso: tratar como no conv_invader.py")

# ---------- pose: ferramentas ----------
def bone(name): return next(pb for pb in arm.pose.bones if re.fullmatch(re.escape(name) + r'_\d+', pb.name))
def world(pb):
    update()
    dg = bpy.context.evaluated_depsgraph_get()
    return arm.matrix_world @ arm.evaluated_get(dg).pose.bones[pb.name].matrix
def set_world(pb, M):
    dg = bpy.context.evaluated_depsgraph_get()
    ev = arm.evaluated_get(dg).pose.bones
    local = arm.matrix_world.inverted() @ M
    rest = pb.bone.matrix_local
    if pb.parent:
        prest = pb.parent.bone.matrix_local
        basis = (prest.inverted() @ rest).inverted() @ (ev[pb.parent.name].matrix.inverted() @ local)
    else:
        basis = rest.inverted() @ local
    pb.rotation_quaternion = basis.to_quaternion()
    pb.location = (0, 0, 0); pb.scale = (1, 1, 1)
    update()
def spin(pb, R):
    M = world(pb); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ R.to_4x4() @ Matrix.Translation(-h) @ M)
def head(name): return world(bone(name)).translation.copy()
def point(pb, child, d):
    # gira o osso (em torno da própria junta) até a direção junta -> junta seguinte ficar igual a d
    cur = (head(child) - world(pb).translation).normalized()
    spin(pb, cur.rotation_difference(Vector(d).normalized()).to_matrix())
def twist(pb, deg, axis='Z'):
    # gira em torno de um eixo do mundo (ou de um vetor) passando pela junta
    spin(pb, Matrix.Rotation(radians(deg), 3, axis if isinstance(axis, str) else Vector(axis).normalized()))
def angle_about(v0, v1, axis):
    # ângulo, em torno de `axis`, que leva v0 a v1 (os dois achatados no plano perpendicular ao eixo)
    ax = Vector(axis).normalized()
    p0 = v0 - v0.dot(ax) * ax; p1 = v1 - v1.dot(ax) * ax
    return atan2(p0.cross(p1).dot(ax), p0.dot(p1))
def snapshot(): return {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in arm.pose.bones}
def restore(pose):
    for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = pose[pb.name]
    update()
def key_all(frame):
    for pb in arm.pose.bones:
        for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=frame)

SIDES = (('l', 1.0), ('r', -1.0))
def side(v, sx): return Vector((v[0] * sx, v[1], v[2]))
def palm_normal(s):
    # para onde a palma olha, pelos nós dos dedos (vale para as duas mãos: o sinal sai do repouso, palma para dentro e para baixo)
    w_, i_, p_ = head('hand_' + s), head('index_01_' + s), head('pinky_01_' + s)
    return (i_ - w_).cross(p_ - w_).normalized() * PALM_SIGN[s]
PALM_SIGN = {'l': 1.0, 'r': 1.0}
for s, sx in SIDES:
    if palm_normal(s).dot(Vector((-sx, 0, -1))) < 0: PALM_SIGN[s] = -1.0

def arm_pose(s, sx, d_arm, d_fore, d_hand, d_palm, lift=0.0):
    """Leva um braço para a pose dada por direções no mundo. O giro do braço em torno do próprio eixo (o que vira o
    antebraço para cima ou para a frente) é posto aqui todo no osso do braço; spread_twist reparte depois."""
    sh, ua, fa, hd = bone('shoulder_' + s), bone('upperarm_' + s), bone('lowerarm_' + s), bone('hand_' + s)
    if lift: twist(sh, -sx * lift, 'Y')
    point(ua, 'lowerarm_' + s, side(d_arm, sx))
    # o cotovelo só dobra num plano: o braço gira no próprio eixo até esse plano conter a direção pedida para o antebraço
    S, E, W_ = head('upperarm_' + s), head('lowerarm_' + s), head('hand_' + s)
    u = (E - S).normalized()
    twist(ua, np.degrees(angle_about(W_ - E, side(d_fore, sx), u)), u)
    point(fa, 'hand_' + s, side(d_fore, sx))
    # punho: gira em torno do antebraço até a palma olhar para onde foi pedido, depois aponta os dedos
    f = (head('hand_' + s) - head('lowerarm_' + s)).normalized()
    twist(hd, np.degrees(angle_about(palm_normal(s), side(d_palm, sx), f)), f)
    point(hd, 'middle_01_' + s, side(d_hand, sx))
    f2 = (head('middle_01_' + s) - head('hand_' + s)).normalized()
    twist(hd, np.degrees(angle_about(palm_normal(s), side(d_palm, sx), f2)), f2)

def loop_clip(name, pose, arms_up):
    """Clipe em laço de 250 quadros sobre uma pose: o tronco e a cabeça viram um pouco para um lado e para o outro (uma
    ida e volta, como na refém antiga) e ela respira (três vezes no laço: peito e ombros sobem um pouco). Com as mãos para
    o alto, os braços também balançam de leve, cada um no seu tempo."""
    act = bpy.data.actions.new(name); arm.animation_data.action = act
    frames = list(range(1, FRAMES, 6)) + [FRAMES]
    for f in frames:
        t = (f - 1) / (FRAMES - 1) * 2 * pi
        restore(pose)
        look = sin(t); br = sin(3 * t - 0.6)
        twist(bone('spine_01'), 2.5 * look)
        twist(bone('spine_03'), -0.7 * br, 'X')
        for s, sx in SIDES:
            twist(bone('shoulder_' + s), -sx * 0.9 * br, 'Y')
            if arms_up:
                twist(bone('upperarm_' + s), 1.3 * sin(t + (0.9 if s == 'l' else 2.3)), 'Y')
                twist(bone('lowerarm_' + s), 1.6 * sin(2 * t + (0.4 if s == 'l' else 1.9)), 'X')
        twist(bone('head'), 9.0 * look)
        twist(bone('head'), 0.5 * br, 'X')
        key_all(f)
    return act

# ---------- repartir a torção ----------
def spread_twist(act):
    """Passa o clipe inteiro repartindo o giro em torno do eixo do braço e do antebraço com os ossos de torção.
    Braço: o osso do braço (pele da metade de cima, junto do ombro) fica só com UPPER_KEEP do giro; o osso de torção
    (metade de baixo) e o antebraço continuam exatamente onde estavam. Antebraço: idem com FORE_KEEP; além disso a torção
    do antebraço acompanha WRIST_SHARE do giro do punho. Mão e antebraço não saem do lugar: só a pele entre eles muda."""
    arm.animation_data.action = act
    rest = {b.name: b.matrix_local.to_quaternion() for b in arm.data.bones}
    def rel(pb): return rest[pb.parent.name].inverted() @ rest[pb.name]          # repouso do osso visto do pai
    def axis_in(pb, child):
        # direção junta -> junta seguinte, no espaço de repouso do próprio osso
        return (pb.bone.matrix_local.to_3x3().inverted() @ (child.bone.head_local - pb.bone.head_local)).normalized()
    def twist_angle(q, ax, prev):
        # parte do giro q que é em torno de ax, em radianos, sem saltos de um quadro para o outro
        th = 2 * atan2(Vector((q.x, q.y, q.z)).dot(ax), q.w)
        while th > pi: th -= 2 * pi
        while th < -pi: th += 2 * pi
        if prev is not None:
            while th - prev > pi: th -= 2 * pi
            while th - prev < -pi: th += 2 * pi
        return th
    f1 = int(act.frame_range[1])
    rows = []
    prev = {}
    for f in range(1, f1 + 1):
        sc.frame_set(f)
        row = {}
        for s, _ in SIDES:
            A, At, F, Ft, H = (bone(n + s) for n in ('upperarm_', 'upperarm_twist_', 'lowerarm_', 'lowerarm_twist_', 'hand_'))
            qA, qAt, qF, qFt, qH = (pb.rotation_quaternion.copy() for pb in (A, At, F, Ft, H))
            # braço
            uA = axis_in(A, F)
            th = twist_angle(qA, uA, prev.get(('A', s))); prev['A', s] = th
            D = Quaternion(uA, th * (1.0 - UPPER_KEEP))
            qA = qA @ D.inverted()
            qAt = rel(At).inverted() @ D @ rel(At) @ qAt
            qF = rel(F).inverted() @ D @ rel(F) @ qF
            # antebraço: o giro que o próprio osso traz
            uF = axis_in(F, H)
            th = twist_angle(qF, uF, prev.get(('F', s))); prev['F', s] = th
            E = Quaternion(uF, th * (1.0 - FORE_KEEP))
            qF = qF @ E.inverted()
            qH = rel(H).inverted() @ E @ rel(H) @ qH
            # torção do antebraço: fica onde estava e ainda acompanha parte do giro do punho
            G = rel(H) @ qH @ rel(H).inverted()                                  # giro da mão em relação ao antebraço, no espaço do antebraço
            th = twist_angle(G, uF, prev.get(('H', s))); prev['H', s] = th
            qFt = rel(Ft).inverted() @ Quaternion(uF, th * WRIST_SHARE) @ E @ rel(Ft) @ qFt
            for pb, q in ((A, qA), (At, qAt), (F, qF), (Ft, qFt), (H, qH)):
                last = prev.get(('q', pb.name))
                if last is not None and last.dot(q) < 0: q.negate()
                prev['q', pb.name] = q.copy(); row[pb.name] = q
        rows.append(row)
    for f, row in enumerate(rows, 1):
        for n, q in row.items():
            pb = arm.pose.bones[n]; pb.rotation_quaternion = q
            pb.keyframe_insert('rotation_quaternion', frame=f)
    sc.frame_set(1); update()

def settle_on_floor(act):
    """Nas quedas o quadril desce na proporção do boneco do Mixamo, que tem outro corpo: deitada de bruços, a barriga dela
    entrava até 11 cm no chão; de costas, ficava 4 cm no ar. Quadro a quadro, o quadril sobe o tanto que a malha passou do
    chão; e, com o corpo já baixo (quadril a menos de LOW do chão), desce o tanto que ela ficou no ar. O ajuste é suavizado
    no tempo, para não tremer. Devolve o maior ajuste para cima e o maior para baixo, em metros."""
    LOW = 0.45
    arm.animation_data.action = act
    hip = bone('hip')
    f1 = int(act.frame_range[1])
    gap, low = [], []
    for f in range(1, f1 + 1):
        sc.frame_set(f); update()
        gap.append(float(posed_points()[:, 2].min())); low.append(head('hip').z < LOW)
    gap = np.array(gap)
    want = np.where(low, -gap, np.maximum(-gap, 0.0))                 # quanto o quadril tem de subir (negativo = descer)
    smooth = np.array([want[max(0, i - 3):i + 4].mean() for i in range(len(want))])
    up = np.where(gap < 0, np.maximum(smooth, -gap), smooth)          # nunca menos que o necessário para sair do chão
    up[-1] = want[-1]                                                  # a última pose (a que fica na tela) encosta exata
    to_local = (arm.matrix_world @ hip.bone.matrix_local).to_quaternion().inverted()
    for f in range(1, f1 + 1):
        if abs(up[f - 1]) < 1e-5: continue
        sc.frame_set(f)
        hip.location = hip.location + to_local @ Vector((0, 0, up[f - 1]))
        hip.keyframe_insert('location', frame=f)
    sc.frame_set(1); update()
    return float(max(up.max(), 0.0)), float(max(-up.min(), 0.0))

# ---------- clipes ----------
REST = snapshot()
arm.animation_data_create()
for s, sx in SIDES: arm_pose(s, sx, UP_ARM, UP_FORE, UP_HAND, UP_PALM, lift=SHOULDER_LIFT)
base = snapshot()
p_ = posed_points()
print("MAOS PARA O ALTO: ponto mais alto %.3f m; punhos em %s" % (p_[:, 2].max(), ["(%.2f, %.2f, %.2f)" % head('hand_' + s)[:] for s, _ in SIDES]))
idle = loop_clip('Idle', base, True)
spread_twist(idle)
clips = [('Idle', idle)]
# braços no primeiro quadro da espera, já com a torção repartida: é de onde as quedas partem
ARMS = [bone(n + s) for s, _ in SIDES for n in ('shoulder_', 'upperarm_', 'upperarm_twist_', 'lowerarm_', 'lowerarm_twist_', 'hand_')]
arm.animation_data.action = idle; sc.frame_set(1); update()
start = {pb.name: pb.rotation_quaternion.copy() for pb in ARMS}

# quedas: as quatro animações do Mixamo (nomes dos ossos do Mixamo -> ossos deste esqueleto)
arm.animation_data.action = None
restore(REST)
MAP = {'Hips': 'hip', 'Spine': 'spine_01', 'Spine1': 'spine_02', 'Spine2': 'spine_03', 'Neck': 'neck', 'Head': 'head'}
for mx_s, s in (('Left', 'l'), ('Right', 'r')):
    MAP.update({mx_s + 'Shoulder': 'shoulder_' + s, mx_s + 'Arm': 'upperarm_' + s, mx_s + 'ForeArm': 'lowerarm_' + s, mx_s + 'Hand': 'hand_' + s,
                mx_s + 'HandMiddle1': 'middle_01_' + s, mx_s + 'UpLeg': 'upperleg_' + s, mx_s + 'Leg': 'lowerleg_' + s, mx_s + 'Foot': 'foot_' + s, mx_s + 'ToeBase': 'ball_' + s})
tmap = {k_: bone(v_) for k_, v_ in MAP.items()}
MXDIR = mixamo_lib.folder(a[2] if len(a) > 2 else None)
for v, f in mixamo_lib.FALLS:
    act, n = mixamo_lib.retarget(os.path.join(MXDIR, f + '.fbx'), arm, tmap, v, travel=0.6)
    spread_twist(act)
    # nos primeiros quadros os braços saem das mãos para o alto e entram na queda
    arm.animation_data.action = act
    for fr in range(1, BLEND + 1):
        sc.frame_set(fr)
        w = (fr - 1) / BLEND; w = w * w * (3 - 2 * w)
        for pb in ARMS:
            q0 = start[pb.name].copy()
            if q0.dot(pb.rotation_quaternion) < 0: q0.negate()
            pb.rotation_quaternion = q0.slerp(pb.rotation_quaternion, w)
            pb.keyframe_insert('rotation_quaternion', frame=fr)
    lift = settle_on_floor(act)
    sc.frame_set(int(act.frame_range[1]))
    hp, he = head('hip'), head('head')
    p_ = posed_points()
    # FIMQUEDA: onde o corpo vai parar, no espaço do jogo (x = para a esquerda dela, z = para a frente). A cabeça é o que
    # está em FALL_REACH, em src/operator.rs.
    print("FIMQUEDA %s: %d quadros, quadril (x %.2f, z %.2f, altura %.2f) cabeça (x %.2f, z %.2f, altura %.2f); malha entre %.3f e %.3f m do chão (quadril ajustado: até %.3f m para cima, %.3f para baixo)" % (
        v, n, hp.x, -hp.y, hp.z, he.x, -he.y, he.z, p_[:, 2].min(), p_[:, 2].max(), lift[0], lift[1]))
    clips.append((v, act))
sc.frame_set(1)

# braços abaixados: depois que o cômodo dela fica limpo, a refém baixa os braços (mesmo balanço da espera)
arm.animation_data.action = None
restore(REST)
for s, sx in SIDES: arm_pose(s, sx, DOWN_ARM, DOWN_FORE, DOWN_HAND, DOWN_PALM)
relaxed = snapshot()
relax = loop_clip('Relax', relaxed, False)
spread_twist(relax)
clips.append(('Relax', relax))

# conferência: altura nas poses
for name in ('Idle', 'Relax'):
    arm.animation_data.action = bpy.data.actions[name]; sc.frame_set(1); update()
    p_ = posed_points()
    print("POSE %s: malha de %.3f a %.3f m; largura %.2f m" % (name, p_[:, 2].min(), p_[:, 2].max(), p_[:, 0].max() - p_[:, 0].min()))

# os clipes vão para o arquivo cada um numa faixa com o próprio nome, na mesma ordem do arquivo antigo
arm.animation_data.action = None
restore(REST)
for name, act in clips:
    tr = arm.animation_data.nla_tracks.new(); tr.name = name; tr.strips.new(name, 1, act)
sc.frame_start, sc.frame_end = 1, FRAMES
sc.render.fps = mixamo_lib.FPS
sc.frame_set(1)

# ---------- malha e material ----------
print("TRIS", sum(len(p.vertices) - 2 for p in body.data.polygons))
arm.name = 'Hostage'; arm.data.name = 'Hostage'
body.name = 'hostage_body'; body.data.name = 'hostage_body'
mats = [m for m in body.data.materials if m]
for m in mats:
    m.name = 'Carla_Mat'
    nt = m.node_tree
    p = next(n for n in nt.nodes if n.type == 'BSDF_PRINCIPLED')
    # sem brilho: sob as luzes do jogo, gente escaneada com qualquer brilho fica com cara de plástico
    for name in ('Roughness', 'Metallic', 'Specular IOR Level', 'Specular Tint', 'Transmission Weight', 'Subsurface Weight', 'Coat Weight', 'Sheen Weight', 'Emission Color', 'Emission Strength', 'Alpha'):
        if name in p.inputs:
            for l in list(p.inputs[name].links): nt.links.remove(l)
    p.inputs['Roughness'].default_value = ROUGH
    p.inputs['Metallic'].default_value = 0.0
    for name in ('Transmission Weight', 'Subsurface Weight', 'Coat Weight', 'Sheen Weight', 'Emission Strength'):
        if name in p.inputs: p.inputs[name].default_value = 0.0
    p.inputs['Alpha'].default_value = 1.0; m.blend_method = 'OPAQUE'
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image and n.image.size[0] > TEX:
            n.image.scale(TEX, TEX * n.image.size[1] // n.image.size[0])
    print("MATERIAL %s texturas %s" % (m.name, [(n.image.name, n.image.size[:]) for n in nt.nodes if n.type == 'TEX_IMAGE' and n.image]))

bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=93, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True, export_morph=False)

# pós-exportação (como no conv_hostage): nada de transparência nem de extensões de brilho; rugosidade alta, nada metálico
raw = open(out, 'rb').read()
ln = struct.unpack_from('<I', raw, 12)[0]
g = json.loads(raw[20:20 + ln]); rest_ = raw[20 + ln:]
for m in g['materials']:
    m.pop('alphaMode', None); m.pop('alphaCutoff', None); m.pop('extensions', None)
    m['doubleSided'] = True
    pbr = m.setdefault('pbrMetallicRoughness', {})
    pbr.pop('metallicRoughnessTexture', None)
    pbr['metallicFactor'] = 0.0
    pbr['roughnessFactor'] = ROUGH
g['extensionsUsed'] = [e for e in g.get('extensionsUsed', []) if not e.startswith('KHR_materials')]
if not g['extensionsUsed']: g.pop('extensionsUsed')
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest_)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest_)
print("EXPORTADO", out, os.path.getsize(out))
