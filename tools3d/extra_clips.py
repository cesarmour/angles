"""Clipes extras das ameaças, iguais nos dois bonecos (o cáqui, do conv_operator.py no modo "threat", e o de camuflagem
verde, do conv_invader.py). Entram depois dos clipes que já existiam, nesta ordem e com estes nomes (o jogo procura pelo nome):

  WalkBack  backwards_rifle_walk      anda para trás, no lugar, em laço
  SneakL    left_cover_sneak_rifle    passo lateral agachado para a esquerda dele, no lugar, em laço
  SneakR    right_cover_sneak_rifle   idem para a direita
  Run       rifle_run                 corrida, no lugar, em laço
  Flinch    rifle_shielding_face      toca uma vez: protege o rosto com o braço esquerdo e termina agachado
  Hit       hit_reaction              toca uma vez: tranco de tiro levado, em pé, a partir da pose de mira
  Toss      toss_grenade              toca uma vez: arremesso com a mão esquerda (movimento espelhado), sem sair do lugar

Os FBX ficam em 3dmodels/mixamo/ e foram baixados com o deslocamento (sem "In Place"); o deslocamento sai na conversão
e a velocidade que ele tinha é impressa na linha VELOCIDADE, para o jogo casar o passo com o chão.

Cada script chama build(rig) com as próprias ferramentas de pose (os dois esqueletos e as duas empunhaduras são diferentes):
  rig.arm, rig.sc, rig.mxdir, rig.tmap (nome do Mixamo -> osso), rig.mx(nome, arquivo, **kw) -> ação,
  rig.world(osso), rig.set_world(osso, M), rig.key(ossos, quadros), rig.snapshot(), rig.restore(pose),
  rig.grip_clip(ação), rig.gun_from_wrists(punho direito, punho esquerdo), rig.fix_hands(arma), rig.gun_now(),
  rig.gripped (ossos que o ajuste das mãos mexe), rig.hip_height (altura do quadril em pé),
  rig.aim_pose (pose do primeiro quadro da mira)
"""
import bpy, os
from mathutils import Vector, Matrix, Quaternion

LOOPS = (('WalkBack', 'backwards_rifle_walk'), ('SneakL', 'left_cover_sneak_rifle'), ('SneakR', 'right_cover_sneak_rifle'), ('Run', 'rifle_run'))
# a mão esquerda só fica na arma enquanto, no Mixamo, as mãos estão juntas: distância entre os punhos em relação à
# distância que eles têm num clipe em que as duas mãos seguram o fuzil o tempo todo (o de andar para trás), medida
# antes do ajuste das mãos (abaixo do primeiro valor = na arma; acima do segundo = solta)
FREE = (1.08, 1.40)
UPPER = ('Spine', 'Spine1', 'Spine2', 'Neck', 'Head', 'LeftShoulder', 'RightShoulder', 'LeftArm', 'RightArm', 'LeftForeArm', 'RightForeArm')
HIT_GAIN = 0.45     # quanto do tranco do clipe original vai para o tronco em pé


def source_speed(path):
    """Deslocamento do quadril no FBX do Mixamo: (para a esquerda dele, para a frente, m/s) e a altura do quadril em repouso."""
    sc = bpy.context.scene
    o0, a0 = set(bpy.data.objects), set(bpy.data.actions)
    d0 = {c: set(getattr(bpy.data, c)) for c in ('meshes', 'materials', 'armatures', 'images')}
    bpy.ops.import_scene.fbx(filepath=path)
    new = [o for o in bpy.data.objects if o not in o0]
    src = next(o for o in new if o.type == 'ARMATURE')
    act = max((a for a in bpy.data.actions if a not in a0), key=lambda a: len(a.fcurves))
    src.animation_data.action = act
    hips = next(pb for pb in src.pose.bones if pb.name.split(':')[-1] == 'Hips')
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    pos = []
    for f in (f0, f1):
        sc.frame_set(f); bpy.context.view_layer.update()
        pos.append((src.matrix_world @ hips.matrix).translation.copy())
    rest_h = (src.matrix_world @ hips.bone.head_local).z - src.matrix_world.translation.z
    dur = (f1 - f0) / sc.render.fps
    for o in new: bpy.data.objects.remove(o)
    for a in list(bpy.data.actions):
        if a not in a0: bpy.data.actions.remove(a)
    for c, before in d0.items():
        coll = getattr(bpy.data, c)
        for d in list(coll):
            if d not in before: coll.remove(d)
    sc.frame_set(1)
    d = pos[1] - pos[0]
    return d.x / dur, -d.y / dur, rest_h


def _close_loop(rig, act, k=5):
    """Laço sem tranco: se o último quadro não é igual ao primeiro (a corrida do Mixamo fecha com uns 5 cm de diferença),
    os últimos k quadros são levados aos poucos até a pose do primeiro. Devolve a maior diferença de giro que havia, em graus."""
    arm, sc = rig.arm, rig.sc
    arm.animation_data.action = act
    n = int(act.frame_range[1])
    sc.frame_set(1); bpy.context.view_layer.update(); first = rig.snapshot()
    sc.frame_set(n); bpy.context.view_layer.update(); end = rig.snapshot()
    worst = max(first[b][1].rotation_difference(end[b][1]).angle for b in first)
    if worst < 1e-4 and max((first[b][0] - end[b][0]).length for b in first) < 1e-6: return 0.0
    for f in range(n - k + 1, n + 1):
        sc.frame_set(f); bpy.context.view_layer.update()
        w = _smooth(f, n - k, n)
        for pb in arm.pose.bones:
            l0, q0, _ = first[pb.name]; l1, q1, _ = end[pb.name]
            fix = q1.inverted() @ q0
            if fix.w < 0: fix.negate()
            pb.rotation_quaternion = pb.rotation_quaternion @ Quaternion().slerp(fix, w)
            pb.location = pb.location + (l0 - l1) * w
        rig.key(list(arm.pose.bones), (f,))
    sc.frame_set(1); bpy.context.view_layer.update()
    return worst * 57.2958


def _smooth(x, lo, hi):
    t = min(max((x - lo) / (hi - lo), 0.0), 1.0)
    return t * t * (3 - 2 * t)


def _partial_grip(rig, act):
    """Clipes em que a mão esquerda larga a arma (protege o rosto, arremessa): enquanto os punhos estão juntos, as duas mãos
    ficam na arma; quando se afastam, o braço esquerdo segue o Mixamo e a mão direita leva a arma com o punho travado
    (como estava no primeiro quadro)."""
    arm, sc = rig.arm, rig.sc
    arm.animation_data.action = act
    R, L = rig.tmap['RightHand'], rig.tmap['LeftHand']
    left = [rig.tmap['LeftArm'], rig.tmap['LeftForeArm'], L]
    last, held = {}, None
    wmin = 1.0
    for f in range(1, int(act.frame_range[1]) + 1):
        sc.frame_set(f); bpy.context.view_layer.update()
        wR, wL = rig.world(R).translation.copy(), rig.world(L).translation.copy()
        w = 1.0 - _smooth((wL - wR).length / rig.wrist_base, FREE[0], FREE[1]); wmin = min(wmin, w)
        free = {pb.name: pb.rotation_quaternion.copy() for pb in left}
        rig.fix_hands(rig.gun_from_wrists(wR, wL))
        if held is None: held = R.rotation_quaternion.copy()
        if w < 1.0:
            q = R.rotation_quaternion.copy()
            if held.dot(q) < 0: q.negate()
            R.rotation_quaternion = held.slerp(q, w)
            for pb in left:
                q = pb.rotation_quaternion.copy(); q0 = free[pb.name]
                if q0.dot(q) < 0: q.negate()
                pb.rotation_quaternion = q0.slerp(q, w)
        for pb in rig.gripped:
            q = pb.rotation_quaternion
            if pb.name in last and last[pb.name].dot(q) < 0: pb.rotation_quaternion = -q
            last[pb.name] = pb.rotation_quaternion.copy()
        rig.key(rig.gripped, (f,))
    sc.frame_set(1); bpy.context.view_layer.update()
    return wmin


def _hit_standing(rig, raw):
    """O clipe de tiro levado do Mixamo é feito de joelho no chão; a ameaça do jogo está em pé. Fica só o tranco: o quanto
    tronco, cabeça e braços giram em relação ao primeiro quadro do clipe é aplicado em cima da pose de mira (pernas e
    quadril ficam como na mira; as mãos continuam na arma). Começa e termina na pose de mira."""
    arm, sc = rig.arm, rig.sc
    names = [n for n in UPPER if n in rig.tmap]
    depth = lambda b: 0 if b.parent is None else 1 + depth(b.parent)
    names.sort(key=lambda n: depth(rig.tmap[n].bone))
    arm.animation_data.action = raw
    n_frames = int(raw.frame_range[1])
    Wq = {}
    for f in range(1, n_frames + 1):
        sc.frame_set(f); bpy.context.view_layer.update()
        Wq[f] = {n: rig.world(rig.tmap[n]).to_quaternion() for n in names}
    act = bpy.data.actions.new('Hit'); arm.animation_data.action = act
    rig.restore(rig.aim_pose)
    aim_q = {n: rig.world(rig.tmap[n]).to_quaternion() for n in names}
    peak = 0.0
    for f in range(1, n_frames + 1):
        rig.restore(rig.aim_pose)
        for n in names:
            d = Wq[f][n] @ Wq[1][n].inverted()
            if HIT_GAIN != 1.0: d = Quaternion().slerp(d, HIT_GAIN)
            if n == 'Spine2': peak = max(peak, d.angle)
            pb = rig.tmap[n]
            loc, _, scl = rig.world(pb).decompose()
            rig.set_world(pb, Matrix.LocRotScale(loc, d @ aim_q[n], scl))
        rig.fix_hands(rig.gun_now())
        rig.key(list(arm.pose.bones), (f,))
    arm.animation_data.action = None
    bpy.data.actions.remove(raw)
    sc.frame_set(1); bpy.context.view_layer.update()
    return act, peak


def build(rig):
    """Cria as sete ações no esqueleto e devolve os nomes, na ordem em que entram no arquivo."""
    names = []
    for name, file in LOOPS:
        vx, vf, src_h = source_speed(os.path.join(rig.mxdir, file + '.fbx'))
        act = rig.mx(name, file, in_place=True)
        if not hasattr(rig, 'wrist_base'):
            rig.arm.animation_data.action = act; rig.sc.frame_set(1); bpy.context.view_layer.update()
            rig.wrist_base = (rig.world(rig.tmap['LeftHand']).translation - rig.world(rig.tmap['RightHand']).translation).length
        rig.grip_clip(act)
        fixed = _close_loop(rig, act)
        if fixed: print("EXTRA %s: laço fechado (o último quadro diferia do primeiro em até %.1f graus num osso)" % (name, fixed))
        k = rig.hip_height / src_h
        print("VELOCIDADE %-8s no Mixamo: %.2f m/s (%.2f para a frente, %.2f para a esquerda dele); neste boneco (quadril a %.2f m, o do Mixamo a %.2f): %.2f m/s; %d quadros, %.3f s" % (
            name, (vx * vx + vf * vf) ** 0.5, vf, vx, rig.hip_height, src_h, (vx * vx + vf * vf) ** 0.5 * k, int(act.frame_range[1]), (act.frame_range[1] - 1) / 30.0))
        names.append(name)
    act = rig.mx('Flinch', 'rifle_shielding_face', in_place=True)
    w = _partial_grip(rig, act)
    print("EXTRA Flinch: %d quadros; mão esquerda na arma no mínimo %.0f%%" % (act.frame_range[1], w * 100))
    names.append('Flinch')
    raw = rig.mx('HitRaw', 'hit_reaction', in_place=True)
    act, peak = _hit_standing(rig, raw)
    print("EXTRA Hit: %d quadros; giro máximo do tronco %.1f graus" % (act.frame_range[1], peak * 57.2958))
    names.append('Hit')
    act = rig.mx('Toss', 'toss_grenade', in_place=True, mirror=True)
    w = _partial_grip(rig, act)
    print("EXTRA Toss: %d quadros; mão esquerda na arma no mínimo %.0f%%" % (act.frame_range[1], w * 100))
    names.append('Toss')
    return names
