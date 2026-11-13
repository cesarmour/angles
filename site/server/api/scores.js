// Ranking global e perpétuo. GET: os melhores de cada tabela. POST: registra uma partida.
import { put } from '@vercel/blob';
import { readJson, writeJson, body, clean } from './_store.js';

const TOP = 100;
const order = (a, b) => (b.score - a.score) || (a.time - b.time) || (a.ts - b.ts);

export default async function handler(req, res) {
  res.setHeader('Access-Control-Allow-Origin', '*');
  if (req.method === 'OPTIONS') { res.setHeader('Access-Control-Allow-Headers', 'content-type'); return res.status(204).end(); }
  if (req.method === 'GET') {
    const boards = await readJson('boards.json', {});
    const n = Math.min(parseInt(req.query?.n || '10', 10) || 10, TOP);
    const out = {};
    for (const [b, rows] of Object.entries(boards)) if (!b.startsWith('__')) out[b] = rows.slice(0, n).map(({ name, score, time, diff, ts, v }) => ({ name, score, time, diff, ts, v }));
    res.setHeader('Cache-Control', 'public, s-maxage=20, stale-while-revalidate=60');
    return res.status(200).json({ boards: out });
  }
  if (req.method !== 'POST') return res.status(405).json({ error: 'method' });
  const b = await body(req);
  const board = clean(b.board, /^[\w ·:.'()\-]+$/u, 60);
  const name = clean((b.name || '').toUpperCase(), /^[A-Z0-9 _\-]+$/, 14);
  const id = clean(b.id, /^[a-f0-9]+$/, 40);
  const v = clean(b.v, /^[0-9.]+$/, 12) || '?';
  const score = Number(b.score), time = Number(b.time), diff = Number(b.diff);
  if (!board || !name || !id || !Number.isFinite(score) || !Number.isFinite(time) || score < -5000 || score > 100000 || time < 1 || time > 36000 || !(diff >= 0 && diff <= 3)) return res.status(400).json({ error: 'invalid' });
  const row = { name: name.trim() || 'OPERATOR', score: Math.round(score), time: Math.round(time * 10) / 10, diff, ts: Date.now(), v, id };
  // registro bruto, um arquivo por partida: é o histórico perpétuo, de onde as tabelas podem ser refeitas
  await put(`raw/${board.replace(/[^\w]+/g, '_')}/${row.ts}-${Math.random().toString(36).slice(2, 8)}.json`, JSON.stringify({ board, ...row }), { access: 'private', addRandomSuffix: false, contentType: 'application/json' });
  const boards = await readJson('boards.json', {});
  const rows = boards[board] || [];
  rows.push(row); rows.sort(order);
  const rank = rows.indexOf(row) + 1;
  boards[board] = rows.slice(0, TOP);
  await writeJson('boards.json', boards);
  const totals = await readJson('totals.json', {});
  totals[board] = (totals[board] || 0) + 1;
  await writeJson('totals.json', totals);
  return res.status(200).json({ rank: rank <= TOP ? rank : null, total: totals[board] });
}
