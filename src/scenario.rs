//! Cenários (cada um treina uma tática), inimigos que se deslocam pela casa, ondas de invasores e ranking.
use crate::lessons::LESSONS;
use crate::map::*;
use crate::state::*;
use crate::tactical::Tac;
use bevy::prelude::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Shield, Hallway, Threshold, Rear, Locked, Mind, Rescue, Defense, Walker }

pub struct Scenario {
    pub name: &'static str, pub kicker: &'static str, pub blurb: &'static str, pub lead: &'static str, pub points: &'static [&'static str],
    /// Mapa em que roda (índice em `map::MAPS`) e se exige bonecos mesmo no nível EASY (alvo de papelão não anda nem se rende).
    pub map: usize, pub kind: Kind, pub figs: bool,
}

pub const SCENARIOS: [Scenario; 9] = [
    Scenario { name: "Hostage shield", kicker: "SCENARIO 1 · PRECISION SHOT", map: 0, kind: Kind::Shield, figs: false,
        blurb: "three threats stand behind a hostage: open the angle before you fire",
        lead: "Three of the threats in this house are holding a hostage in front of them. From the door you see a sliver of the threat and a lot of hostage.",
        points: &["Do not take the tight shot. Step sideways until the threat's head or chest is clear of the hostage, then fire.",
            "The threat still fires over her shoulder: change the angle from behind the door frame, not from the middle of the room.",
            "Hitting a hostage costs 20 points. A clean run drops all three shielded threats with every hostage unharmed."] },
    Scenario { name: "Opposing doors", kicker: "SCENARIO 2 · HALLWAY AND CROSSING", map: 4, kind: Kind::Hallway, figs: false,
        blurb: "doors on both sides and a four-way crossing: never walk past an opening you have not sliced",
        lead: "One hallway, four rooms facing each other in pairs and a crossing with a passage on each side.",
        points: &["Every opening you walk past without slicing it is a line of fire into your side or your back. It costs 4 points each time.",
            "Opposing doors: slice the first from the far wall, then cross and slice the second. You cannot watch both at once, so decide the order before you move.",
            "At the crossing, clear one arm from the corner before you show yourself to the other."] },
    Scenario { name: "Threshold", kicker: "SCENARIO 3 · CLEAR FROM THE DOOR", map: 1, kind: Kind::Threshold, figs: false,
        blurb: "small rooms: clear them from the door and step in only for what you cannot see",
        lead: "Five small rooms. In a room this size, most of the work is done from outside.",
        points: &["Slice the whole doorway from the hallway. Threats you drop from outside the room never get a clean look at you.",
            "Step in only for the corner you could not see, and only after everything visible is clear.",
            "Entering a room with a threat still standing in it and less than 70% of it seen costs 6 points."] },
    Scenario { name: "Rear security", kicker: "SCENARIO 4 · THREATS ON THE MOVE", map: 3, kind: Kind::Rear, figs: true,
        blurb: "two threats move through the building and can come out of areas you already cleared",
        lead: "Most threats hold their rooms. Two of them do not: once the shooting starts they move through the building looking for you.",
        points: &["A cleared room does not stay cleared. Check behind you before you stop at a door, and again after every engagement.",
            "Do not park with your back to a long open space. Put a wall behind you when you slice.",
            "The run scores how many of the two roaming threats you saw before they saw you."] },
    Scenario { name: "Locked door", kicker: "SCENARIO 5 · NOISY BREACH", map: 2, kind: Kind::Locked, figs: false,
        blurb: "every door is locked: breaching takes time and tells everyone inside where you are",
        lead: "The three closed doors on this floor are locked. F starts a breach: it takes a second and a half and it is loud.",
        points: &["The noise alerts every threat in earshot, and the ones in that room turn to face the door before it opens.",
            "Breach from the side of the frame, never from the middle of the doorway. A breach from in front of the door costs 5 points.",
            "After the door gives, do not rush the gap: slice it like any other opening, from the wall."] },
    Scenario { name: "Second look", kicker: "SCENARIO 6 · TARGETS THAT CHANGE", map: 1, kind: Kind::Mind, figs: true,
        blurb: "some armed people give up, and some who gave up change their mind",
        lead: "Not everyone in this apartment will act the way they looked a second ago.",
        points: &["Some armed threats drop the gun and raise their hands when you challenge them. From then on they are no-shoot: hitting one costs 20 points.",
            "Some people standing with their hands up are waiting for you to look away. Keep them covered: if one draws, it is a threat again.",
            "Identification is for the moment you press the trigger, not for the first glance."] },
    Scenario { name: "Hostage rescue", kicker: "SCENARIO 7 · AGAINST THE CLOCK", map: 2, kind: Kind::Rescue, figs: false,
        blurb: "one hostage, one captor and a clock: go straight to the problem",
        lead: "A hostage is being held at the far end of the floor and time is running out. A marker and a knocking sound show the direction.",
        points: &["You do not have time to clear every room. Pass the ones that are not on your way, and accept the risk you can measure.",
            "The rest of the technique still applies at each opening you do cross: slice, then move.",
            "The run ends when the captor is down. If the clock runs out, the hostage is lost."] },
    Scenario { name: "Home defense", kicker: "SCENARIO 8 · DEFEND AND MOVE", map: 5, kind: Kind::Defense, figs: true,
        blurb: "hold your own house against waves of intruders: 2, then 4, 6, 8 and 10",
        lead: "Intruders come in waves from the street, through the gates: two, then four, then six, eight and ten. They slice a doorway before they enter, fall back when hit, and learn: a spot that worked once will not work twice.",
        points: &["The walls are OSB: they hide you, they do not stop rounds. Once they have seen you, move off that spot, because they will fire through the wall at it. You can do the same to them.",
            "A doorway that drops two of them is marked. The next ones throw a grenade through it before they enter, some take another way round, and some go to a window that looks into your room.",
            "So do not hold one doorway for the whole fight. Take your shots, change position, and reload between waves. If a grenade lands beside you, get a wall between you and it."] },
    Scenario { name: "Walker", kicker: "SCENARIO 9 · VERY HARD · ARMOUR AND FLANK", map: 3, kind: Kind::Walker, figs: true,
        blurb: "very hard: a walking tank with an infantry escort; one hit from its cannon ends the run",
        lead: "A walking tank patrols the lane between the shelving rows, and four armed threats hold the floor around it. This stage has its own difficulty, above HARD, whatever is selected on the menu.",
        points: &["The cannon needs one second on you. A red CANNON LOCK warning counts it: get a shelf between you and the walker before it reaches 100%. One hit ends the run.",
            "Rounds from the front bounce off. It takes twelve from the side or the rear: break its line of sight, move, and empty the magazine while it is still turning.",
            "The escort fires in under a second and hits 95% of its shots. Deal with the men first, from cover, and never with the walker's cannon coming round to you."] },
];
/// Quantos tiros pelos lados ou por trás derrubam o mech, e os dois pontos entre os quais ele patrulha (no galpão).
pub const MECH_HP: u8 = 12;
/// Tempo com o operador na mira até o canhão disparar, e velocidades de giro (rad/s) e de marcha (m/s) do mech.
pub const MECH_LOCK: f32 = 1.0;
pub const MECH_TURN: f32 = 1.9;
const MECH_SPEED: f32 = 1.5;
const MECH_POSTS: [Vec2; 2] = [Vec2::new(7.6, 10.7), Vec2::new(18.4, 10.7)];
pub const WAVES: u32 = 5;
/// Invasores da defesa da casa: tempo com o operador na mira até o primeiro tiro, intervalo entre tiros, acerto e passo.
pub const INV_REACT: f32 = 0.6;
pub const INV_FOLLOW: f32 = 0.26;
pub const INV_HIT: f32 = 0.85;
const INV_SPEED: f32 = 2.5;
/// Saída de uma passagem (escada, laje): quanto tempo o invasor leva para subir e de que profundidade ele vem.
/// Troca de andar de um invasor: ele anda até sumir no vão escuro da escada (ou sobe a escada de mão até o alto do muro) e sai
/// andando do vão escuro do outro andar. `leave` e `rise` guardam quanto do trecho já foi andado (zero = fora da passagem).
const HOP_SPEED: f32 = 2.3;
const LADDER_TOP: f32 = 3.0;
fn hop_path(q: &Portal, leaving: bool, start: Vec2) -> Vec<Vec3> {
    if leaving {
        let y = ramp_h(start.x, start.y);
        if q.alt { vec![Vec3::new(start.x, 0.0, start.y), Vec3::new(q.ha.0, 0.0, q.ha.1), Vec3::new(q.ha.0, LADDER_TOP, q.ha.1)] }
        else if !q.pa.is_empty() { std::iter::once(Vec3::new(start.x, y, start.y)).chain(q.pa.iter().map(|w| Vec3::new(w.0, w.2, w.1))).collect() }
        else { vec![Vec3::new(start.x, y, start.y), Vec3::new(q.ha.0, y, q.ha.1)] }
    } else {
        let y = ramp_h(q.ex, q.ez);
        if !q.pb.is_empty() { q.pb.iter().map(|w| Vec3::new(w.0, w.2, w.1)).chain(std::iter::once(Vec3::new(q.ex, y, q.ez))).collect() }
        else { vec![Vec3::new(q.hb.0, y, q.hb.1), Vec3::new(q.ex, y, q.ez)] }
    }
}
/// Ponto a `s` metros do começo do caminho, a direção (no chão) do trecho em que ele está e o comprimento total.
fn along(path: &[Vec3], s: f32) -> (Vec3, Vec2, f32) {
    let total: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = s.clamp(0.0, total);
    let (mut at, mut dir) = (path[0], Vec2::ZERO);
    for w in path.windows(2) {
        let l = w[0].distance(w[1]);
        let d = Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z);
        if d.length() > 0.05 { dir = d.normalize(); }
        if left <= l { at = if l > 1e-4 { w[0].lerp(w[1], left / l) } else { w[1] }; return (at, dir, total); }
        left -= l; at = w[1];
    }
    (at, dir, total)
}
const ROAM_SPEED: f32 = 1.6;

/// Tempo do resgate, conforme a dificuldade (CQB_CLOCK troca o valor nos testes).
fn clock() -> f32 { crate::world::env_f("CQB_CLOCK", [90.0, 60.0, 45.0][difficulty() as usize]) }
pub fn kind(game: &Game) -> Option<Kind> { game.scn.map(|i| SCENARIOS[i].kind) }
pub fn is(game: &Game, k: Kind) -> bool { kind(game) == Some(k) }
/// Quantos invasores a defesa da casa tem no total (2 + 4 + ... ).
pub fn invaders_total() -> u32 { (1..=WAVES).map(|w| w * 2).sum() }

fn blocked(level: &Level, q: Vec2, r: f32) -> bool {
    level.solids.iter().any(|s| s.on && q.x > s.x1 - r && q.x < s.x2 + r && q.y > s.z1 - r && q.y < s.z2 + r)
}

/// Monta o cenário logo depois da partida ser zerada (alvos já sorteados nas vagas do mapa).
pub fn setup(i: usize, game: &mut Game, p: &mut Player, level: &Level, targets: &mut Targets, rnd: &mut dyn FnMut() -> u32) {
    let sc = &SCENARIOS[i];
    game.scn = Some(i); game.mode = Mode::Card;
    let n = targets.list.len();
    let threats: Vec<usize> = (0..n).filter(|&k| { let t = &targets.list[k]; t.def.kind == TKind::Threat && !t.def.passive && !t.off }).collect();
    let hostages: Vec<usize> = (0..n).filter(|&k| targets.list[k].def.kind == TKind::Civil && !targets.list[k].off).collect();
    match sc.kind {
        Kind::Shield => {
            // cada refém vai para a frente de uma ameaça (de preferência do mesmo cômodo), um pouco para o lado: da porta
            // aparece só uma fatia da ameaça
            let mut used: Vec<usize> = Vec::new();
            for &h in &hostages {
                let zone = targets.list[h].def.zone;
                let mut cands: Vec<usize> = threats.iter().copied().filter(|c| !used.contains(c)).collect();
                cands.sort_by_key(|c| (targets.list[*c].def.zone != zone) as u8);
                'pick: for c in cands {
                    let (tp, f, tz) = { let t = &targets.list[c]; (Vec2::new(t.def.x, t.def.z), t.fwd, t.def.zone) };
                    for side in [0.15f32, -0.15] {
                        let q = tp + f * 0.5 + f.perp() * side;
                        if blocked(level, q, 0.26) || zone_at(q.x, q.y) != Some(zone_index(tz)) { continue; }
                        let hd = &mut targets.list[h];
                        hd.def.x = q.x; hd.def.z = q.y; hd.def.fx = q.x + f.x * 3.0; hd.def.fz = q.y + f.y * 3.0; hd.def.zone = tz;
                        crate::combat::place_target(level, hd);
                        targets.list[c].shield = true;
                        used.push(c);
                        break 'pick;
                    }
                }
            }
            game.scn_a = used.len() as u32;
            for &c in &threats { targets.list[c].moved = 2; }
        }
        Kind::Locked => { for d in 0..level.doors.len().min(MAX_DOORS) { game.locked[d] = true; } }
        Kind::Mind => {
            // duas ameaças vão se render quando forem vistas de perto; duas já estão de mãos para o alto, mas vão sacar
            let mut pool = threats.clone();
            for k in (1..pool.len()).rev() { let j = (rnd() as usize) % (k + 1); pool.swap(k, j); }
            for (k, &c) in pool.iter().take(4).enumerate() {
                let t = &mut targets.list[c];
                t.mind = if k < 2 { MIND_WILL_YIELD } else { MIND_FAKE };
                t.mind_t = 0.0; t.moved = 2;
            }
            game.scn_a = pool.len().min(2) as u32; game.scn_b = pool.len().saturating_sub(2).min(2) as u32;
        }
        Kind::Rescue => {
            // o refém mais distante do ponto de partida é o que precisa ser resgatado; uma ameaça vira a captora dele
            let st = Vec2::new(map().start.0, map().start.1);
            if let Some(&h) = hostages.iter().max_by(|a, b| { let d = |k: usize| Vec2::new(targets.list[k].def.x, targets.list[k].def.z).distance(st); d(**a).partial_cmp(&d(**b)).unwrap() }) {
                let (hp, f, hz) = { let t = &targets.list[h]; (Vec2::new(t.def.x, t.def.z), t.fwd, t.def.zone) };
                let mut cands = threats.clone();
                cands.sort_by(|a, b| {
                    let key = |k: usize| { let t = &targets.list[k]; ((t.def.zone != hz) as u8 as f32) * 100.0 + Vec2::new(t.def.x, t.def.z).distance(hp) };
                    key(*a).partial_cmp(&key(*b)).unwrap()
                });
                if let Some(&c) = cands.first() {
                    for (side, back) in [(0.75f32, 0.1f32), (-0.75, 0.1), (0.0, -0.8), (0.9, -0.5), (-0.9, -0.5)] {
                        let q = hp + f.perp() * side + f * back;
                        if blocked(level, q, 0.28) || zone_at(q.x, q.y) != Some(zone_index(hz)) { continue; }
                        let t = &mut targets.list[c];
                        t.def.x = q.x; t.def.z = q.y; t.def.fx = q.x + f.x * 3.0; t.def.fz = q.y + f.y * 3.0; t.def.zone = hz;
                        crate::combat::place_target(level, t);
                        break;
                    }
                    targets.list[c].captor = true; targets.list[c].moved = 2;
                }
                targets.list[h].captor = true;
            }
            game.scn_t = clock();
        }
        Kind::Defense => {
            // os dez invasores ficam fora de cena até a primeira onda; munição reforçada para aguentar os trinta
            for &c in &threats { let t = &mut targets.list[c]; t.off = true; t.alive = false; t.fall = 1.0; t.landed = true; }
            for a in p.ammo.iter_mut() { a.1 *= 2; }
            p.reserve *= 2;
            // CQB_WAVE (testes): começa direto numa onda mais adiantada
            game.wave = (crate::world::env_f("CQB_WAVE", 1.0) as u32).max(1) - 1; game.scn_t = 6.0;
        }
        Kind::Rear => { for &c in &threats { targets.list[c].moved = 2; } }
        Kind::Walker => {
            // só o mech fica em cena
            for k in 0..n {
                let t = &mut targets.list[k];
                if is_mech(&t.def) { t.off = false; t.alive = true; t.alert = 9999.0; t.moved = 2; }
                // a escolta são as ameaças do salão do galpão; o resto da casa e os reféns ficam fora
                else if t.def.kind == TKind::Threat && !t.def.passive && t.def.zone == "W" {}
                else if !t.def.passive || t.def.kind == TKind::Civil { t.off = true; t.alive = false; t.fall = 1.0; t.landed = true; }
            }
        }
        Kind::Hallway | Kind::Threshold => {}
    }
}

/* ------------------------------------------------------------------ deslocamento pela casa ------------------------------------------------------------------ */

const NC: f32 = 0.25;
const NW: usize = 88;
const NH: usize = 76;
fn ncell(q: Vec2) -> Option<usize> {
    let (i, j) = (((q.x - MAP_X1) / NC).floor() as i32, ((q.y - MAP_Z1) / NC).floor() as i32);
    if i < 0 || j < 0 || i >= NW as i32 || j >= NH as i32 { None } else { Some(j as usize * NW + i as usize) }
}
fn ncenter(k: usize) -> Vec2 { Vec2::new(MAP_X1 + ((k % NW) as f32 + 0.5) * NC, MAP_Z1 + ((k / NW) as f32 + 0.5) * NC) }

/// Grade de caminhada do mapa atual e distância a pé de cada casa até o operador (com e sem o caminho alternativo).
#[derive(Resource, Default)]
/// `hot` = casas dos vãos que já derrubaram dois invasores: para quem vai pelo caminho alternativo, passar por elas custa caro.
/// `post_at`/`post` = posto de janela (o ponto, de onde se vê o operador por uma janela) e a distância a pé de cada casa até ele.
pub struct Nav { built: Option<(usize, usize)>, walk: Vec<bool>, dist: [Vec<u32>; 2], t: f32, hot: Vec<bool>, post_at: Option<Vec2>, post: Vec<u32>, post_t: f32 }
impl Nav {
    fn build(&mut self, level: &Level) {
        // portas com folha não contam como parede: quem caça o operador abre a porta
        let hit = |q: Vec2| level.solids.iter().any(|s| s.on && s.kind != SolidKind::Door && q.x > s.x1 - 0.27 && q.x < s.x2 + 0.27 && q.y > s.z1 - 0.27 && q.y < s.z2 + 0.27);
        self.walk = (0..NW * NH).map(|k| { let c = ncenter(k); zone_at(c.x, c.y).is_some() && !hit(c) }).collect();
        self.built = Some((map_index(), level.solids.len()));
        self.t = 0.0;
    }
    /// Distância a pé de cada casa até `from`, por qualquer caminho (escada e escadas de mão), sem o custo das portas marcadas.
    fn field(&mut self, from: Vec2) -> Vec<u32> {
        let (keep, hot) = (std::mem::take(&mut self.dist), std::mem::take(&mut self.hot));
        self.flow(from);
        let out = std::mem::take(&mut self.dist[1]);
        self.dist = keep; self.hot = hot;
        out
    }
    fn flow(&mut self, from: Vec2) {
        for alt in 0..2 {
            let mut d = vec![u32::MAX; NW * NH];
            let Some(start) = ncell(from) else { self.dist[alt] = d; continue };
            let mut heap = BinaryHeap::new();
            d[start] = 0; heap.push(Reverse((0u32, start)));
            // passagens: quem está no retângulo de entrada chega à saída quase de graça
            let ports: Vec<(usize, &Portal)> = map().portals.iter().filter(|q| alt == 1 || !q.alt).filter_map(|q| ncell(Vec2::new(q.ex, q.ez)).map(|c| (c, q))).collect();
            while let Some(Reverse((dk, k))) = heap.pop() {
                if dk > d[k] { continue; }
                let (i, j) = ((k % NW) as i32, (k / NW) as i32);
                for (di, dj, cost) in [(1i32, 0i32, 10u32), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)] {
                    let (ni, nj) = (i + di, j + dj);
                    if ni < 0 || nj < 0 || ni >= NW as i32 || nj >= NH as i32 { continue; }
                    let nk = nj as usize * NW + ni as usize;
                    if !self.walk[nk] { continue; }
                    // na diagonal, não corta quina
                    if di != 0 && dj != 0 && (!self.walk[j as usize * NW + ni as usize] || !self.walk[nj as usize * NW + i as usize]) { continue; }
                    let cost = cost + if alt == 1 && self.hot.get(nk).copied().unwrap_or(false) { 300 } else { 0 };
                    if dk + cost < d[nk] { d[nk] = dk + cost; heap.push(Reverse((dk + cost, nk))); }
                }
                for (ec, q) in &ports {
                    if *ec != k { continue; }
                    let (i0, i1, j0, j1) = (((q.x1 - MAP_X1) / NC).floor() as usize, ((q.x2 - MAP_X1) / NC).floor() as usize, ((q.z1 - MAP_Z1) / NC).floor() as usize, ((q.z2 - MAP_Z1) / NC).floor() as usize);
                    for jj in j0..=j1.min(NH - 1) { for ii in i0..=i1.min(NW - 1) {
                        let nk = jj * NW + ii;
                        // só as casas com o centro dentro do retângulo: é lá que a passagem funciona de fato
                        let cc = ncenter(nk);
                        if cc.x <= q.x1 || cc.x >= q.x2 || cc.y <= q.z1 || cc.y >= q.z2 { continue; }
                        if self.walk[nk] && dk + 6 < d[nk] { d[nk] = dk + 6; heap.push(Reverse((dk + 6, nk))); }
                    } }
                }
            }
            self.dist[alt] = d;
        }
    }
    /// Distância a pé (em metros) de um ponto até o operador.
    pub fn walk_dist(&self, q: Vec2) -> Option<f32> { ncell(q).and_then(|k| self.dist[0].get(k).copied()).filter(|d| *d != u32::MAX).map(|d| d as f32 * NC / 10.0) }
}

/* ---------- cabeça de quem caça o operador: fatiar portas, susto e recuo ---------- */

/// Até que distância a pé do operador o invasor fatia as portas do caminho (mais longe ele só anda).
const SLICE_RANGE: f32 = 14.0;
const STACK_SPEED: f32 = 1.6;
const SLICE_SPEED: f32 = 0.85;
const RETREAT_SPEED: f32 = 3.4;
/// Inclinação na quina (radianos): a cabeça sai uns 45 cm para o lado, o pé fica atrás do batente.
const LEAN: f32 = 0.30;
/// Quanto tempo ele troca tiros inclinado atrás do batente antes de sair de corpo inteiro para o vão.
const PEEK_SECS: f32 = 1.0;

fn guv(g: &Gap, q: Vec2) -> (f32, f32) { if g.horizontal { (q.x, q.y) } else { (q.y, q.x) } }
fn gxy(g: &Gap, u: f32, v: f32) -> Vec2 { if g.horizontal { Vec2::new(u, v) } else { Vec2::new(v, u) } }

/// Vão que o trecho `a`→`b` atravessa (índice em `Level::gaps`).
fn crossed_gap(level: &Level, a: Vec2, b: Vec2) -> Option<usize> { level.gaps.iter().position(|g| crosses(g, a, b)) }
fn crosses(g: &Gap, a: Vec2, b: Vec2) -> bool {
    let ((ua, va), (ub, vb)) = (guv(g, a), guv(g, b));
    if (va - g.c) * (vb - g.c) >= 0.0 { return false; }
    let u = ua + (ub - ua) * ((g.c - va) / (vb - va));
    u > g.a - 0.05 && u < g.b + 0.05
}

/// Janela: vão com peitoril (ninguém passa por ele).
fn is_window(level: &Level, g: &Gap) -> bool {
    let c = gxy(g, (g.a + g.b) / 2.0, g.c);
    level.props.iter().any(|p| p.kind == PropKind::Sill && c.x > p.x1 - 0.01 && c.x < p.x2 + 0.01 && c.y > p.z1 - 0.01 && c.y < p.z2 + 0.01)
}

/// Próxima casa do caminho até o operador (a vizinha de menor distância, sem cortar quina).
fn downhill(d: &[u32], c: usize) -> usize {
    let (ci, cj) = ((c % NW) as i32, (c / NW) as i32);
    let mut best = (d[c], c);
    for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let (ni, nj) = (ci + di, cj + dj);
        if ni < 0 || nj < 0 || ni >= NW as i32 || nj >= NH as i32 { continue; }
        let nk = nj as usize * NW + ni as usize;
        if di != 0 && dj != 0 && (d[cj as usize * NW + ni as usize] == u32::MAX || d[nj as usize * NW + ci as usize] == u32::MAX) { continue; }
        if d[nk] < best.0 { best = (d[nk], nk); }
    }
    best.1
}

/// Porta nos próximos dois metros e pouco do caminho de quem está em `pos`.
fn door_ahead(level: &Level, d: &[u32], pos: Vec2) -> Option<usize> {
    let mut c = ncell(pos)?;
    if d[c] == u32::MAX { return None; }
    let mut prev = pos;
    for _ in 0..9 {
        let n = downhill(d, c);
        if n == c { break; }
        let q = ncenter(n);
        if let Some(g) = crossed_gap(level, prev, q) { return Some(g); }
        prev = q; c = n;
    }
    None
}

/// Plano para fatiar um vão a partir de `pos`: onde parar ao lado do batente (se houver espaço), a quina em que ele gira
/// (o batente mais próximo) e até onde ele cruza o vão, sempre do lado de cá da parede.
fn slice_plan(nav: &Nav, g: &Gap, pos: Vec2) -> (Option<Vec2>, Vec2, Vec2) {
    let (pu, pv) = guv(g, pos);
    let s = if pv > g.c { 1.0 } else { -1.0 };
    let order = if pu < (g.a + g.b) / 2.0 { [(g.a, 1.0f32), (g.b, -1.0)] } else { [(g.b, -1.0), (g.a, 1.0)] };
    let reach = ((g.b - g.a) / 2.0).min(0.6);
    for (jamb, inw) in order {
        let stack = gxy(g, jamb - inw * 0.30, g.c + s * 0.52);
        if ncell(stack).map_or(false, |c| nav.walk[c]) { return (Some(stack), gxy(g, jamb + inw * 0.06, g.c), gxy(g, jamb + inw * reach, g.c + s * 0.52)); }
    }
    let (jamb, inw) = order[0];
    (None, gxy(g, jamb + inw * 0.06, g.c), gxy(g, jamb + inw * reach, g.c + s * 0.52))
}

/// Abrigo para quem recua: a casa livre mais próxima (a pé) de onde o operador não vê nem a cabeça nem o corpo dele, sem
/// ficar mais perto do operador. Devolve o caminho até lá, casa a casa.
fn cover_path(nav: &Nav, d: &[u32], level: &Level, from: Vec2, p: &Player) -> Option<Vec<Vec2>> {
    let start = ncell(from).filter(|c| nav.walk.get(*c).copied().unwrap_or(false))?;
    let mut prev = vec![usize::MAX; NW * NH];
    let mut depth = vec![u16::MAX; NW * NH];
    let mut q = std::collections::VecDeque::new();
    depth[start] = 0; q.push_back(start);
    let head = p.head_h();
    let hidden = |c: Vec2| !level.los(c.x, c.y, 1.4, p.eye.x, p.eye.z, p.eye.y) && !level.los(c.x, c.y, 1.0, p.pos.x, p.pos.y, head - 0.25);
    while let Some(k) = q.pop_front() {
        let c = ncenter(k);
        // escondido com folga: o ombro (25 cm para cada lado) também fica fora da vista
        let side = (p.pos - c).perp().normalize_or_zero() * 0.25;
        if depth[k] >= 3 && c.distance(p.pos) > 2.5 && (d[start] == u32::MAX || (d[k] != u32::MAX && d[k] + 40 >= d[start])) && hidden(c) && hidden(c + side) && hidden(c - side) {
            let mut path = vec![c];
            let mut j = k;
            while prev[j] != usize::MAX { j = prev[j]; path.push(ncenter(j)); }
            path.pop();
            path.reverse();
            return Some(path);
        }
        if depth[k] >= 34 { continue; }
        let (i, j) = ((k % NW) as i32, (k / NW) as i32);
        for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (ni, nj) = (i + di, j + dj);
            if ni < 0 || nj < 0 || ni >= NW as i32 || nj >= NH as i32 { continue; }
            let nk = nj as usize * NW + ni as usize;
            if !nav.walk[nk] || depth[nk] != u16::MAX { continue; }
            if di != 0 && dj != 0 && (!nav.walk[j as usize * NW + ni as usize] || !nav.walk[nj as usize * NW + i as usize]) { continue; }
            depth[nk] = depth[k] + 1; prev[nk] = k; q.push_back(nk);
        }
    }
    None
}

/// Põe quem caça o operador num ponto, de frente para `f` (corpo, zona e boneco; nos degraus, na altura do piso).
fn place(t: &mut TargetState, roots: &mut Query<&mut Transform, Without<crate::operator::Fig>>, np: Vec2, f: Vec2) {
    t.def.x = np.x; t.def.z = np.y; t.def.fx = np.x + f.x; t.def.fz = np.y + f.y; t.fwd = f;
    if let Some(z) = zone_at(np.x, np.y) { t.def.zone = zones()[z].id; }
    if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(np.x, ramp_h(np.x, np.y), np.y); tf.rotation = Quat::from_rotation_y(f.x.atan2(f.y)); }
}

#[derive(Default)]
pub struct ScnState {
    patrol: usize, patrol_wait: f32,
    last_time: f32, sides: Vec<i8>, hint_cd: f32, entered: [bool; MAX_DOORWAYS], knock: f32, roamers: u32, wave_done: u32, pass_cd: f32, voice_cd: f32, last_pos: Vec2,
    /// Vãos que alguém do grupo acabou de fatiar (índice e até quando vale): os outros entram atrás sem repetir.
    sliced: Vec<(i16, f32)>,
    /// Quantos invasores cada vão já derrubou (índice do vão, contagem): com dois, o grupo passa a tratar a porta como
    /// perigosa; intervalo até alguém poder jogar outra granada antes de entrar.
    door_kills: Vec<(i16, u8)>, flush_cd: f32,
}

/// Regras de cada cenário, passagens entre andares, ondas de invasores e deslocamento de quem caça o operador.
#[allow(clippy::too_many_arguments)]
pub fn scenario_system(
    time: Res<Time>, mut game: ResMut<Game>, mut p: ResMut<Player>, mut level: ResMut<Level>, mut targets: ResMut<Targets>, tac: Res<Tac>, mut nav: ResMut<Nav>,
    mut sfx: EventWriter<Sfx>, mut commands: Commands, mut roots: Query<&mut Transform, Without<crate::operator::Fig>>, mut st: Local<ScnState>,
) {
    let dt = time.delta_secs().min(0.05);
    if game.fade_t > 0.0 { game.fade_t = (game.fade_t - dt).max(0.0); }
    if game.mode != Mode::Play || game.demo { return; }
    if game.time < st.last_time { *st = ScnState::default(); nav.hot.clear(); }
    st.last_time = game.time;
    st.hint_cd = (st.hint_cd - dt).max(0.0); st.pass_cd = (st.pass_cd - dt).max(0.0); st.voice_cd = (st.voice_cd - dt).max(0.0);
    let k = kind(&game);
    let dying = game.down_t > 0.0;
    let rnd = |s: f32| (s.sin() * 43758.547).fract().abs();

    /* ---------- passagens entre andares (escada) ---------- */
    game.portal_cd = (game.portal_cd - dt).max(0.0);
    if game.portal_cd <= 0.0 && !dying {
        // só quando o operador anda para dentro do vão escuro: quem acabou de chegar e se ajeita no patamar não volta sem querer
        if let Some(q) = map().portals.iter().find(|q| q.player && p.pos.x > q.x1 && p.pos.x < q.x2 && p.pos.y > q.z1 && p.pos.y < q.z2 && (Vec2::new(q.ha.0, q.ha.1) - p.pos).dot(p.vel) > 0.15) {
            p.pos = Vec2::new(q.ex, q.ez); p.vel = Vec2::ZERO; p.yaw = q.yaw; p.eye = Vec3::new(q.ex, p.eye.y, q.ez);
            game.portal_cd = 1.2; game.fade_t = 0.45;
            sfx.write(Sfx::Step(true));
        }
    }

    /* ---------- refém na linha de tiro: dica para abrir o ângulo ---------- */
    if p.ads && st.hint_cd <= 0.0 {
        let (eye, f) = (p.eye2(), p.fwd());
        let aimed = targets.list.iter().filter(|t| t.hostile() && t.vis).any(|t| {
            let v = Vec2::new(t.def.x, t.def.z) - eye;
            let d = v.length().max(0.01);
            v.dot(f) / d > 0.995 && targets.list.iter().any(|c| c.alive && !c.off && c.def.kind == TKind::Civil && { let w = Vec2::new(c.def.x, c.def.z) - eye; let along = w.dot(v / d); along > 0.0 && along < d + 0.4 && w.perp_dot(v / d).abs() < 0.3 })
        });
        if aimed {
            st.hint_cd = 7.0;
            game.coach = Some(("Hostage in your line of fire. Step sideways until the threat's head or chest is clear of her.".to_string(), 2, 4.5));
            game.ui_dirty = true;
        }
    }

    /* ---------- abertura não fatiada deixada para trás (percurso livre e cenários, fora da defesa) ---------- */
    if game.free && k != Some(Kind::Defense) {
        if st.sides.len() != openings().len() { st.sides = vec![0; openings().len()]; }
        // salto de posição (escada, retomada): esquece de que lado de cada abertura o operador estava
        if p.pos.distance(st.last_pos) > 0.6 { st.sides.fill(0); }
        st.last_pos = p.pos;
        for (i, o) in openings().iter().enumerate() {
            let zi = zone_index(o.zone);
            let rel = p.pos - o.mid();
            let (out, along, half) = (-rel.dot(o.n), rel.dot(o.w()), o.a.distance(o.b) / 2.0);
            // só conta quem passa pelo lado de fora, rente ao vão, e atravessa a largura inteira dele
            if !(0.15..2.6).contains(&out) || p.zone == zi { st.sides[i] = 0; continue; }
            let side: i8 = if along < -half - 0.1 { -1 } else if along > half + 0.1 { 1 } else { 0 };
            if side == 0 { continue; }
            if st.sides[i] != 0 && st.sides[i] != side {
                let hot = targets.list.iter().any(|t| t.hostile() && zone_index(t.def.zone) == zi);
                let pie = tac.pie_deg(i);
                if (hot || tac.pct[zi] < 0.6) && pie < 50.0 && o.door.map_or(true, |d| level.doors[d].open > 0.5) {
                    game.passes += 1; game.scn_adj -= 4;
                    if st.pass_cd <= 0.0 {
                        st.pass_cd = 4.0;
                        game.coach = Some((format!("You walked past the {} with only {}° of it cleared. Slice an opening before you cross it.", o.name.to_lowercase(), pie.round() as i32), 2, 5.0));
                        sfx.write(Sfx::Bad); game.ui_dirty = true;
                    }
                }
            }
            st.sides[i] = side;
        }
    }

    match k {
        /* ---------- limpar da soleira: entrar com ameaça de pé lá dentro e pouco cômodo visto ---------- */
        Some(Kind::Threshold) => {
            for (dw, w) in doorways().iter().enumerate() {
                if st.entered[dw] { continue; }
                if let Some(seen) = game.entry[dw] {
                    st.entered[dw] = true;
                    let zi = zone_index(w.zone);
                    if seen < 0.7 && targets.list.iter().any(|t| t.hostile() && zone_index(t.def.zone) == zi) {
                        game.scn_adj -= 6; game.scn_a += 1;
                        game.coach = Some((format!("You entered the {} with {}% of it seen and a threat still standing inside. Clear from the door first.", zones()[zi].name.to_lowercase(), (seen * 100.0).round() as i32), 2, 5.0));
                        sfx.write(Sfx::Bad); game.ui_dirty = true;
                    }
                }
            }
        }
        /* ---------- resgate contra o relógio ---------- */
        Some(Kind::Rescue) if !dying => {
            let hostage = targets.list.iter().position(|t| t.captor && t.def.kind == TKind::Civil);
            let captor = targets.list.iter().position(|t| t.captor && t.def.kind == TKind::Threat);
            if let (Some(h), Some(c)) = (hostage, captor) {
                let hp = Vec3::new(targets.list[h].def.x, 1.0, targets.list[h].def.z);
                if !targets.list[c].alive && targets.list[h].alive {
                    if !game.won { game.won = true; game.scn_adj += (game.scn_t / clock() * 20.0).round() as i32; sfx.write(Sfx::Good); game.toast("Captor down. Hostage safe.", 1, 2.0); }
                    game.lesson_end += dt;
                    if game.lesson_end > 1.4 { game.mode = Mode::Debrief; game.debriefed = true; game.ui_dirty = true; }
                } else if !targets.list[h].alive {
                    game.down_title = "HOSTAGE LOST".into(); game.down_why = "The hostage you came for was hit. Identify before every shot, most of all in the last room.".into();
                    game.mode = Mode::Down; game.ui_dirty = true;
                } else {
                    game.scn_t -= dt;
                    // batidas vindas do cômodo do refém, para dar a direção
                    st.knock -= dt;
                    if st.knock <= 0.0 { st.knock = 3.5; sfx.write(Sfx::Metal(hp)); }
                    if game.scn_t <= 0.0 {
                        let t = &mut targets.list[h];
                        t.alive = false; t.fall_v = 1.0;
                        sfx.write(Sfx::EnemyShot(hp)); sfx.write(Sfx::Bad);
                        game.down_title = "HOSTAGE LOST".into(); game.down_why = "The clock ran out before you reached the captor. Go straight to the problem: pass the rooms that are not on your way.".into();
                        game.mode = Mode::Down; game.ui_dirty = true;
                    }
                }
            }
        }
        /* ---------- alvos que mudam de ideia ---------- */
        Some(Kind::Mind) => {
            let eye = p.eye2();
            for i in 0..targets.list.len() {
                let t = &mut targets.list[i];
                if !t.alive || t.off { continue; }
                let d = Vec2::new(t.def.x, t.def.z).distance(eye);
                match t.mind {
                    MIND_WILL_YIELD if t.vis && d < 9.0 => {
                        if t.mind_t == 0.0 && st.voice_cd <= 0.0 { st.voice_cd = 3.0; sfx.write(Sfx::Voice(1)); }
                        t.mind_t += dt;
                        if t.mind_t > 0.5 {
                            t.mind = MIND_YIELDED; t.engaged = false; t.react = 0.0; t.alert = 0.0;
                            game.coach = Some(("It dropped the gun and raised its hands. It is no-shoot now: keep it covered and move on.".to_string(), 1, 4.5));
                            game.ui_dirty = true;
                        }
                    }
                    MIND_FAKE => {
                        // saca depois de um tempo sendo vigiado, ou logo que o operador desvia o olhar
                        let seen_enough = t.mind_t > 0.3;
                        if t.vis && d < 10.0 { t.mind_t += dt; t.notice = 0.0; } else if seen_enough { t.notice += dt; }
                        let limit = 1.8 + rnd(i as f32 * 12.7 + game.seed as f32) * 1.6;
                        if t.mind_t > limit || (seen_enough && t.notice > 0.45) {
                            t.mind = MIND_DREW; t.alert = 30.0; t.react = 0.25; t.notice = 1.0;
                            sfx.write(Sfx::Rustle);
                        }
                    }
                    _ => {}
                }
            }
        }
        /* ---------- ameaças que circulam e aparecem por trás ---------- */
        Some(Kind::Rear) => {
            let want = if game.stats.threats_down >= 6 { 2 } else if game.stats.threats_down >= 3 { 1 } else { 0 };
            if st.roamers < want {
                // uma ameaça ainda não vista sai do cômodo dela e reaparece numa área já limpa, longe e fora da vista do operador
                let pick = (0..targets.list.len()).filter(|&i| { let t = &targets.list[i]; t.hostile() && !t.hunter && !t.seen && !t.vis && !t.engaged })
                    .max_by(|a, b| { let d = |i: usize| Vec2::new(targets.list[i].def.x, targets.list[i].def.z).distance(p.pos); d(*a).partial_cmp(&d(*b)).unwrap() });
                let mut spots: Vec<Vec2> = vec![Vec2::new(map().start.0, map().start.1)];
                for s in slots() { if game.cleared & (1 << zone_index(s.0)) != 0 { spots.push(Vec2::new(s.1, s.2)); } }
                let spot = spots.into_iter().filter(|q| q.distance(p.pos) > 7.0 && !level.los(q.x, q.y, 1.5, p.eye.x, p.eye.z, p.eye.y) && !blocked(&level, *q, 0.28)
                        && !targets.list.iter().any(|t| t.alive && !t.off && Vec2::new(t.def.x, t.def.z).distance(*q) < 0.7))
                    .max_by(|a, b| a.distance(p.pos).partial_cmp(&b.distance(p.pos)).unwrap());
                if let (Some(i), Some(q)) = (pick, spot) {
                    let t = &mut targets.list[i];
                    let f = (p.pos - q).normalize_or_zero();
                    t.def.x = q.x; t.def.z = q.y; t.def.fx = q.x + f.x; t.def.fz = q.y + f.y;
                    if let Some(z) = zone_at(q.x, q.y) { t.def.zone = zones()[z].id; }
                    crate::combat::place_target(&level, t);
                    t.hunter = true; t.alert = 600.0; t.goal = None;
                    if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(q.x, 0.0, q.y); tf.rotation = Quat::from_rotation_y(f.x.atan2(f.y)); }
                    st.roamers += 1;
                    if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT RONDA {} saiu para ({:.1},{:.1}), a {:.1} m do operador", targets.list[i].def.id, q.x, q.y, q.distance(p.pos)); }
                }
            }
        }
        /* ---------- mech: patrulha de um ponto ao outro enquanto não tem o operador na mira ---------- */
        Some(Kind::Walker) => {
            // o mech tem corpo: o operador não entra nele (sem isso, o focinho e o canhão atravessavam a câmera e pareciam
            // estar na mão do operador), nem de pé nem depois de derrubado
            if let Some((mp, mf)) = targets.list.iter().find(|t| is_mech(&t.def) && !t.off).map(|t| (Vec2::new(t.def.x, t.def.z), t.fwd)) {
                // três círculos ao longo do corpo: cauda, meio e focinho (o focinho chega a 1,2 m do centro)
                for (along, r) in [(0.0f32, 0.85f32), (0.8, 0.7), (-0.75, 0.6)] {
                    let c = mp + mf * along;
                    let v = p.pos - c;
                    let (d, min) = (v.length(), r + PLAYER_R);
                    if d < min {
                        let out = if d > 0.01 { v / d } else { -mf };
                        let mut np = c + out * min;
                        level.resolve_circle(&mut np, PLAYER_R);
                        let shift = np - p.pos;
                        p.pos = np; p.eye.x += shift.x; p.eye.z += shift.y;
                    }
                }
            }
            if let Some(t) = targets.list.iter_mut().find(|t| is_mech(&t.def) && t.alive && !t.off) {
                if t.engaged { t.moving = false; }
                else {
                    let pos = Vec2::new(t.def.x, t.def.z);
                    let goal = MECH_POSTS[st.patrol % 2];
                    let v = goal - pos;
                    t.moving = false;
                    if v.length() < 0.15 { st.patrol_wait += dt; if st.patrol_wait > 1.5 { st.patrol_wait = 0.0; st.patrol += 1; } }
                    else {
                        // vira para onde vai, devagar, e só anda quando está de frente
                        let dir = v.normalize();
                        let cur = t.fwd.x.atan2(t.fwd.y);
                        let off = ang_diff(dir.x.atan2(dir.y), cur);
                        let a = cur + off.clamp(-MECH_TURN * dt, MECH_TURN * dt);
                        t.fwd = Vec2::new(a.sin(), a.cos());
                        let mut np = pos;
                        // as pernas trabalham também enquanto ele gira no lugar
                        t.moving = true;
                        if off.abs() < 0.35 { np = pos + dir * (MECH_SPEED * dt).min(v.length()); }
                        t.def.x = np.x; t.def.z = np.y; t.def.fx = np.x + t.fwd.x; t.def.fz = np.y + t.fwd.y;
                        if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(np.x, 0.0, np.y); tf.rotation = Quat::from_rotation_y(t.fwd.x.atan2(t.fwd.y)); }
                    }
                }
            }
        }
        /* ---------- defesa da casa: ondas ---------- */
        Some(Kind::Defense) if !dying => {
            // o operador defende a casa: não passa do portão para a rua (é lá, atrás do muro, que os invasores chegam)
            if p.pos.y > 16.75 { let dz = p.pos.y - 16.75; p.pos.y = 16.75; p.eye.z -= dz; p.vel.y = p.vel.y.min(0.0); }
            let n = targets.list.len();
            // quem está esperando a vez entra em cena
            for i in 0..n {
                let t = &mut targets.list[i];
                if t.pend > 0.0 {
                    t.pend -= dt;
                    if t.pend <= 0.0 {
                        t.off = false; t.alive = true; t.fall = 0.0; t.landed = false;
                        commands.entity(t.root).insert(Visibility::Inherited);
                        if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(t.def.x, 0.0, t.def.z); tf.rotation = Quat::from_rotation_y((t.def.fx - t.def.x).atan2(t.def.fz - t.def.z)); }
                    }
                }
            }
            let busy = targets.list.iter().any(|t| t.pend > 0.0 || (t.hunter && t.alive && !t.off));
            if !busy {
                if game.wave > 0 && st.wave_done != game.wave {
                    st.wave_done = game.wave;
                    game.scn_t = 6.0;
                    // entre uma onda e outra o operador se recompõe: os dois tiros voltam a valer por onda
                    game.wounds = 0;
                    sfx.write(Sfx::Good);
                    if game.wave >= WAVES { game.won = true; game.toast("Last wave down. The house held.", 1, 3.0); }
                    else { let msg = format!("Wave {} down. Next wave: {} intruders. Reload now.", game.wave, (game.wave + 1) * 2); game.toast(&msg, 1, 4.5); }
                    game.ui_dirty = true;
                }
                if game.won {
                    game.lesson_end += dt;
                    if game.lesson_end > 2.0 { game.mode = Mode::Debrief; game.debriefed = true; game.ui_dirty = true; }
                } else {
                    game.scn_t -= dt;
                    if game.scn_t <= 0.0 {
                        game.wave += 1;
                        let count = ((game.wave * 2) as usize).min(n);
                        let pool: Vec<usize> = (0..n).filter(|&i| targets.list[i].def.kind == TKind::Threat).take(count).collect();
                        for (slot, &i) in pool.iter().enumerate() {
                            let t = &mut targets.list[i];
                            t.def = target_defs()[i];
                            t.clear_run();
                            crate::combat::place_target(&level, t);
                            // fora de cena até a vez dele: a onda chega junta à rua e entra em grupo pelos dois portões (um a um, o portão vira um
                            // funil que qualquer posição de frente para a entrada vence)
                            t.off = true; t.alive = false; t.fall = 1.0; t.landed = true;
                            // alguns, dos dois lados, conhecem a subida para o terraço
                            t.hunter = true; t.alt = matches!(slot, 2 | 3 | 6 | 7); t.alert = 9999.0; t.pend = 0.05 + slot as f32 * 0.3;
                            for m in t.marks.drain(..) { commands.entity(m).despawn(); }
                            commands.entity(t.root).insert(Visibility::Hidden);
                        }
                        if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT ONDA {} com {} invasores t={:.1} derrubados={}", game.wave, count, game.time, game.stats.threats_down); }
                        let msg = format!("Wave {} of {}: {} intruders at the gates.", game.wave, WAVES, count);
                        game.toast(&msg, 2, 3.5);
                        sfx.write(Sfx::Bad); game.ui_dirty = true;
                    }
                }
            }
        }
        _ => {}
    }

    /* ---------- portas que custam caro ---------- */
    // cada invasor derrubado conta para o vão de passagem (janela não) mais próximo de onde ele caiu. Com dois, a porta fica
    // marcada: metade do grupo passa a preferir outro caminho e quem for entrar por ela joga uma granada antes
    st.flush_cd = (st.flush_cd - dt).max(0.0);
    let fell: Vec<Vec2> = std::mem::take(&mut game.fell);
    let mut newly_hot = false;
    for q in fell {
        // a porta por onde o tiro passou (a linha entre quem caiu e o operador cruza o vão); senão, a mais próxima de onde ele caiu
        let through = level.gaps.iter().enumerate().filter(|(_, g)| !is_window(&level, g)).find(|(_, g)| crosses(g, q, p.pos)).map(|(gi, _)| (gi, 0.0f32));
        let best = through.or_else(|| level.gaps.iter().enumerate().filter(|(_, g)| !is_window(&level, g))
            .map(|(gi, g)| { let (u, v) = guv(g, q); (gi, Vec2::new(u - u.clamp(g.a, g.b), v - g.c).length()) })
            .filter(|x| x.1 < 2.0).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
        if let Some((gi, _)) = best {
            match st.door_kills.iter_mut().find(|d| d.0 == gi as i16) { Some(d) => d.1 = d.1.saturating_add(1), None => st.door_kills.push((gi as i16, 1)) }
            let n = st.door_kills.iter().find(|d| d.0 == gi as i16).map_or(0, |d| d.1);
            if n == 2 { newly_hot = true; }
            if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT PORTA vao={} derrubou {} t={:.1}", gi, n, game.time); }
        }
    }
    if newly_hot {
        let mut hot = vec![false; NW * NH];
        for (gi, n) in st.door_kills.iter().filter(|d| d.1 >= 2) {
            let _ = n;
            let g = &level.gaps[*gi as usize];
            for (k, h) in hot.iter_mut().enumerate() { let (u, v) = guv(g, ncenter(k)); if u > g.a - 0.1 && u < g.b + 0.1 && (v - g.c).abs() < 0.3 { *h = true; } }
        }
        nav.hot = hot; nav.t = 0.0;
        let mut odd = false;
        for t in targets.list.iter_mut().filter(|t| t.hunter && t.alive && !t.off) { odd = !odd; if odd { t.alt = true; } }
    }

    /* ---------- quem caça o operador anda até ele ---------- */
    if !targets.list.iter().any(|t| t.hunter && t.alive && !t.off) { return; }
    if nav.built != Some((map_index(), level.solids.len())) { nav.build(&level); nav.post_at = None; }
    // porta marcada: um em cada três do grupo não entra por ela; vai para uma janela de onde se vê o operador e atira de lá.
    // O posto é escolhido de novo a cada segundo e meio (o operador muda de lugar); sem janela que sirva, ninguém é desviado
    nav.post_t -= dt;
    if nav.post_t <= 0.0 {
        nav.post_t = 1.5;
        let mut best: Option<(f32, Vec2)> = None;
        if st.door_kills.iter().any(|d| d.1 >= 2) {
            for g in level.gaps.iter().filter(|g| is_window(&level, g)) {
                for side in [-0.6f32, 0.6] {
                    let q = gxy(g, (g.a + g.b) / 2.0, g.c + side);
                    if !ncell(q).map_or(false, |c| nav.walk[c]) { continue; }
                    let dq = q.distance(p.pos);
                    if dq < 1.5 || dq > 12.0 || !level.los(q.x, q.y, 1.4, p.eye.x, p.eye.z, p.eye.y) { continue; }
                    // de preferência o posto em que já estão
                    let dq = if nav.post_at.map_or(false, |o| o.distance(q) < 0.1) { dq - 3.0 } else { dq };
                    if best.map_or(true, |b| dq < b.0) { best = Some((dq, q)); }
                }
            }
        }
        match best {
            Some((_, q)) => { if nav.post_at.map_or(true, |o| o.distance(q) > 0.1) { nav.post = nav.field(q); nav.post_at = Some(q); if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT JANELA posto em ({:.1},{:.1}) t={:.1}", q.x, q.y, game.time); } } }
            None => nav.post_at = None,
        }
    }
    nav.t -= dt;
    if nav.t <= 0.0 {
        nav.t = 0.3; nav.flow(p.pos);
        if std::env::var("CQB_HUNTLOG").is_ok() { let v: Vec<String> = targets.list.iter().filter(|t| t.hunter && t.alive && !t.off).map(|t| format!("{}({:.1},{:.1}){}", t.def.id, t.def.x, t.def.z, if t.engaged { "!" } else { "" })).collect(); println!("BOT CACA t={:.1} op=({:.1},{:.1}) {}", game.time, p.pos.x, p.pos.y, v.join(" ")); }
    }
    let speed = if k == Some(Kind::Defense) { INV_SPEED } else { ROAM_SPEED };
    let others: Vec<(usize, Vec2)> = targets.list.iter().enumerate().filter(|(_, t)| t.hunter && t.alive && !t.off).map(|(i, t)| (i, Vec2::new(t.def.x, t.def.z))).collect();
    // quem está fatiando qual vão (os outros do grupo esperam atrás e entram depois)
    let mut slicers: Vec<(usize, i16)> = targets.list.iter().enumerate().filter(|(_, t)| t.hunter && t.alive && !t.off && matches!(t.ai.tactic, TAC_STACK | TAC_SLICE)).map(|(i, t)| (i, t.ai.door)).collect();
    let now = game.time;
    st.sliced.retain(|s| s.1 > now);
    let script = std::env::var("CQB_SCRIPT").is_ok();
    let r01 = crate::combat::rand01;
    let turn = |from: Vec2, to: Vec2, rate: f32| { let f = Vec2::new(crate::map::damp(from.x, to.x, rate, dt), crate::map::damp(from.y, to.y, rate, dt)).normalize_or_zero(); if f == Vec2::ZERO { from } else { f } };
    for i in 0..targets.list.len() {
        let t = &mut targets.list[i];
        if !t.hunter || !t.alive || t.off { continue; }
        t.alert = t.alert.max(60.0);
        t.ai.lean = crate::map::damp(t.ai.lean, t.ai.lean_to, 10.0, dt);
        t.ai.flinch = (t.ai.flinch - dt).max(0.0); t.ai.toss = (t.ai.toss - dt).max(0.0); t.ai.backing = false; t.ai.side = 0.0;
        t.ai.door_t = (t.ai.door_t - dt).max(0.0);
        // de frente para o operador e com linha de tiro: para e atira (quem decide é o combate)
        if t.leave > 0.0 || t.rise > 0.0 {
            // na passagem entre andares: partida (até sumir no vão escuro ou no alto da escada de mão) e chegada (saindo do vão)
            let q = &map().portals[(t.via as usize).min(map().portals.len() - 1)];
            t.ai.tactic = TAC_NONE; t.ai.lean_to = 0.0; t.ai.ready = false; t.ai.scare = 0.0;
            let leaving = t.leave > 0.0;
            let path = hop_path(q, leaving, Vec2::new(t.def.x, t.def.z));
            let s = if leaving { &mut t.leave } else { &mut t.rise };
            *s += HOP_SPEED * dt;
            let (at, dir, total) = along(&path, *s);
            let done = *s >= total;
            // de frente para onde anda; na escada de mão, de frente para a parede
            let f = if dir != Vec2::ZERO { dir } else { t.fwd };
            t.fwd = f; t.def.fx = t.def.x + f.x; t.def.fz = t.def.z + f.y;
            if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = at; tf.rotation = Quat::from_rotation_y(f.x.atan2(f.y)); }
            if done {
                if leaving {
                    t.leave = 0.0; t.rise = 0.001;
                    t.def.x = q.ex; t.def.z = q.ez;
                    if let Some(z) = zone_at(q.ex, q.ez) { t.def.zone = zones()[z].id; }
                    let p0 = hop_path(q, false, Vec2::ZERO)[0];
                    if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = p0; }
                } else { t.rise = 0.0; }
            }
            t.moving = true;
            continue;
        }
        let pos = Vec2::new(t.def.x, t.def.z);
        // quem foi desviado para a janela segue o caminho até o posto; os outros, até o operador
        let post = nav.post_at.filter(|_| i % 3 == 1 && nav.post.len() == NW * NH && ncell(pos).map_or(false, |c| nav.post[c] != u32::MAX));
        let d: &[u32] = if post.is_some() { &nav.post } else { &nav.dist[t.alt as usize] };
        // susto: tiro rente, parceiro caindo perto ou ferimento. Leva um tranco e, conforme o sangue-frio de cada um, recua
        // para o abrigo mais próximo (uma vez só; ferido, quase metade das vezes)
        if t.ai.scare > 0.0 {
            let why = t.ai.scare;
            t.ai.scare = 0.0;
            if t.ai.flinch <= 0.0 { t.ai.flinch = 0.38; }
            let nerve = 0.3 + 0.7 * r01(i as f32 * 12.9898 + 3.1);
            let chance = if why >= 3.0 { 0.45 } else if why >= 2.0 { 0.45 * (1.3 - nerve) } else { 0.3 * (1.3 - nerve) };
            if t.ai.tactic != TAC_RETREAT && t.ai.retreats < 1 && r01(now * 37.7 + i as f32 * 5.3) < chance {
                if let Some(path) = cover_path(&nav, d, &level, pos, &p) {
                    t.ai.retreats += 1; t.ai.tactic = TAC_RETREAT; t.ai.path = path; t.ai.anchor = p.pos; t.wait = 4.0;
                    t.ai.t = if why >= 3.0 { 3.0 } else { 1.4 } + 2.0 * r01(now * 11.3 + i as f32);
                    t.ai.lean_to = 0.0; t.ai.ready = false; t.engaged = false; t.react = 0.0;
                    if script { println!("BOT RECUA {} motivo={} de ({:.1},{:.1}) t={:.1}", t.def.id, why, pos.x, pos.y, now); }
                }
            }
        }
        if t.ai.flinch > 0.0 { t.moving = false; continue; }
        // acabou de dar uma rajada através da parede: fica parado um instante, de arma apontada
        if t.ai.hold > 0.0 { t.ai.hold -= dt; t.moving = false; continue; }
        if t.ai.tactic == TAC_RETREAT {
            // corre até o abrigo, de costas para o operador; lá, vira para onde ele estava
            t.wait -= dt;
            let Some(&wp) = t.ai.path.first() else { t.ai.tactic = TAC_HOLD; continue; };
            if t.wait <= 0.0 { t.ai.path.clear(); t.ai.tactic = TAC_HOLD; continue; }
            let v = wp - pos;
            let len = v.length();
            if len < 0.15 { t.ai.path.remove(0); continue; }
            // o começo do recuo é de costas: ele se afasta sem tirar os olhos (nem o fuzil) do operador;
            // só depois vira e corre para o abrigo
            let back = 4.0 - t.wait < 1.3 && pos.distance(p.pos) > 0.6;
            let mut np = pos + v / len * ((if back { 1.1 } else { RETREAT_SPEED }) * dt).min(len);
            level.resolve_circle(&mut np, 0.28);
            let f = if back { turn(t.fwd, (p.pos - pos).normalize_or_zero(), 9.0) } else { turn(t.fwd, v / len, 14.0) };
            place(t, &mut roots, np, f);
            t.moving = true; t.ai.backing = back;
            continue;
        }
        // com o operador à vista ele para e atira (quem decide é o combate). Fatiando, só fica inclinado atrás do batente por
        // um instante: depois sai de trás da parede e termina de cruzar o vão, de corpo inteiro, mesmo sob fogo
        if t.engaged {
            if t.ai.tactic == TAC_SLICE { t.ai.peek += dt; }
            if t.ai.tactic != TAC_SLICE || t.ai.peek < PEEK_SECS { t.moving = false; continue; }
        }
        if t.ai.tactic == TAC_HOLD {
            // abrigado: arma apontada para onde o operador estava; depois volta, e metade das vezes por outro caminho
            t.ai.t -= dt; t.moving = false; t.ai.ready = true;
            let f = turn(t.fwd, (t.ai.anchor - pos).normalize_or_zero(), 8.0);
            place(t, &mut roots, pos, f);
            if t.ai.t <= 0.0 {
                t.ai.tactic = TAC_NONE; t.ai.ready = false;
                if r01(now * 7.1 + i as f32 * 3.3) < 0.5 { t.alt = !t.alt; }
                if script { println!("BOT VOLTA {} alt={} t={:.1}", t.def.id, t.alt, now); }
            }
            continue;
        }
        if t.ai.tactic == TAC_STACK {
            // vai para o lado do batente, de olho na quina
            t.ai.t -= dt;
            let v = t.ai.anchor - pos;
            let len = v.length();
            if len < 0.08 || t.ai.t <= 0.0 {
                t.ai.tactic = TAC_SLICE; t.ai.anchor = pos; t.ai.t = 0.7 + 0.6 * r01(now * 3.7 + i as f32); t.ai.ready = true; t.moving = false;
                // porta que já derrubou dois do grupo: antes de entrar, uma granada pelo vão. Cai perto do operador se der para
                // ver o ponto pela porta; senão, dois metros para dentro. Ele espera o estouro encostado na parede
                if t.ai.flush {
                    t.ai.flush = false;
                    let g = &level.gaps[(t.ai.door.max(0) as usize).min(level.gaps.len() - 1)];
                    let side = if guv(g, pos).1 > g.c { 1.0 } else { -1.0 };
                    let dc = gxy(g, (g.a + g.b) / 2.0, g.c);
                    let mut to: Option<Vec2> = None;
                    if dc.distance(p.pos) < 10.0 {
                        'find: for r in [1.0f32, 1.8, 2.6] {
                            for k in 0..10 {
                                let a = k as f32 / 10.0 * std::f32::consts::TAU;
                                let q = p.pos + Vec2::new(a.cos(), a.sin()) * r;
                                if zone_at(q.x, q.y).is_none() || blocked(&level, q, 0.15) { continue; }
                                if level.los(dc.x, dc.y, 1.5, q.x, q.y, 0.4) && level.los(q.x, q.y, 0.4, p.pos.x, p.pos.y, 1.0) { to = Some(q); break 'find; }
                            }
                        }
                    }
                    let to = to.unwrap_or_else(|| { let q = gxy(g, (g.a + g.b) / 2.0, g.c - side * 2.0); if zone_at(q.x, q.y).is_some() && !blocked(&level, q, 0.15) { q } else { gxy(g, (g.a + g.b) / 2.0, g.c - side * 1.0) } });
                    game.nade_req.push((Vec3::new(pos.x, 1.5, pos.y), Vec3::new(to.x, 0.05, to.y)));
                    t.ai.t = NADE_FLY + NADE_FUSE + 0.4; t.ai.toss = 1.1;
                    if script { println!("BOT GRANADA de porta: {} no vao {} para ({:.1},{:.1}) t={:.1}", t.def.id, t.ai.door, to.x, to.y, now); }
                }
                continue;
            }
            let mut np = pos + v / len * (STACK_SPEED * dt).min(len);
            for (j, q) in &others { if *j != i { let w = np - *q; let l = w.length(); if l < 0.5 && l > 0.001 { np += w / l * (0.5 - l) * 0.5; } } }
            level.resolve_circle(&mut np, 0.28);
            let f = turn(t.fwd, (t.ai.pivot - np).normalize_or_zero(), 9.0);
            place(t, &mut roots, np, f);
            t.moving = true;
            continue;
        }
        if t.ai.tactic == TAC_SLICE {
            // fatia o vão: arma apontada para a quina do batente, corpo inclinado para o lado do vão enquanto o pé ainda está
            // atrás da parede; espia um instante e cruza devagar. Se o operador aparecer, o combate assume (ele para e atira)
            t.ai.ready = true;
            let f = if pos.distance(t.ai.pivot) > 0.3 { turn(t.fwd, (t.ai.pivot - pos).normalize_or_zero(), 9.0) } else { t.fwd };
            let inward = (t.ai.to - t.ai.anchor).normalize_or_zero();
            let gone = (pos - t.ai.anchor).dot(inward);
            t.ai.lean_to = if gone < 0.45 && t.ai.t < 1.0 { LEAN * Vec2::new(-f.y, f.x).dot(inward).signum() } else { 0.0 };
            t.ai.t -= dt;
            let v = t.ai.to - pos;
            let len = v.length();
            if len < 0.08 || t.ai.t < -5.0 {
                t.ai.tactic = TAC_NONE; t.ai.ready = false; t.ai.lean_to = 0.0; t.ai.door_t = 8.0;
                st.sliced.push((t.ai.door, now + 5.0));
                continue;
            }
            if t.ai.t > 0.0 && t.ai.peek < PEEK_SECS { place(t, &mut roots, pos, f); t.moving = false; continue; }
            // sob fogo, sai mais depressa e já de frente para o operador (o combate cuida de para onde ele olha)
            let (f, sp) = if t.engaged { (t.fwd, SLICE_SPEED * 1.8) } else { (f, SLICE_SPEED) };
            if t.engaged { t.ai.lean_to = 0.0; }
            // cruzando o vão sem ninguém à vista: passo lateral agachado, para o lado em que ele está andando
            if !t.engaged { let lat = (v / len).dot(Vec2::new(-f.y, f.x)); if lat.abs() > 0.4 { t.ai.side = lat; } }
            let mut np = pos + v / len * (sp * dt).min(len);
            level.resolve_circle(&mut np, 0.28);
            place(t, &mut roots, np, f);
            t.moving = false;
            continue;
        }
        t.ai.ready = false; t.ai.lean_to = 0.0;
        // no posto de janela: parado, arma apontada para o operador (se houver visada, o combate já assumiu lá em cima)
        if let Some(q) = post {
            if pos.distance(q) < 0.4 {
                let f = turn(t.fwd, (p.pos - pos).normalize_or_zero(), 8.0);
                place(t, &mut roots, pos, f);
                t.ai.ready = true; t.moving = false;
                continue;
            }
        }
        // porta no caminho, com o operador por perto: o primeiro do grupo para ao lado do batente e fatia antes de entrar;
        // os outros esperam atrás dele e entram em seguida
        if let Some(gi) = door_ahead(&level, d, pos) {
            let near = ncell(pos).map_or(false, |c| d[c] != u32::MAX && d[c] as f32 * NC / 10.0 < SLICE_RANGE);
            let recent = (t.ai.door == gi as i16 && t.ai.door_t > 0.0) || st.sliced.iter().any(|s| s.0 == gi as i16);
            if near && !recent {
                if slicers.iter().any(|s| s.0 != i && s.1 == gi as i16) { t.moving = false; continue; }
                let (stack, pivot, to) = slice_plan(&nav, &level.gaps[gi], pos);
                t.ai.door = gi as i16; t.ai.pivot = pivot; t.ai.to = to; t.ai.peek = 0.0;
                t.ai.flush = st.flush_cd <= 0.0 && st.door_kills.iter().any(|d| d.0 == gi as i16 && d.1 >= 2);
                if t.ai.flush { st.flush_cd = 9.0; }
                match stack { Some(sp) => { t.ai.tactic = TAC_STACK; t.ai.anchor = sp; t.ai.t = 2.5; } None if t.ai.flush => { t.ai.tactic = TAC_STACK; t.ai.anchor = pos; t.ai.t = 0.1; } None => { t.ai.tactic = TAC_SLICE; t.ai.anchor = pos; t.ai.t = 0.2; } }
                slicers.push((i, gi as i16));
                if script { println!("BOT FATIA {} vao={} de ({:.1},{:.1}) batente={} t={:.1}", t.def.id, gi, pos.x, pos.y, stack.is_some(), now); }
                continue;
            }
        }
        // passagem (escada ou laje): sai do outro lado se por lá o caminho for mais curto
        // (vale o centro da casa da grade em que ele está: é assim que a distância foi calculada)
        let pc = ncell(pos).map_or(pos, ncenter);
        if let Some((qi, q)) = map().portals.iter().enumerate().find(|(_, q)| (t.alt || !q.alt) && pc.x > q.x1 && pc.x < q.x2 && pc.y > q.z1 && pc.y < q.z2) {
            let (here, there) = (ncell(pos).map_or(u32::MAX, |c| d[c]), ncell(Vec2::new(q.ex, q.ez)).map_or(u32::MAX, |c| d[c]));
            if here != u32::MAX && there < here {
                // começa a subir aqui mesmo; a chegada ao outro andar vem quando ele some no alto
                t.leave = 0.001; t.via = qi as u8;
                if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT SOBE {} pela passagem {} em ({:.1},{:.1}) t={:.1}", t.def.id, qi, t.def.x, t.def.z, game.time); }
                continue;
            }
        }
        // segue a descida da distância por algumas casas e anda na direção desse ponto
        let mut goal = post.unwrap_or(p.pos);
        if let Some(mut c) = ncell(pos) {
            if d.get(c).copied().unwrap_or(u32::MAX) == u32::MAX {
                // fora da grade de caminhada (encostado num móvel): vai para a casa livre mais próxima
                let mut best: Option<(f32, usize)> = None;
                for dj in -3i32..=3 { for di in -3i32..=3 {
                    let (ni, nj) = ((c % NW) as i32 + di, (c / NW) as i32 + dj);
                    if ni < 0 || nj < 0 || ni >= NW as i32 || nj >= NH as i32 { continue; }
                    let nk = nj as usize * NW + ni as usize;
                    if d[nk] != u32::MAX { let dd = ncenter(nk).distance(pos); if best.map_or(true, |b| dd < b.0) { best = Some((dd, nk)); } }
                } }
                if let Some((_, nk)) = best { goal = ncenter(nk); }
            } else {
                for step in 0..4 {
                    let (ci, cj) = ((c % NW) as i32, (c / NW) as i32);
                    let mut best = (d[c], c);
                    for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                        let (ni, nj) = (ci + di, cj + dj);
                        if ni < 0 || nj < 0 || ni >= NW as i32 || nj >= NH as i32 { continue; }
                        let nk = nj as usize * NW + ni as usize;
                        if di != 0 && dj != 0 && (d[cj as usize * NW + ni as usize] == u32::MAX || d[nj as usize * NW + ci as usize] == u32::MAX) { continue; }
                        if d[nk] < best.0 { best = (d[nk], nk); }
                    }
                    if best.1 == c { break; }
                    // só olha mais longe enquanto o caminho até lá estiver livre
                    let q = ncenter(best.1);
                    if step > 0 {
                        // a linha reta até lá precisa passar só por casas livres (senão o corpo esbarra em quina)
                        let v = q - pos;
                        let n = (v.length() / 0.1).ceil().max(1.0) as usize;
                        if (1..=n).any(|s| ncell(pos + v * (s as f32 / n as f32)).map_or(true, |cc| d[cc] == u32::MAX)) { break; }
                    }
                    c = best.1; goal = q;
                    if d[c] == 0 { goal = post.unwrap_or(p.pos); break; }
                }
            }
        }
        let v = goal - pos;
        let len = v.length();
        if std::env::var("CQB_HUNTLOG").map_or(false, |v| v == "2") && (game.time * 4.0).fract() < 0.04 { println!("BOT PASSO {} pos=({:.2},{:.2}) casa_d={:?} alvo=({:.2},{:.2}) alt={}", t.def.id, pos.x, pos.y, ncell(pos).map(|c| d[c]), goal.x, goal.y, t.alt); }
        if len < 0.05 { t.moving = false; continue; }
        let dir = v / len;
        let mut np = pos + dir * (speed * dt).min(len);
        // não anda por dentro de outro invasor
        for (j, q) in &others { if *j != i { let w = np - *q; let l = w.length(); if l < 0.5 && l > 0.001 { np += w / l * (0.5 - l) * 0.5; } } }
        level.resolve_circle(&mut np, 0.28);
        // porta fechada no caminho: abre
        for dr in level.doors.iter_mut() { if !dr.opening && Vec2::new(dr.cx, dr.cz).distance(np) < 1.3 { dr.opening = true; sfx.write(Sfx::Door(Vec3::new(dr.cx, 1.0, dr.cz))); } }
        let mv = np - pos;
        let face = if mv.length() > 0.004 { mv.normalize() } else { dir };
        let f = Vec2::new(crate::map::damp(t.fwd.x, face.x, 12.0, dt), crate::map::damp(t.fwd.y, face.y, 12.0, dt)).normalize_or_zero();
        t.def.x = np.x; t.def.z = np.y; t.def.fx = np.x + f.x; t.def.fz = np.y + f.y; t.fwd = f; t.moving = true;
        if let Some(z) = zone_at(np.x, np.y) { t.def.zone = zones()[z].id; }
        // nos degraus o corpo acompanha a altura do piso
        if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(np.x, ramp_h(np.x, np.y), np.y); tf.rotation = Quat::from_rotation_y(f.x.atan2(f.y)); }
    }
}

/* ------------------------------------------------------------------ granadas ------------------------------------------------------------------ */

/// Granada lançada por uma ameaça: voa em arco até o ponto de queda e estoura depois do pavio.
#[derive(Component)]
pub struct Grenade { from: Vec3, to: Vec3, t: f32, fuse: f32 }
const NADE_FLY: f32 = 0.9;
const NADE_FUSE: f32 = 2.4;
/// Raios do estouro: perto disso abate; até o segundo, fere. Parede e móvel alto entre a granada e o operador protegem.
const NADE_KILL: f32 = 2.2;
const NADE_HURT: f32 = 4.0;
/// Quanto tempo o operador pode ficar no mesmo lugar (num raio de 1,2 m) antes de alguém jogar uma granada.
const CAMP_SECS: f32 = 4.0;

#[derive(Default)]
pub struct NadeState { camp: Vec2, camp_t: f32, cd: f32, tick: f32, last_time: f32 }

/// Contra quem fica parado: uma ameaça que sabe do operador, está perto e não tem linha de tiro joga uma granada para junto
/// dele. Vale no percurso livre, na missão 2 e nos cenários (não no tutorial nem com alvos de papelão).
#[allow(clippy::too_many_arguments)]
pub fn grenade_system(time: Res<Time>, mut game: ResMut<Game>, p: Res<Player>, level: Res<Level>, targets: Res<Targets>, fx: Res<crate::combat::Fx>,
    mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut smats: ResMut<Assets<StandardMaterial>>, mut nades: Query<(Entity, &mut Grenade, &mut Transform)>,
    mut sfx: EventWriter<Sfx>, mut gz: Gizmos, mut st: Local<NadeState>, mut art: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>) {
    let dt = time.delta_secs().min(0.05);
    if game.mode != Mode::Play {
        // partida encerrada ou zerada: some o que estiver no ar
        if matches!(game.mode, Mode::Menu | Mode::Debrief | Mode::Down | Mode::Card) { for (e, _, _) in &nades { commands.entity(e).try_despawn(); } game.nade = None; }
        return;
    }
    if game.time < st.last_time { *st = NadeState::default(); for (e, _, _) in &nades { commands.entity(e).try_despawn(); } game.nade = None; }
    st.last_time = game.time;
    let script = std::env::var("CQB_SCRIPT").is_ok();
    let nodeath = std::env::var("CQB_NODEATH").is_ok();

    /* ---------- granadas em cena ---------- */
    game.nade = None;
    for (e, mut g, mut tf) in &mut nades {
        // caiu no chão: o quique da granada avisa onde ela está
        if g.t < NADE_FLY && g.t + dt >= NADE_FLY { sfx.write(Sfx::Clink(g.to)); }
        g.t += dt;
        if g.t < NADE_FLY {
            let u = g.t / NADE_FLY;
            tf.translation = g.from.lerp(g.to, u) + Vec3::Y * (1.1 * (std::f32::consts::PI * u).sin());
            tf.rotate(Quat::from_rotation_x(9.0 * dt));
        } else {
            if g.t - dt < NADE_FLY { sfx.write(Sfx::Casing(g.to)); sfx.write(Sfx::Metal(g.to)); }
            tf.translation = g.to;
            g.fuse -= dt;
            // anel vermelho no chão: até onde o estouro fere
            let c = g.to;
            let blink = if (g.fuse * 6.0).fract() < 0.5 { 1.0 } else { 0.45 };
            for k in 0..28 {
                let (a0, a1) = (k as f32 / 28.0 * std::f32::consts::TAU, (k + 1) as f32 / 28.0 * std::f32::consts::TAU);
                gz.line(Vec3::new(c.x + a0.cos() * NADE_HURT, 0.04, c.z + a0.sin() * NADE_HURT), Vec3::new(c.x + a1.cos() * NADE_HURT, 0.04, c.z + a1.sin() * NADE_HURT), Color::srgba(1.0, 0.25, 0.2, 0.55 * blink));
                gz.line(Vec3::new(c.x + a0.cos() * 0.18, 0.05, c.z + a0.sin() * 0.18), Vec3::new(c.x + a1.cos() * 0.18, 0.05, c.z + a1.sin() * 0.18), Color::srgba(1.0, 0.3, 0.2, blink));
            }
        }
        game.nade = Some((Vec2::new(g.to.x, g.to.z), (NADE_FLY - g.t).max(0.0) + g.fuse.max(0.0)));
        if g.t >= NADE_FLY && g.fuse <= 0.0 {
            let c = g.to;
            commands.entity(e).try_despawn();
            game.nade = None;
            crate::combat::blast(&mut commands, &fx, &mut smats, c, game.time);
            commands.spawn((PointLight { color: Color::srgb(1.0, 0.8, 0.5), intensity: 2_500_000.0, range: 12.0, shadows_enabled: false, ..default() }, Transform::from_translation(c + Vec3::Y * 0.6), crate::world::light_layers(), crate::operator::EnemyFlash(0.11)));
            sfx.write(Sfx::Blast(c));
            let d = Vec2::new(c.x, c.z).distance(p.pos);
            let open = level.los(c.x, c.z, 0.5, p.pos.x, p.pos.y, 1.0) || level.los(c.x, c.z, 0.5, p.eye.x, p.eye.z, p.eye.y);
            if script { println!("BOT GRANADA estourou a {:.1} m do operador, linha livre={}", d, open); }
            game.shake = (1.6 - d * 0.2).clamp(0.3, 1.5);
            if open && d < NADE_HURT && game.down_t <= 0.0 {
                game.stats.hits_taken += 1; game.dmg_t = 1.3;
                sfx.write(Sfx::Hurt);
                game.wounds += if d < NADE_KILL { MAX_WOUNDS } else { 1 };
                if game.wounds >= MAX_WOUNDS && !nodeath {
                    game.down_t = DOWN_SECS; game.toast = None;
                    game.down_why = "A grenade went off beside you. When one lands, leave the spot at once and put a wall between you and it.".into();
                } else { game.toast("Hit by a grenade (simulated). You stayed in one place too long: when one lands, get a wall between you and it.", 2, 6.0); }
            }
        }
    }

    /* ---------- granadas pedidas por quem vai entrar por uma porta que já custou caro ---------- */
    let reqs: Vec<(Vec3, Vec3)> = std::mem::take(&mut game.nade_req);
    for (from, to) in reqs {
        let (mesh, mat) = art.get_or_insert_with(|| (meshes.add(Sphere::new(0.045).mesh().uv(12, 8)), smats.add(StandardMaterial { base_color: Color::srgb(0.10, 0.14, 0.08), perceptual_roughness: 0.6, metallic: 0.3, ..default() }))).clone();
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_translation(from).with_scale(Vec3::new(1.0, 1.35, 1.0)), bevy::pbr::NotShadowCaster, Grenade { from, to, t: 0.0, fuse: NADE_FUSE }));
        sfx.write(Sfx::Rustle);
        st.cd = st.cd.max(6.0);
    }

    /* ---------- quem fica parado leva granada ---------- */
    if game.demo || !game.free || !want_figs() || game.down_t > 0.0 { return; }
    if p.pos.distance(st.camp) > 1.2 { st.camp = p.pos; st.camp_t = 0.0; } else { st.camp_t += dt; }
    st.cd = (st.cd - dt).max(0.0);
    st.tick -= dt;
    if st.tick > 0.0 || st.cd > 0.0 || st.camp_t < CAMP_SECS || !nades.is_empty() { return; }
    st.tick = 0.25;
    for t in targets.list.iter().filter(|t| t.hostile() && !is_mech(&t.def) && t.mind == MIND_NONE && !t.shield && !t.captor && t.pend <= 0.0 && t.rise <= 0.0 && (t.alert > 0.0 || t.hunter)) {
        let tp = Vec2::new(t.def.x, t.def.z);
        let d = tp.distance(p.pos);
        if !(3.0..11.0).contains(&d) { continue; }
        // quem tem linha de tiro atira; a granada é de quem não tem
        if level.los(tp.x, tp.y, 1.4, p.pos.x, p.pos.y, 1.2) || level.los(tp.x, tp.y, 1.4, p.eye.x, p.eye.z, p.eye.y) { continue; }
        // ponto de queda: perto do operador, à vista de quem lança e com linha livre até o operador
        let mut best: Option<(f32, Vec2)> = None;
        for r in [1.0f32, 1.8, 2.6] {
            for k in 0..10 {
                let a = k as f32 / 10.0 * std::f32::consts::TAU;
                let q = p.pos + Vec2::new(a.cos(), a.sin()) * r;
                if zone_at(q.x, q.y).is_none() || blocked(&level, q, 0.15) { continue; }
                if !level.los(tp.x, tp.y, 1.5, q.x, q.y, 0.4) || !level.los(q.x, q.y, 0.4, p.pos.x, p.pos.y, 1.0) { continue; }
                if best.map_or(true, |b| r < b.0) { best = Some((r, q)); }
            }
            if best.is_some() { break; }
        }
        let Some((_, q)) = best else { continue };
        let (mesh, mat) = art.get_or_insert_with(|| (meshes.add(Sphere::new(0.045).mesh().uv(12, 8)), smats.add(StandardMaterial { base_color: Color::srgb(0.10, 0.14, 0.08), perceptual_roughness: 0.6, metallic: 0.3, ..default() }))).clone();
        let from = Vec3::new(tp.x, 1.5, tp.y);
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::from_translation(from).with_scale(Vec3::new(1.0, 1.35, 1.0)), bevy::pbr::NotShadowCaster, Grenade { from, to: Vec3::new(q.x, 0.05, q.y), t: 0.0, fuse: NADE_FUSE }));
        sfx.write(Sfx::Rustle);
        st.cd = if map().defense { 12.0 } else if difficulty() == 2 { 7.0 } else { 10.0 };
        st.camp_t = CAMP_SECS * 0.5;
        if script { println!("BOT GRANADA lançada por {} de {:.1} m para ({:.1},{:.1})", t.def.id, d, q.x, q.y); }
        break;
    }
}

/* ------------------------------------------------------------------ pontuação e ranking ------------------------------------------------------------------ */

/// Nota final da partida (0 a 100). Nos cenários entra o ajuste próprio de cada um; na defesa da casa valem os invasores derrubados.
pub fn final_score(game: &Game, targets: &Targets) -> i32 {
    if is(game, Kind::Defense) { return ((game.stats.threats_down * 100) as f32 / invaders_total() as f32).round().min(100.0) as i32; }
    let s = &game.stats;
    let left = if is(game, Kind::Rescue) { 0 } else { targets.list.iter().filter(|t| t.def.kind == TKind::Threat && !t.def.passive && !t.off && t.hostile()).count() as i32 };
    (game.base_score().round() as i32 - s.civ_hit as i32 * 20 - s.hits_taken as i32 * 10 - s.failed as i32 * 5 - left * 8 + game.scn_adj).clamp(0, 100)
}

/// Tabela do ranking em que a partida entra.
pub fn board(game: &Game) -> String {
    match game.scn { Some(i) => SCENARIOS[i].name.to_string(), None if game.mission2 => "Mission 2".to_string(), None if game.free => format!("Free run · {}", map().name), None => "Tutorial".to_string() }
}

#[derive(Clone, Debug)]
pub struct RankRow { pub board: String, pub score: i32, pub time: f32, pub diff: u8, pub ts: u64 }
/// Ranking guardado em disco (uma linha por partida terminada): mais pontos primeiro; no empate, menos tempo.
#[derive(Resource, Default)]
pub struct Ranking { pub rows: Vec<RankRow>, path: Option<std::path::PathBuf> }
impl Ranking {
    pub fn load() -> Self {
        // nos testes automáticos o ranking do jogador não é tocado, a não ser que CQB_RANK aponte um arquivo
        let path = match std::env::var("CQB_RANK") {
            Ok(f) => Some(std::path::PathBuf::from(f)),
            Err(_) if std::env::var("CQB_SCRIPT").is_ok() => None,
            Err(_) => std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Angles/ranking.tsv")),
        };
        let mut rows = Vec::new();
        if let Some(text) = path.as_ref().and_then(|f| std::fs::read_to_string(f).ok()) {
            for l in text.lines() {
                let c: Vec<&str> = l.split('\t').collect();
                if c.len() < 5 { continue; }
                if let (Ok(score), Ok(time), Ok(diff), Ok(ts)) = (c[1].parse(), c[2].parse(), c[3].parse(), c[4].parse()) { rows.push(RankRow { board: c[0].to_string(), score, time, diff, ts }); }
            }
        }
        Ranking { rows, path }
    }
    fn order(a: &RankRow, b: &RankRow) -> std::cmp::Ordering { b.score.cmp(&a.score).then(a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)).then(a.ts.cmp(&b.ts)) }
    /// As melhores partidas de uma tabela.
    pub fn top(&self, board: &str, n: usize) -> Vec<&RankRow> {
        let mut v: Vec<&RankRow> = self.rows.iter().filter(|r| r.board == board).collect();
        v.sort_by(|a, b| Self::order(a, b));
        v.truncate(n);
        v
    }
    pub fn boards(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for r in &self.rows { if !v.contains(&r.board) { v.push(r.board.clone()); } }
        v
    }
    /// Registra uma partida e devolve a colocação dela (1 = melhor) e o total de partidas na tabela.
    pub fn add(&mut self, board: &str, score: i32, time: f32, diff: u8) -> (usize, usize) {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let row = RankRow { board: board.to_string(), score, time, diff, ts };
        let better = self.rows.iter().filter(|r| r.board == board && Self::order(r, &row) == std::cmp::Ordering::Less).count();
        self.rows.push(row);
        if let Some(f) = &self.path {
            if let Some(dir) = f.parent() { let _ = std::fs::create_dir_all(dir); }
            let text: String = self.rows.iter().map(|r| format!("{}\t{}\t{:.1}\t{}\t{}\n", r.board, r.score, r.time, r.diff, r.ts)).collect();
            let _ = std::fs::write(f, text);
        }
        (better + 1, self.rows.iter().filter(|r| r.board == board).count())
    }
}
/// Nível em que a partida foi jogada (a fase do mech tem o dela: 3 = very hard).
pub fn run_diff(game: &Game) -> u8 { if is(game, Kind::Walker) { 3 } else { difficulty() } }
pub fn fmt_time(t: f32) -> String { format!("{}:{:02}", (t / 60.0) as i32, (t % 60.0) as i32) }

/// Ao chegar à avaliação (ou ao cair na defesa da casa, onde vale quantos invasores foram derrubados), a partida entra no ranking.
pub fn rank_system(mut game: ResMut<Game>, targets: Res<Targets>, mut rk: ResMut<Ranking>, net: Res<crate::net::Net>) {
    if game.ranked || game.demo { return; }
    let done = game.mode == Mode::Debrief || (game.mode == Mode::Down && is(&game, Kind::Defense));
    if !done { return; }
    game.ranked = true;
    let (score, b) = (final_score(&game, &targets), board(&game));
    game.rank = Some(rk.add(&b, score, game.time, run_diff(&game)));
    // ranking mundial e contagem de uso (em segundo plano; sem internet nada muda)
    net.submit(&b, score, game.time, run_diff(&game));
    net.event("end", Some(&b), score >= 60, game.time);
    game.ui_dirty = true;
    if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT RANKING {}: {} pts em {} -> {:?}", b, score, fmt_time(game.time), game.rank); }
}

/// Texto do cartão de instruções: do cenário, se houver um em curso, ou da lição.
pub fn card(game: &Game) -> (&'static str, String, &'static str, &'static [&'static str]) {
    match game.scn {
        Some(i) => { let s = &SCENARIOS[i]; (s.kicker, s.name.to_uppercase(), s.lead, s.points) }
        None => { let l = &LESSONS[game.lesson.min(5)]; (l.kicker, l.title.to_uppercase(), l.lead, l.points) }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Menor distância entre a casa do ponto e as vizinhas (quem nasce colado numa parede anda primeiro até a casa livre ao lado).
    fn near_dist(nav: &Nav, alt: usize, q: Vec2) -> u32 {
        let mut best = u32::MAX;
        for dz in -1..=1 { for dx in -1..=1 { if let Some(c) = ncell(q + Vec2::new(dx as f32, dz as f32) * NC) { best = best.min(nav.dist[alt][c]); } } }
        best
    }

    /// Todos os mapas: dados conferidos (alvos nos cômodos certos, vagas, portas sobre vãos, tudo alcançável a pé).
    #[test]
    fn all_maps_valid() {
        for i in 0..MAPS.len() {
            set_map(i);
            let errs = Level::new().validate();
            assert!(errs.is_empty(), "mapa {} ({}): {:?}", i, map().name, errs);
        }
        assert_eq!(MAPS.iter().map(|m| m.name).collect::<Vec<_>>(), ["Training house", "Apartment", "Office", "Warehouse", "Hallway", "Family home"]);
    }

    /// Casa térrea: dados do mapa conferidos, todo invasor chega a pé a qualquer cômodo (pelos dois caminhos),
    /// ninguém atravessa janela, e a visão passa por cima do peitoril mas não por baixo.
    #[test]
    fn home_map_routes() {
        set_map(5);
        let level = Level::new();
        let errs = level.validate();
        assert!(errs.is_empty(), "{:?}", errs);
        let mut nav = Nav::default();
        nav.build(&level);
        for from in [Vec2::new(12.1, 5.0), Vec2::new(6.0, 0.7), Vec2::new(4.5, 10.5), Vec2::new(2.5, 3.0), Vec2::new(6.0, 3.0), Vec2::new(8.5, 5.0)] {
            nav.flow(from);
            for t in map().targets {
                for alt in 0..2 {
                    assert!(near_dist(&nav, alt, Vec2::new(t.x, t.z)) != u32::MAX, "{} sem caminho (alt {}) até {:?}", t.id, alt, from);
                }
            }
        }
        let windows: Vec<&Gap> = level.gaps.iter().filter(|g| is_window(&level, g)).collect();
        assert_eq!(windows.len(), 6);
        for g in &windows { let c = gxy(g, (g.a + g.b) / 2.0, g.c); assert!(!nav.walk[ncell(c).unwrap()], "janela atravessável em {:?}", c); }
        assert!(level.los(2.2, 0.7, 1.4, 2.2, 2.5, 1.4), "a janela do corredor para a suíte 1 deveria deixar ver");
        assert!(!level.los(2.2, 0.7, 0.5, 2.2, 2.5, 0.5), "abaixo do peitoril não deveria dar para ver");
        assert!(!level.los(3.2, 0.7, 1.4, 3.2, 2.5, 1.4), "parede sem janela deveria barrar");
        for g in level.gaps.iter().filter(|g| !is_window(&level, g)) {
            for s in [-0.8f32, 0.8] { let (_, pivot, to) = slice_plan(&nav, g, gxy(g, (g.a + g.b) / 2.0, g.c + s)); assert!(pivot.is_finite() && to.is_finite()); }
        }
        // posto de janela: do quarto principal (onde o operador começa) há uma janela de onde ele é visto, e todo invasor chega lá
        let posts: Vec<Vec2> = windows.iter().flat_map(|g| [-0.6f32, 0.6].map(|sd| gxy(g, (g.a + g.b) / 2.0, g.c + sd)))
            .filter(|q| ncell(*q).map_or(false, |c| nav.walk[c]) && level.los(q.x, q.y, 1.4, 12.1, 5.0, 1.62)).collect();
        assert!(!posts.is_empty(), "nenhuma janela vê o quarto principal");
        let f = nav.field(posts[0]);
        for t in map().targets {
            let q = Vec2::new(t.x, t.z);
            let mut best = u32::MAX;
            for dz in -1..=1 { for dx in -1..=1 { if let Some(c) = ncell(q + Vec2::new(dx as f32, dz as f32) * NC) { best = best.min(f[c]); } } }
            assert!(best != u32::MAX, "{} não chega ao posto de janela {:?}", t.id, posts[0]);
        }
        // planta térrea: sem passagens; o corredor liga a sala aos quartos e a suíte principal tem duas portas
        assert!(map().portals.is_empty() && map().ramps.is_empty());
        for q in [Vec2::new(5.0, 7.0), Vec2::new(12.0, 6.5), Vec2::new(5.0, 10.5), Vec2::new(12.0, 10.0), Vec2::new(4.0, 15.0), Vec2::new(11.0, 5.0), Vec2::new(2.5, 3.0), Vec2::new(4.0, 0.7)] {
            assert!(zone_at(q.x, q.y).is_some() && ncell(q).map_or(false, |c| nav.walk[c]), "{:?} deveria ser piso livre", q);
        }
        let ms_doors = level.gaps.iter().filter(|g| !is_window(&level, g)).filter(|g| (!g.horizontal && (g.c - 9.3).abs() < 0.01 && g.a < 2.0) || (g.horizontal && (g.c - 6.0).abs() < 0.01 && g.a > 10.0)).count();
        assert_eq!(ms_doors, 2, "a suíte principal deveria ter duas portas");
        // porta marcada: com o custo extra, o caminho alternativo ainda existe para todo mundo
        nav.hot = vec![false; NW * NH];
        let g = level.gaps.iter().find(|g| !g.horizontal && (g.c - 9.3).abs() < 0.01 && !is_window(&level, g)).unwrap();
        for k in 0..NW * NH { let (u, v) = guv(g, ncenter(k)); if u > g.a - 0.1 && u < g.b + 0.1 && (v - g.c).abs() < 0.3 { nav.hot[k] = true; } }
        nav.flow(Vec2::new(12.1, 5.0));
        for t in map().targets { let q = Vec2::new(t.x, t.z); assert!(near_dist(&nav, 1, q) != u32::MAX && near_dist(&nav, 0, q) != u32::MAX); }
    }
}
