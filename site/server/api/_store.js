// Armazenamento do ranking e do uso do jogo: arquivos JSON num Blob privado da Vercel.
import { put, get } from '@vercel/blob';

export async function readJson(path, fallback) {
  try {
    const r = await get(path, { access: 'private', useCache: false });
    if (!r || r.statusCode !== 200) return fallback;
    return JSON.parse(await new Response(r.stream).text());
  } catch (e) { return fallback; }
}
export async function writeJson(path, data) {
  await put(path, JSON.stringify(data), { access: 'private', addRandomSuffix: false, allowOverwrite: true, contentType: 'application/json' });
}
export async function body(req) {
  if (req.body && typeof req.body === 'object') return req.body;
  if (typeof req.body === 'string') { try { return JSON.parse(req.body); } catch (e) { return {}; } }
  const chunks = []; for await (const c of req) chunks.push(c);
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'); } catch (e) { return {}; }
}
export const clean = (s, re, max) => (typeof s === 'string' && s.length <= max && re.test(s)) ? s : null;
export const day = () => new Date().toISOString().slice(0, 10);
