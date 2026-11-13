"""Refém a partir de um personagem do Mixamo (FBX em pose T, já com o esqueleto "mixamorig") -> hostage.glb
Sai um arquivo com as mesmas convenções do assets/models/hostage.glb feito pelo conv_hostage.py: clipes "Idle" (mãos para
o alto, olha para os lados), "Relax" (braços abaixados) e "Fall1" a "Fall4" (quedas do Mixamo), 1,70 m de altura, de frente
para +Z no arquivo (-Y no Blender), e os materiais "Top_Mat", "Pants_Mat" e "hair_Transparency" em cinza claro, que é
onde o jogo aplica a cor de cada refém (src/operator.rs, hostage_look).

  blender -b --python tools3d/conv_civil.py -- <personagem.fbx> <saida.glb> [pasta dos FBX do Mixamo]

Feito para a "Louise" (3dmodels/mixamo/chars/Louise.fbx: blazer, calça, salto). Outros personagens do Mixamo entram pela
tabela PARTS logo abaixo: ela diz, pelo nome de cada malha, que papel a peça tem e para quantos triângulos ela é reduzida.

O que muda em relação ao original: pose e clipes feitos aqui; cílios retirados; malha reduzida; texturas em 1024;
blazer, calça e cabelo em cinza claro; sem brilho (rugosidade alta, sem textura de brilho).
"""
import bpy, sys, os, json, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mixamo_lib
import numpy as np
from math import radians
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index('--') + 1:]
src, out = a[0], a[1]

# trecho do nome da malha -> (papel, triângulos no máximo). Papéis: top/pants/hair recebem cor do jogo; drop sai do arquivo.
PARTS = (('Eyelash', 'drop', 0), ('Hair', 'hair', 6000), ('Suit', 'top', 5000), ('Pants', 'pants', 3000),
         ('Shirt', 'body', 1000), ('Heels', 'body', 900), ('Shoes', 'body', 900), ('Body', 'body', 7500))
MAT_NAME = {'top': 'Top_Mat', 'pants': 'Pants_Mat', 'hair': 'hair_Transparency', 'body': 'Body_Mat'}
HEIGHT = 1.70
TEX = 1024
ROUGH = 0.9
UPPER_TWIST = 55.0    # quanto do giro "palma para a frente" fica no ombro; o resto fica no antebraço
BLEND = 14            # quadros em que os braços saem das mãos para o alto e entram na queda

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=src)
sc = bpy.context.scene
arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')

# ---------- limpeza ----------
def role_of(o):
    for key, role, tris in PARTS:
        if key.lower() in o.name.lower(): return role, tris
    return 'body', None
for o in list(bpy.data.objects):
    if o.type == 'MESH' and role_of(o)[0] == 'drop': bpy.data.objects.remove(o)
    elif o.type not in ('MESH', 'ARMATURE'): bpy.data.objects.remove(o)
for o in bpy.data.objects:
    if o.animation_data: o.animation_data_clear()
    if o.type == 'MESH' and o.data.shape_keys: o.shape_key_clear()
for act in list(bpy.data.actions): bpy.data.actions.remove(act)
for pb in arm.pose.bones:
    pb.rotation_mode = 'QUATERNION'
    pb.location = (0, 0, 0); pb.rotation_quaternion = (1, 0, 0, 0); pb.scale = (1, 1, 1)
sc.frame_set(1)
bpy.context.view_layer.update()
meshes = [o for o in bpy.data.objects if o.type == 'MESH']

# altura: a mesma medida do conv_hostage (caixa de todas as malhas em repouso, do salto ao topo do cabelo)
zs = [(o.matrix_world @ Vector(c)).z for o in meshes for c in o.bound_box]
k = HEIGHT / (max(zs) - min(zs))
arm.scale = [s * k for s in arm.scale]
arm.location.z -= min(zs) * k
bpy.context.view_layer.update()

# ---------- pose ----------
def bone(name): return next(pb for pb in arm.pose.bones if pb.name.split(':')[-1] == name)
def world(pb):
    bpy.context.view_layer.update()
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
    pb.location, pb.rotation_quaternion, pb.scale = basis.decompose()
    pb.location = (0, 0, 0); pb.scale = (1, 1, 1)
    bpy.context.view_layer.update()
def spin(pb, R):
    M = world(pb); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ R.to_4x4() @ Matrix.Translation(-h) @ M)
def point(pb, child, d):
    # gira o osso (em torno da própria junta) até a direção junta -> junta seguinte ficar igual a d
    cur = (world(bone(child)).translation - world(pb).translation).normalized()
    spin(pb, cur.rotation_difference(Vector(d).normalized()).to_matrix())
def twist(pb, deg, axis='Z'):
    # gira em torno de um eixo do mundo (ou de um vetor) passando pela junta
    if isinstance(axis, str): spin(pb, Matrix.Rotation(radians(deg), 3, axis))
    else: spin(pb, Matrix.Rotation(radians(deg), 3, Vector(axis).normalized()))
def snapshot(): return {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in arm.pose.bones}
def restore(pose):
    for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = pose[pb.name]
    bpy.context.view_layer.update()
def key_all(frame):
    for pb in arm.pose.bones:
        for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=frame)
def sway(name, pose):
    # o tronco e a cabeça viram um pouco para um lado e para o outro, em laço (igual ao conv_hostage)
    act = bpy.data.actions.new(name); arm.animation_data.action = act
    for frame, waist, head in ((1, 0, 0), (63, 2.5, 9), (125, 0, 0), (188, -2.5, -9), (250, 0, 0)):
        restore(pose)
        if waist: twist(bone('Spine'), waist); twist(bone('Head'), head)
        key_all(frame)
    return act

t_pose = snapshot()
arm.animation_data_create()
# mãos para o alto, palmas abertas viradas para a frente (ela olha para -Y; o braço esquerdo dela fica em +X).
# Na pose T a palma olha para o chão: o braço gira no próprio eixo para a palma ir para a frente, parte no ombro e parte
# no antebraço (tudo num osso só torce a manga).
for s, sx in (('Right', -1.0), ('Left', 1.0)):
    twist(bone(s + 'Arm'), -UPPER_TWIST, 'X')
    point(bone(s + 'Arm'), s + 'ForeArm', (sx * 0.88, -0.10, 0.22))
    point(bone(s + 'ForeArm'), s + 'Hand', (sx * 0.16, -0.14, 1.0))
    fa = world(bone(s + 'Hand')).translation - world(bone(s + 'ForeArm')).translation
    twist(bone(s + 'ForeArm'), -sx * (90.0 - UPPER_TWIST), fa)
    point(bone(s + 'Hand'), s + 'HandMiddle1', (sx * 0.04, -0.06, 1.0))
base = snapshot()
idle = sway('Idle', base)
clips = [('Idle', idle)]

# quedas: as quatro animações do Mixamo; os ossos têm o mesmo nome nos dois esqueletos (só muda o prefixo)
restore(t_pose)
tmap = {pb.name.split(':')[-1]: pb for pb in arm.pose.bones}
MXDIR = mixamo_lib.folder(a[2] if len(a) > 2 else None)
ARMS = [bone(s + n) for s in ('Left', 'Right') for n in ('Shoulder', 'Arm', 'ForeArm', 'Hand')]
for v, f in mixamo_lib.FALLS:
    act, n = mixamo_lib.retarget(os.path.join(MXDIR, f + '.fbx'), arm, tmap, v, travel=0.6)
    arm.animation_data.action = act
    for fr in range(1, BLEND + 1):
        sc.frame_set(fr)
        w = (fr - 1) / BLEND; w = w * w * (3 - 2 * w)
        for pb in ARMS:
            q0 = base[pb.name][1].copy()
            if q0.dot(pb.rotation_quaternion) < 0: q0.negate()
            pb.rotation_quaternion = q0.slerp(pb.rotation_quaternion, w)
            pb.keyframe_insert('rotation_quaternion', frame=fr)
    sc.frame_set(int(act.frame_range[1]))
    hp, he = world(bone('Hips')).translation, world(bone('Head')).translation
    print("FIMQUEDA %s: %d quadros, quadril (%.2f, %.2f, z %.2f) cabeça (%.2f, %.2f, z %.2f)" % (v, n, hp.x, hp.y, hp.z, he.x, he.y, he.z))
    clips.append((v, act))
sc.frame_set(1)

# braços abaixados: depois que o cômodo dela fica limpo, a refém baixa os braços (mesmo balanço da espera)
arm.animation_data.action = None
restore(t_pose)
for s, sx in (('Right', -1.0), ('Left', 1.0)):
    point(bone(s + 'Arm'), s + 'ForeArm', (sx * 0.20, 0.02, -1.0))
    point(bone(s + 'ForeArm'), s + 'Hand', (sx * 0.10, -0.16, -1.0))
    point(bone(s + 'Hand'), s + 'HandMiddle1', (sx * 0.04, -0.10, -1.0))
    # dedos um pouco fechados (mão solta); o eixo de dobra é o que, na pose T, fecha a mão para baixo
    for fg in ('Index', 'Middle', 'Ring', 'Pinky'):
        for i_ in (1, 2, 3):
            try: pb = bone('%sHand%s%d' % (s, fg, i_))
            except StopIteration: continue
            M = world(pb); nxt = pb.children[0] if pb.children else None
            if not nxt: continue
            d = (world(nxt).translation - M.translation).normalized()
            palm = (world(bone(s + 'Hand')).to_3x3() @ (bone(s + 'Hand').bone.matrix_local.to_3x3().inverted() @ (arm.matrix_world.to_3x3().inverted() @ Vector((0, 0, -1))))).normalized()
            ax = d.cross(palm)
            if ax.length > 1e-4: twist(pb, 16.0, ax)
relaxed = snapshot()
clips.append(('Relax', sway('Relax', relaxed)))

# os clipes vão para o arquivo cada um numa faixa com o próprio nome
arm.animation_data.action = None
restore(t_pose)
for name, act in clips:
    tr = arm.animation_data.nla_tracks.new(); tr.name = name; tr.strips.new(name, 1, act)
sc.frame_start, sc.frame_end = 1, 250
sc.render.fps = mixamo_lib.FPS
sc.frame_set(1)

# ---------- malha ----------
def tri_count(o): return sum(len(p.vertices) - 2 for p in o.data.polygons)
total = 0
for o in meshes:
    role, t = role_of(o)
    tris = tri_count(o)
    if t and tris > t * 1.1:
        if o.data.users > 1: o.data = o.data.copy()
        d = o.modifiers.new('dec', 'DECIMATE'); d.ratio = t / tris
        with bpy.context.temp_override(object=o, active_object=o, selected_objects=[o]):
            while o.modifiers[0].name != 'dec': bpy.ops.object.modifier_move_up(modifier='dec')
            bpy.ops.object.modifier_apply(modifier='dec')
    print("MALHA %-16s %-6s %6d -> %6d" % (o.name, role, tris, tri_count(o)))
    total += tri_count(o)
print("TRIS", total)

# ---------- materiais ----------
def principled(m): return next((n for n in m.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None) if m and m.use_nodes else None
def tex_of(sock, depth=0):
    for l in sock.links:
        n = l.from_node
        if n.type == 'TEX_IMAGE' and n.image: return n
        if depth < 4:
            for i_ in n.inputs:
                r = tex_of(i_, depth + 1)
                if r: return r
    return None
def pixels(img):
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32); img.pixels.foreach_get(px)
    return px.reshape(h, w, 4)
def new_image(name, px):
    h, w = px.shape[:2]
    img = bpy.data.images.new(name, w, h, alpha=True)
    img.pixels.foreach_set(np.ascontiguousarray(px, dtype=np.float32).reshape(-1)); img.update()
    return img

# texturas em 1024 e sem brilho: cor e relevo ficam; as texturas de brilho/especular saem
for m in bpy.data.materials:
    p = principled(m)
    if not p: continue
    nt = m.node_tree
    for name in ('Roughness', 'Metallic', 'Specular IOR Level', 'Specular Tint', 'Transmission Weight', 'Subsurface Weight', 'Coat Weight', 'Sheen Weight', 'Emission Color', 'Emission Strength'):
        if name in p.inputs:
            for l in list(p.inputs[name].links): nt.links.remove(l)
    p.inputs['Roughness'].default_value = ROUGH
    p.inputs['Metallic'].default_value = 0.0
    p.inputs['Specular IOR Level'].default_value = 0.25
    for name in ('Transmission Weight', 'Subsurface Weight', 'Coat Weight', 'Sheen Weight', 'Emission Strength'):
        if name in p.inputs: p.inputs[name].default_value = 0.0
    used = {tex_of(p.inputs['Base Color']), tex_of(p.inputs['Alpha']), tex_of(p.inputs['Normal'])}
    for n in list(nt.nodes):
        if n.type == 'TEX_IMAGE' and n not in used: nt.nodes.remove(n)
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image and n.image.size[0] > TEX:
            n.image.scale(TEX, TEX * n.image.size[1] // n.image.size[0])
    m.use_backface_culling = False

# cada peça com o material do seu papel. Blazer, calça e cabelo ganham uma cópia da textura em cinza claro (a média da
# peça vai para 0,78): a cor de cada refém é dada pelo jogo.
made = {}
for o in meshes:
    role, _ = role_of(o)
    m0 = o.data.materials[0]
    key = (role, m0.name)
    if key not in made:
        m = m0.copy(); m.name = MAT_NAME[role]
        p = principled(m); nt = m.node_tree
        bn = tex_of(p.inputs['Base Color'])
        if role == 'hair':
            m.blend_method = 'BLEND'
            if bn:
                for l in list(p.inputs['Alpha'].links): nt.links.remove(l)
                nt.links.new(bn.outputs['Alpha'], p.inputs['Alpha'])
        else:
            for l in list(p.inputs['Alpha'].links): nt.links.remove(l)
            p.inputs['Alpha'].default_value = 1.0; m.blend_method = 'OPAQUE'
        if role in ('top', 'pants', 'hair') and bn:
            px = pixels(bn.image).copy(); h, w = px.shape[:2]
            lum = px[:, :, 0] * 0.2126 + px[:, :, 1] * 0.7152 + px[:, :, 2] * 0.0722
            # média só dos pontos da textura que esta peça usa (a textura é dividida com o resto do corpo)
            uv = np.empty(len(o.data.loops) * 2, dtype=np.float32); o.data.uv_layers.active.data.foreach_get('uv', uv); uv = uv.reshape(-1, 2)
            xi = np.clip((np.mod(uv[:, 0], 1.0) * w).astype(int), 0, w - 1); yi = np.clip((np.mod(uv[:, 1], 1.0) * h).astype(int), 0, h - 1)
            samp = lum[yi, xi]; solid = px[yi, xi, 3] > 0.05
            mean = float(samp[solid].mean()) if solid.any() else float(samp.mean())
            # cabelo escuro: clarear na proporção direta deixa os fios com cara de metal; a raiz quadrada segura o contraste
            v = np.clip((lum / max(mean, 1e-4)) ** (0.5 if role == 'hair' else 1.0) * 0.78, 0.0, 1.0)
            for ch in range(3): px[:, :, ch] = v
            bn.image = new_image(m.name + '_grey', px)
            print("CINZA %s (média original %.3f)" % (m.name, mean))
        made[key] = m
    o.data.materials[0] = made[key]
for m in list(bpy.data.materials):
    if m.users == 0: bpy.data.materials.remove(m)

bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=84, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True, export_morph=False)

# pós-exportação (igual ao conv_hostage): cabelo com recorte (MASK) em vez de mistura; nada de extensões de brilho
raw = open(out, 'rb').read()
ln = struct.unpack_from('<I', raw, 12)[0]
g = json.loads(raw[20:20 + ln]); rest = raw[20 + ln:]
for m in g['materials']:
    if m['name'] == MAT_NAME['hair']: m['alphaMode'] = 'MASK'; m['alphaCutoff'] = 0.18
    else: m.pop('alphaMode', None); m.pop('alphaCutoff', None)
    m['doubleSided'] = True
    m.pop('extensions', None)
    pbr = m.setdefault('pbrMetallicRoughness', {})
    pbr.pop('metallicRoughnessTexture', None)
    pbr['metallicFactor'] = 0.0
    pbr['roughnessFactor'] = ROUGH
g['extensionsUsed'] = [e for e in g.get('extensionsUsed', []) if not e.startswith('KHR_materials')]
if not g['extensionsUsed']: g.pop('extensionsUsed')
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest)
print("EXPORTADO", out, os.path.getsize(out))
