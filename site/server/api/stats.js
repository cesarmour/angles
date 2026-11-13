// Resumo do uso para o painel: por dia, quantas instalações jogaram, quantas partidas, tempo de jogo, por etapa.
import { readJson } from './_store.js';
export default async function handler(req, res) {
  const stats = await readJson('stats.json', { days: {}, installs: {} });
  const totals = await readJson('totals.json', {});
  const days = Object.entries(stats.days).sort().map(([date, d]) => ({ date, players: d.ids.length, launches: d.launch, runs_started: d.start, runs_finished: d.end, minutes: Math.round(d.secs / 60), boards: d.boards, versions: d.versions,
    new_players: Object.values(stats.installs).filter(x => x === date).length }));
  res.setHeader('Cache-Control', 'no-store');
  return res.status(200).json({ players_total: Object.keys(stats.installs).length, scores_by_board: totals, days });
}
