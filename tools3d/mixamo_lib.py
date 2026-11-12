"""Transfere animações do Mixamo (arquivos FBX baixados com o boneco padrão) para os esqueletos do jogo.

Os arquivos do Mixamo ficam em 3dmodels/mixamo/ (fora do git: a licença do Mixamo não deixa redistribuir o arquivo de
animação solto; ela só entra no jogo já aplicada aos nossos modelos).

Como funciona: para cada quadro e cada osso, pega o quanto o osso do Mixamo girou em relação à pose de repouso dele
(pose em T) e aplica esse mesmo giro, no espaço do mundo, ao osso correspondente do nosso boneco. Se o repouso do
nosso boneco não for a mesma pose em T (a refém repousa com os braços em A), cada osso é antes alinhado pela direção
"deste osso até o filho" nos dois esqueletos. O quadril também recebe o deslocamento, na proporção das alturas.
"""
import bpy
from mathutils import Vector, Quaternion

# para achar a direção de cada osso: o filho que continua a cadeia
CHAIN = {'Hips': 'Spine', 'Spine': 'Spine1', 'Spine1': 'Spine2', 'Spine2': 'Neck', 'Neck': 'Head',
         'LeftShoulder': 'LeftArm', 'LeftArm': 'LeftForeArm', 'LeftForeArm': 'LeftHand', 'LeftHand': 'LeftHandMiddle1',
         'RightShoulder': 'RightArm', 'RightArm': 'RightForeArm', 'RightForeArm': 'RightHand', 'RightHand': 'RightHandMiddle1',
         'LeftUpLeg': 'LeftLeg', 'LeftLeg': 'LeftFoot', 'LeftFoot': 'LeftToeBase', 'RightUpLeg': 'RightLeg', 'RightLeg': 'RightFoot', 'RightFoot': 'RightToeBase'}


# Arquivos esperados em 3dmodels/mixamo/ (baixados do Mixamo em FBX, "With Skin", 30 quadros por segundo, boneco X Bot):
#   pistol_idle_low  = espera com a pistola baixa        pistol_idle_aim = espera com a pistola apontada
#   pistol_walk      = caminhada com a pistola (marcar "In Place")
#   death_front, death_right, death_forward, death_back = quatro quedas diferentes (viram Fall1 a Fall4, nesta ordem)
FALLS = (('Fall1', 'death_front'), ('Fall2', 'death_right'), ('Fall3', 'death_forward'), ('Fall4', 'death_back'))
FPS = 30


def folder(arg=None):
    """Pasta dos FBX do Mixamo: a que veio na linha de comando ou 3dmodels/mixamo ao lado de tools3d."""
    import os
    d = arg if arg and os.path.isdir(arg) else os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '3dmodels', 'mixamo')
    if not os.path.isdir(d): raise SystemExit("Faltam as animações do Mixamo em %s (ver a lista no começo de tools3d/mixamo_lib.py)" % d)
    return d


def retarget(path, tgt, tmap, name, in_place=False, frames=None, travel=1.0, hip_base=None):
    """Cria no esqueleto `tgt` a ação `name` a partir do FBX do Mixamo em `path`.
    tmap: nome do osso no Mixamo (sem o prefixo "mixamorig:") -> osso de pose do alvo.
    frames: (primeiro, último) para usar só um trecho. travel: fração do deslocamento horizontal do quadril que é mantida
    (as quedas do Mixamo andam mais de 1 m; no jogo o corpo precisa ocupar menos chão). hip_base: onde fica o quadril do
    alvo em pé, no mundo (há esqueletos cujo repouso deixa o quadril no chão e a altura só aparece na pose); sem ele, vale o
    repouso. Devolve (ação, número de quadros)."""
    sc = bpy.context.scene
    objs0, acts0 = set(bpy.data.objects), set(bpy.data.actions)
    data0 = {c: set(getattr(bpy.data, c)) for c in ('meshes', 'materials', 'armatures', 'images')}
    bpy.ops.import_scene.fbx(filepath=path)
    new = [o for o in bpy.data.objects if o not in objs0]
    src = next(o for o in new if o.type == 'ARMATURE')
    act = max((a for a in bpy.data.actions if a not in acts0), key=lambda a: len(a.fcurves))
    src.animation_data.action = act
    f0, f1 = (int(act.frame_range[0]), int(act.frame_range[1])) if frames is None else frames
    bpy.context.view_layer.update()
    sw, tw = src.matrix_world.copy(), tgt.matrix_world.copy()
    swq, twq = sw.to_quaternion(), tw.to_quaternion()
    sb = {b.name.split(':')[-1]: b for b in src.data.bones}
    keys = [k for k in tmap if k in sb]
    # repouso dos dois esqueletos, no mundo
    s_rest = {k: (sw @ sb[k].matrix_local).to_quaternion() for k in keys}
    s_head = {k: sw @ sb[k].head_local for k in sb}
    t_rest = {k: (tw @ tmap[k].bone.matrix_local).to_quaternion() for k in keys}
    t_head = {k: tw @ tmap[k].bone.head_local for k in keys}
    # alinhamento de repouso: leva a direção do osso do alvo para a direção do osso do Mixamo
    Q = {}
    for k in keys:
        c = CHAIN.get(k)
        if c in s_head and c in t_head and (s_head[c] - s_head[k]).length > 1e-6 and (t_head[c] - t_head[k]).length > 1e-6:
            Q[k] = (t_head[c] - t_head[k]).normalized().rotation_difference((s_head[c] - s_head[k]).normalized())
    def q_of(k):
        b = tmap[k].bone
        while k not in Q:
            b = b.parent
            if b is None: return Quaternion()
            k = next((kk for kk in keys if tmap[kk].bone == b), None)
            if k is None: continue
        return Q[k]
    t_ref = {k: q_of(k) @ t_rest[k] for k in keys}
    depth = lambda b: 0 if b.parent is None else 1 + depth(b.parent)
    order = sorted(keys, key=lambda k: depth(tmap[k].bone))
    by_bone = {tmap[k].bone.name: k for k in keys}
    rest_q = {b.name: (tw @ b.matrix_local).to_quaternion() for b in tgt.data.bones}
    base = Vector(hip_base) if hip_base is not None else (t_head['Hips'].copy() if 'Hips' in t_head else Vector())
    scale = (base.z - tw.translation.z) / max(s_head['Hips'].z - sw.translation.z, 1e-6) if 'Hips' in t_head else 1.0
    tgt.animation_data_create()
    action = bpy.data.actions.new(name)
    tgt.animation_data.action = action
    # ossos sem correspondente ficam na pose de repouso (com chave, para o clipe exportado cobrir o esqueleto inteiro)
    mapped = {tmap[k].name for k in keys}
    for pb in tgt.pose.bones:
        pb.rotation_mode = 'QUATERNION'
        pb.location = (0, 0, 0); pb.scale = (1, 1, 1)
        if pb.name not in mapped: pb.rotation_quaternion = (1, 0, 0, 0)
    # braços: o comprimento do braço do alvo em relação ao do Mixamo, e o ponto médio dos ombros no repouso
    def arm_len(head, side): return (head[side + 'ForeArm'] - head[side + 'Arm']).length + (head[side + 'Hand'] - head[side + 'ForeArm']).length
    hands = all(s + n in t_head and s + n in s_head for s in ('Left', 'Right') for n in ('Arm', 'ForeArm', 'Hand'))
    karm = (arm_len(t_head, 'Left') / max(arm_len(s_head, 'Left'), 1e-6)) if hands else 1.0
    t_off = {}                                       # deslocamento de repouso de cada osso do alvo em relação ao pai, no mundo
    for b in tgt.data.bones:
        t_off[b.name] = (tw @ b.head_local) - (tw @ b.parent.head_local) if b.parent else Vector()
    last = {}
    for f in range(f0, f1 + 1):
        sc.frame_set(f)
        se = src.evaluated_get(bpy.context.evaluated_depsgraph_get())
        des = {}
        def world_q(b):
            # giro no mundo de um osso do alvo neste quadro (ossos sem correspondente acompanham o pai)
            if b is None: return twq
            if b.name in by_bone and by_bone[b.name] in des: return des[by_bone[b.name]]
            pq = world_q(b.parent)
            prest = rest_q[b.parent.name] if b.parent else twq
            return pq @ (prest.inverted() @ rest_q[b.name])
        out = f - f0 + 1
        s_pos = {k: (sw @ se.pose.bones[sb[k].name].matrix).translation for k in keys}
        for k in order:
            des[k] = ((sw @ se.pose.bones[sb[k].name].matrix).to_quaternion() @ s_rest[k].inverted()) @ t_ref[k]
        hip_off = Vector()
        if 'Hips' in des:
            d = (s_pos['Hips'] - s_head['Hips']) * scale
            k = 0.0 if in_place else travel
            # quadril do alvo neste quadro, no mundo; hip_off é o deslocamento em relação ao repouso do alvo
            hip_off = Vector((base.x + d.x * k, base.y + d.y * k, base.z + d.z)) - t_head['Hips']
        def world_p(b):
            # posição no mundo da cabeça de um osso do alvo neste quadro
            if b.parent is None: return tw @ b.head_local
            base = world_p(b.parent)
            if by_bone.get(b.name) == 'Hips': return (tw @ b.head_local) + hip_off
            return base + (world_q(b.parent) @ rest_q[b.parent.name].inverted()) @ t_off[b.name]
        if hands:
            # as mãos do Mixamo se encontram na arma; com braços de outro tamanho elas ficariam afastadas. Cada punho vai
            # para o mesmo lugar em relação aos ombros que ocupa no Mixamo, na proporção do braço (cotovelo dobra para o mesmo lado).
            s_mid = (s_pos['LeftArm'] + s_pos['RightArm']) / 2
            t_mid = (world_p(tmap['LeftArm'].bone) + world_p(tmap['RightArm'].bone)) / 2
            for s in ('Left', 'Right'):
                ub, fb, hb = tmap[s + 'Arm'].bone, tmap[s + 'ForeArm'].bone, tmap[s + 'Hand'].bone
                S = world_p(ub); E0 = world_p(fb); W0 = world_p(hb)
                L1, L2 = (E0 - S).length, (W0 - E0).length
                W = t_mid + (s_pos[s + 'Hand'] - s_mid) * karm
                v = W - S; d = min(max(v.length, abs(L1 - L2) + 1e-4), (L1 + L2) * 0.999); u = v.normalized()
                a_ = (L1 * L1 - L2 * L2 + d * d) / (2 * d); h = max(L1 * L1 - a_ * a_, 0.0) ** 0.5
                pole = (E0 - S) - (E0 - S).dot(u) * u
                if pole.length < 1e-5: pole = Vector((0, 0, -1)) - Vector((0, 0, -1)).dot(u) * u
                E = S + u * a_ + pole.normalized() * h; W = S + u * d
                des[s + 'Arm'] = (E0 - S).normalized().rotation_difference((E - S).normalized()) @ des[s + 'Arm']
                des[s + 'ForeArm'] = (W0 - E0).normalized().rotation_difference((W - E).normalized()) @ des[s + 'ForeArm']
        for k in order:
            D = des[k]
            pb = tmap[k]; b = pb.bone
            pq = world_q(b.parent)
            prest = rest_q[b.parent.name] if b.parent else twq
            basis = (prest.inverted() @ t_rest[k]).inverted() @ (pq.inverted() @ D)
            if k in last and last[k].dot(basis) < 0: basis.negate()
            last[k] = basis.copy()
            pb.rotation_quaternion = basis
            pb.keyframe_insert('rotation_quaternion', frame=out)
            if k == 'Hips':
                loc = t_rest[k].inverted() @ hip_off
                s3 = tw.to_scale()
                pb.location = Vector((loc.x / s3.x, loc.y / s3.y, loc.z / s3.z))
                pb.keyframe_insert('location', frame=out)
        if f in (f0, f1):
            for pb in tgt.pose.bones:
                if pb.name not in mapped:
                    for path_ in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path_, frame=out)
                else:
                    pb.keyframe_insert('scale', frame=out)
                    if pb.name != tmap.get('Hips', pb).name: pb.keyframe_insert('location', frame=out)
    for o in new: bpy.data.objects.remove(o)
    for a in list(bpy.data.actions):
        if a not in acts0 and a is not action: bpy.data.actions.remove(a)
    for c, before in data0.items():
        coll = getattr(bpy.data, c)
        for d in list(coll):
            if d not in before: coll.remove(d)
    sc.render.fps = FPS
    sc.frame_set(1)
    return action, f1 - f0 + 1
