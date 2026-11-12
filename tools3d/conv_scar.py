"""SCAR (JUST, CC-BY) FBX -> assets/models/scar.glb
blender -b --python tools3d/conv_scar.py -- <pasta do zip extraído> <saida.glb>"""
import bpy, sys, os
a = sys.argv[sys.argv.index('--') + 1:]
root, out = a[0], a[1]
tex = os.path.join(root, 'textures')
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.fbx(filepath=os.path.join(root, 'source', 'SCAR.fbx'))
for o in list(bpy.data.objects):
    if o.type in ('CAMERA', 'LIGHT') or o.name.startswith('shape_') or o.name == 'crosshair':
        bpy.data.objects.remove(o)
files = {f.lower(): f for f in os.listdir(tex)}
def load(name):
    return bpy.data.images.load(os.path.join(tex, name))
def shrink(img, n):
    if img.size[0] > n:
        img.scale(n, n * img.size[1] // img.size[0])
def setup(mat, base, normal=None, rough=None, metal=None, size=1024, colour=None):
    mat.use_nodes = True
    nt = mat.node_tree
    for n in list(nt.nodes): nt.nodes.remove(n)
    o = nt.nodes.new('ShaderNodeOutputMaterial'); p = nt.nodes.new('ShaderNodeBsdfPrincipled')
    nt.links.new(p.outputs[0], o.inputs[0])
    if colour: p.inputs['Base Color'].default_value = colour
    p.inputs['Roughness'].default_value = 0.75
    if base:
        i = nt.nodes.new('ShaderNodeTexImage'); i.image = load(base); shrink(i.image, size)
        nt.links.new(i.outputs[0], p.inputs['Base Color'])
    if normal:
        i = nt.nodes.new('ShaderNodeTexImage'); i.image = load(normal); i.image.colorspace_settings.name = 'Non-Color'; shrink(i.image, size)
        nm = nt.nodes.new('ShaderNodeNormalMap'); nt.links.new(i.outputs[0], nm.inputs['Color']); nt.links.new(nm.outputs[0], p.inputs['Normal'])
    if rough:
        i = nt.nodes.new('ShaderNodeTexImage'); i.image = load(rough); i.image.colorspace_settings.name = 'Non-Color'; shrink(i.image, size)
        nt.links.new(i.outputs[0], p.inputs['Roughness'])
    if metal:
        i = nt.nodes.new('ShaderNodeTexImage'); i.image = load(metal); i.image.colorspace_settings.name = 'Non-Color'; shrink(i.image, size)
        nt.links.new(i.outputs[0], p.inputs['Metallic'])
    mat.blend_method = 'OPAQUE'
M = bpy.data.materials
setup(M['Material'], 'SCARL_LPForNormalFix_Material_AlbedoTransp.png', 'SCARL_LPForNormalFix_Material_NormalOpenGL.png', 'SCARL_LPForNormalFix_Material_Roughness.png', 'SCARL_LPForNormalFix_Material_MetallicSmoo.png', 2048)
setup(M['glove_hardknuckle'], 'glove_hardknuckle_Base_color.png', 'glove_hardknuckle_Normal_OpenGL.png', None, None, 1024)
setup(M['sleeve_st6_generalist'], 'sleeve_st6_generalist_Base_color.png', 'sleeve_st6_generalist_Normal_OpenGL.png', None, None, 1024)
setup(M['v_l_skin'], None, colour=(0.02, 0.02, 0.02, 1))
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
arm.animation_data_create()
for tr in list(arm.animation_data.nla_tracks): arm.animation_data.nla_tracks.remove(tr)
arm.animation_data.action = None
for act in list(bpy.data.actions):
    if not act.name.startswith('Armature|'):
        bpy.data.actions.remove(act); continue
    name = act.name.split('|')[-1]
    tr = arm.animation_data.nla_tracks.new(); tr.name = name
    tr.strips.new(name, int(act.frame_range[0]), act)
    print("CLIPE", name, tuple(act.frame_range))
bpy.ops.export_scene.gltf(filepath=out, export_format='GLB', export_animations=True, export_animation_mode='NLA_TRACKS', export_skins=True,
    export_image_format='JPEG', export_jpeg_quality=85, export_materials='EXPORT', export_cameras=False, export_lights=False, export_yup=True)
print("EXPORTADO", out, os.path.getsize(out))
