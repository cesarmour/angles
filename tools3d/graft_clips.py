#!/usr/bin/env python3
"""Copia clipes de animação de um GLB para outro que tenha o mesmo esqueleto (mesmos nomes de osso e mesma pose de repouso).
Usado para dar ao corpo do jogador (player.glb) as quedas e o tranco de tiro levado que a ameaça (threat.glb) já tem:
os dois saem do mesmo boneco pelo conv_operator.py.

  python3 tools3d/graft_clips.py assets/models/threat.glb assets/models/player.glb Fall1 Fall2 Fall3 Fall4 Hit

Não usa o Blender: mexe direto no arquivo. Canais de ossos que não existem no destino ficam de fora (e são listados)."""
import json, struct, sys
src, dst, names = sys.argv[1], sys.argv[2], sys.argv[3:]
def load(f):
    d = open(f, 'rb').read(); n = struct.unpack('<I', d[12:16])[0]
    return json.loads(d[20:20 + n]), bytearray(d[20 + n + 8:])
S, sb = load(src); D, db = load(dst)
dn = {n['name']: i for i, n in enumerate(D['nodes'])}
have = {a['name'] for a in D.get('animations', [])}
SIZE = {'SCALAR': 4, 'VEC3': 12, 'VEC4': 16}
def copy_acc(i, cache={}):
    if i in cache: return cache[i]
    a = dict(S['accessors'][i]); bv = S['bufferViews'][a['bufferView']]
    off = bv.get('byteOffset', 0) + a.get('byteOffset', 0); n = a['count'] * SIZE[a['type']]
    assert bv.get('byteStride') in (None, SIZE[a['type']])
    while len(db) % 4: db.append(0)
    D['bufferViews'].append({'buffer': 0, 'byteOffset': len(db), 'byteLength': n})
    db.extend(sb[off:off + n])
    a['bufferView'] = len(D['bufferViews']) - 1; a.pop('byteOffset', None)
    D['accessors'].append(a); cache[i] = len(D['accessors']) - 1
    return cache[i]
for name in names:
    if name in have: print(name, 'já existe no destino: pulado'); continue
    a = next(x for x in S['animations'] if x['name'] == name)
    out = {'name': name, 'samplers': [], 'channels': []}; miss = set()
    for c in a['channels']:
        nm = S['nodes'][c['target']['node']]['name']
        if nm not in dn: miss.add(nm); continue
        s = a['samplers'][c['sampler']]
        out['samplers'].append({'input': copy_acc(s['input']), 'output': copy_acc(s['output']), 'interpolation': s.get('interpolation', 'LINEAR')})
        out['channels'].append({'sampler': len(out['samplers']) - 1, 'target': {'node': dn[nm], 'path': c['target']['path']}})
    D.setdefault('animations', []).append(out)
    print(name, len(out['channels']), 'canais', ('sem ' + ', '.join(sorted(miss))) if miss else '')
while len(db) % 4: db.append(0)
D['buffers'][0]['byteLength'] = len(db)
js = json.dumps(D, separators=(',', ':')).encode()
js += b' ' * (-len(js) % 4)
open(dst, 'wb').write(b'glTF' + struct.pack('<II', 2, 12 + 8 + len(js) + 8 + len(db)) + struct.pack('<I', len(js)) + b'JSON' + js + struct.pack('<I', len(db)) + b'BIN\x00' + bytes(db))
