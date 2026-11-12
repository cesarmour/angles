"""Refém: "Women in jeans" (aribb, CC BY-NC 4.0) -> assets/models/hostage.glb
Civil de rosto à mostra, mãos para o alto, com o clipe "Idle" (olha para os lados) e quatro quedas ("Fall1" a "Fall4").

  blender -b --python tools3d/conv_hostage.py -- <women_in_jeans.glb> <saida.glb>

O que muda em relação ao original: sai a pose de vitrine (o clipe de um quadro só), entram a pose de mãos para o alto e o
clipe de espera feitos aqui; saem as formas de expressão facial e as peças miúdas do rosto; malha e texturas reduzidas;
altura ajustada para 1,70 m.
"""
import bpy, sys, os, re, json, struct
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pose_lib
from math import radians
from mathutils import Vector, Matrix

a = sys.argv[sys.argv.index('--') + 1:]
src, out = a[0], a[1]
bpy.ops.wm.read_factory_settings(use_empty=True)
# ossos criados a partir da posição dos nós do arquivo: com a opção padrão (adivinhar a pose de ligação) vários ossos
# deste modelo nascem na origem e qualquer giro sai em torno do chão
bpy.ops.import_scene.gltf(filepath=src, guess_original_bind_pose=False)
sc = bpy.context.scene
arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')

# ---------- limpeza ----------
DROP = ('Std_Tongue', 'Std_Upper_Teeth', 'Std_Lower_Teeth', 'Std_Tearline_R', 'Std_Tearline_L', 'Std_Eye_Occlusion_R', 'Std_Eye_Occlusion_L',
        'Std_Cornea_R', 'Std_Cornea_L', 'Std_Eyelash')
for o in list(bpy.data.objects):
    if o.name.startswith('Icosphere'): bpy.data.objects.remove(o); continue
    if o.type == 'MESH' and o.data.materials and o.data.materials[0] and o.data.materials[0].name in DROP: bpy.data.objects.remove(o)
for o in bpy.data.objects:
    if o.animation_data: o.animation_data_clear()
    if o.type == 'MESH' and o.data.shape_keys: o.shape_key_clear()
for act in list(bpy.data.actions): bpy.data.actions.remove(act)
for pb in arm.pose.bones:
    pb.location = (0, 0, 0); pb.rotation_quaternion = (1, 0, 0, 0); pb.scale = (1, 1, 1)
sc.frame_set(1)
bpy.context.view_layer.update()

# altura 1,70 m (o arquivo vem com 3,48)
top = arm
while top.parent: top = top.parent
bpy.context.view_layer.update()
zs = [(o.matrix_world @ Vector(c)).z for o in bpy.data.objects if o.type == 'MESH' for c in o.bound_box]
k = 1.70 / (max(zs) - min(zs))
top.scale = [s * k for s in top.scale]
bpy.context.view_layer.update()

# ---------- pose ----------
def bone(name): return next(pb for pb in arm.pose.bones if re.fullmatch(r'CC_Base_' + name + r'_\d+', pb.name))
def world(pb):
    # lê a pose já avaliada: logo depois de importar, parte dos ossos ainda não tem a matriz calculada no objeto original
    bpy.context.view_layer.update()
    dg = bpy.context.evaluated_depsgraph_get()
    return arm.matrix_world @ arm.evaluated_get(dg).pose.bones[pb.name].matrix
def set_world(pb, M):
    # converte a matriz desejada (mundo) para a pose local do osso, usando o pai já avaliado
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
    bpy.context.view_layer.update()
def point(pb, d):
    M = world(pb); cur = (M.to_3x3() @ Vector((0, 1, 0))).normalized()
    R = cur.rotation_difference(Vector(d).normalized()).to_matrix().to_4x4(); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ R @ Matrix.Translation(-h) @ M)
def twist(pb, deg):
    M = world(pb); h = M.translation.copy()
    set_world(pb, Matrix.Translation(h) @ Matrix.Rotation(radians(deg), 4, 'Z') @ Matrix.Translation(-h) @ M)

# mãos para o alto, palmas abertas (ela olha para -Y; o braço esquerdo dela fica em +X)
for s, sx in (('R', -1.0), ('L', 1.0)):
    point(bone(s + '_Upperarm'), (sx * 0.88, -0.10, 0.22))
    point(bone(s + '_Forearm'), (sx * 0.16, -0.14, 1.0))
    point(bone(s + '_Hand'), (sx * 0.04, -0.06, 1.0))
base = {pb.name: (pb.location.copy(), pb.rotation_quaternion.copy(), pb.scale.copy()) for pb in arm.pose.bones}
arm.animation_data_create()
arm.animation_data.action = bpy.data.actions.new('Idle')
# espera: o tronco e a cabeça viram um pouco para um lado e para o outro, em laço
for frame, waist, head in ((1, 0, 0), (50, 2.5, 9), (100, 0, 0), (150, -2.5, -9), (200, 0, 0)):
    for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = base[pb.name]
    bpy.context.view_layer.update()
    if waist: twist(bone('Waist'), waist); twist(bone('Head'), head)
    for pb in arm.pose.bones:
        for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=frame)
idle = arm.animation_data.action
# queda: joelhos dobram e o corpo desaba para a frente (em vez de tombar rígido)
CC = {'hips': 'Hip', 'spine0': 'Waist', 'spine1': 'Spine01', 'spine2': 'Spine02', 'neck': 'NeckTwist01', 'head': 'Head',
      'thigh_l': 'L_Thigh', 'thigh_r': 'R_Thigh', 'calf_l': 'L_Calf', 'calf_r': 'R_Calf', 'arm_l': 'L_Upperarm', 'arm_r': 'R_Upperarm', 'fore_l': 'L_Forearm', 'fore_r': 'R_Forearm'}
def restore():
    for pb in arm.pose.bones: pb.location, pb.rotation_quaternion, pb.scale = base[pb.name]
    bpy.context.view_layer.update()
def key_all(frame):
    for pb in arm.pose.bones:
        for path in ('location', 'rotation_quaternion', 'scale'): pb.keyframe_insert(path, frame=frame)
clips = [('Idle', idle)]
for v in pose_lib.VARIANTS:
    act = bpy.data.actions.new(v); arm.animation_data.action = act
    pose_lib.make_fall(lambda k: bone(CC[k]), world, set_world, point, restore, key_all, v)
    clips.append((v, act))
# os clipes vão para o arquivo cada um numa faixa com o próprio nome
arm.animation_data.action = None
for name, act in clips:
    tr = arm.animation_data.nla_tracks.new(); tr.name = name; tr.strips.new(name, 1, act)
sc.frame_start, sc.frame_end = 1, 200
sc.frame_set(1)

# ---------- malha ----------
TARGET = {'Top_Mat': 6500, 'Pants_Mat': 3600, 'hair_Transparency': 9000, 'Std_Skin_Head': 5200, 'Std_Skin_Arm': 3000, 'High_Heels': 900}
total = 0
for o in [o for o in bpy.data.objects if o.type == 'MESH']:
    tris = sum(len(p.vertices) - 2 for p in o.data.polygons)
    mat = o.data.materials[0].name if o.data.materials and o.data.materials[0] else ''
    t = TARGET.get(mat)
    if t and tris > t * 1.2:
        if o.data.users > 1: o.data = o.data.copy()
        d = o.modifiers.new('dec', 'DECIMATE'); d.ratio = t / tris
        with bpy.context.temp_override(object=o, active_object=o, selected_objects=[o]):
            while o.modifiers[0].name != 'dec': bpy.ops.object.modifier_move_up(modifier='dec')
            bpy.ops.object.modifier_apply(modifier='dec')
        tris = sum(len(p.vertices) - 2 for p in o.data.polygons)
    total += tris
print("TRIS", total)

# ---------- materiais ----------
HAIR = ('hair_Transparency', 'scalp_Transparency', 'Front_Transparency')
SIZE = {'Std_Skin_Head': 1024, 'Top_Mat': 1024, 'Pants_Mat': 1024, 'hair_Transparency': 1024}
for m in bpy.data.materials:
    if not m.use_nodes: continue
    nt = m.node_tree
    p = next((n for n in nt.nodes if n.type == 'BSDF_PRINCIPLED'), None)
    if not p: continue
    for name in ('Transmission Weight', 'Subsurface Weight', 'Coat Weight', 'Sheen Weight'):
        if name in p.inputs:
            for l in list(p.inputs[name].links): nt.links.remove(l)
            p.inputs[name].default_value = 0.0
    if m.name in HAIR: m.blend_method = 'BLEND'
    else:
        for l in list(p.inputs['Alpha'].links): nt.links.remove(l)
        p.inputs['Alpha'].default_value = 1.0; m.blend_method = 'OPAQUE'
    for n in nt.nodes:
        if n.type == 'TEX_IMAGE' and n.image:
            s = SIZE.get(m.name, 512)
            if n.image.size[0] > s: n.image.scale(s, s * n.image.size[1] // n.image.size[0])
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=84, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True, export_morph=False)

# pós-exportação: cabelo com recorte (MASK) em vez de mistura; tecidos e pele sem brilho forte
raw = open(out, 'rb').read()
ln = struct.unpack_from('<I', raw, 12)[0]
g = json.loads(raw[20:20 + ln]); rest = raw[20 + ln:]
for m in g['materials']:
    if m['name'] in HAIR: m['alphaMode'] = 'MASK'; m['alphaCutoff'] = 0.6 if m['name'] == 'scalp_Transparency' else 0.18; m['doubleSided'] = True
    else: m.pop('alphaMode', None)
    m.pop('extensions', None)
    pbr = m.setdefault('pbrMetallicRoughness', {})
    pbr['metallicFactor'] = 0.0
    if 'metallicRoughnessTexture' not in pbr: pbr['roughnessFactor'] = max(pbr.get('roughnessFactor', 0.8), 0.7)
g['extensionsUsed'] = [e for e in g.get('extensionsUsed', []) if not e.startswith('KHR_materials')]
if not g['extensionsUsed']: g.pop('extensionsUsed')
js = json.dumps(g, separators=(',', ':')).encode()
while len(js) % 4: js += b' '
with open(out, 'wb') as w:
    w.write(struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(js) + len(rest)))
    w.write(struct.pack('<II', len(js), 0x4E4F534A)); w.write(js); w.write(rest)
print("EXPORTADO", out, os.path.getsize(out))
