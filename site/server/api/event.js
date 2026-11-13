// Uso do jogo, sem dado pessoal: cada instalação tem um identificador sorteado. Guarda contagens por dia.
import { readJson, writeJson, body, clean, day } from './_store.js';

const KINDS = new Set(['launch', 'start', 'end']);
export default async function handler(req, res) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  if (req.method === 'OPTIONS') { res.setHeader('Access-Control-Allow-Headers', 'content-type'); return res.status(204).end(); }
  if (req.method !== 'POST') return res.status(405).json({ error: 'method' });
  const b = await body(req);
  const kind = KINDS.has(b.kind) ? b.kind : null;
  const id = clean(b.id, /^[a-f0-9]+$/, 40);
  const v = clean(b.v, /^[0-9.]+$/, 12) || '?';
  const board = b.board ? clean(b.board, /^[\w ·:.'()\-]+$/u, 60) : null;
  if (!kind || !id) return res.status(400).json({ error: 'invalid' });
  if (v === '0.0') return res.status(200).json({ ok: true, test: true });   // testes automáticos não contam
  const stats = await readJson('stats.json', { days: {}, installs: {} });
  const d = stats.days[day()] ||= { launch: 0, start: 0, end: 0, ids: [], boards: {}, versions: {}, secs: 0 };
  d[kind] += 1;
  if (!d.ids.includes(id)) d.ids.push(id);
  d.versions[v] = (d.versions[v] || 0) + (kind === 'launch' ? 1 : 0);
  if (board) { const bb = d.boards[board] ||= { start: 0, end: 0, passed: 0 }; if (kind !== 'launch') bb[kind] += 1; if (kind === 'end' && b.passed) bb.passed += 1; }
  if (kind === 'end' && Number.isFinite(Number(b.time))) d.secs += Math.min(Math.max(Number(b.time), 0), 3600);
  if (!stats.installs[id]) stats.installs[id] = day();
  await writeJson('stats.json', stats);
  return res.status(200).json({ ok: true });
}
