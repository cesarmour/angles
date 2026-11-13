//! Planta da casa de tiro, colisão e visibilidade (2D com altura: quem abaixa passa a ser coberto por móveis baixos).
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

pub const WALL_T: f32 = 0.2;
pub const WALL_H: f32 = 2.8;
pub const DOOR_H: f32 = 2.08;
pub const EYE_STAND: f32 = 1.62;
pub const EYE_CROUCH: f32 = 1.05;
pub const PLAYER_R: f32 = 0.3;
pub const BODY_R: f32 = 0.25;
pub const MAP_X1: f32 = -0.5;
pub const MAP_Z1: f32 = -0.5;
pub const MAP_X2: f32 = 21.5;
pub const MAP_Z2: f32 = 18.5;
pub const VIS_RANGE: f32 = 30.0;

pub fn ang_diff(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % TAU;
    if d > PI { d -= TAU } else if d < -PI { d += TAU }
    d
}
pub fn damp(a: f32, b: f32, l: f32, dt: f32) -> f32 { a + (b - a) * (1.0 - (-l * dt).exp()) }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SolidKind { Wall, Prop, Door }

#[derive(Clone, Copy, Debug)]
pub struct Solid { pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32, pub h: f32, pub on: bool, pub kind: SolidKind }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// `Sill` = peitoril de janela: meia parede no vão (barra a passagem; a visão, o tiro e a granada passam por cima).
pub enum PropKind { Bench, Crate, Barrel, Sofa, Table, Shelf, Locker, Cover, Bed, Pool, Rail, Sill }

#[derive(Clone, Copy, Debug)]
pub struct Prop { pub kind: PropKind, pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32, pub h: f32 }

#[derive(Clone, Copy, Debug)]
pub struct Gap { pub horizontal: bool, pub c: f32, pub a: f32, pub b: f32 }

#[derive(Clone, Debug)]
pub struct Door { pub id: &'static str, pub hx: f32, pub hz: f32, pub len: f32, pub cx: f32, pub cz: f32, pub t: f32, pub open: f32, pub opening: bool, pub solid: usize }

#[derive(Clone, Copy, Debug)]
pub struct Zone { pub id: &'static str, pub name: &'static str, pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32 }

/// Vão de entrada de um cômodo: caixa do vão, lado de entrada (`v`, `s`) e o índice da abertura (leque) correspondente.
#[derive(Clone, Copy, Debug)]
pub struct Doorway { pub zone: &'static str, pub x1: f32, pub x2: f32, pub z1: f32, pub z2: f32, pub cx: f32, pub cz: f32, pub axis_z: bool, pub v: f32, pub s: f32, pub opening: usize }

#[derive(Clone, Copy, Debug)]
pub struct LightDef { pub x: f32, pub z: f32, pub color: [f32; 3], pub i: f32, pub rot: bool, pub flicker: bool }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TKind { Threat, Civil }

#[derive(Clone, Copy, Debug)]
pub struct TargetDef { pub id: &'static str, pub kind: TKind, pub v: u8, pub x: f32, pub z: f32, pub fx: f32, pub fz: f32, pub zone: &'static str, pub passive: bool }

#[derive(Clone, Copy, Debug)]
pub struct Corner { pub zone: &'static str, pub x: f32, pub z: f32, pub seen: bool }

impl Zone {
    /// Cômodo estreito e comprido (corredor): muda a reverberação do som.
    pub fn is_corridor(&self) -> bool { (self.x2 - self.x1).min(self.z2 - self.z1) <= 2.5 }
}

/// Abertura a ser fatiada (porta ou boca de corredor). `a` e `b` são as pontas do vão na linha da parede e
/// `n` aponta para o espaço que fica além dela. O espaço além de uma abertura ocupa um leque de 180°.
#[derive(Clone, Copy, Debug)]
pub struct Opening { pub name: &'static str, pub zone: &'static str, pub a: Vec2, pub b: Vec2, pub n: Vec2, pub door: Option<usize> }
impl Opening {
    pub fn mid(&self) -> Vec2 { (self.a + self.b) / 2.0 }
    pub fn w(&self) -> Vec2 { (self.b - self.a).normalize() }
    /// Ângulo de entrada de uma direção, de 0° (rente à parede, para o lado de `b`) a 180° (rente, para o lado de `a`).
    pub fn phi(&self, dir: Vec2) -> f32 { dir.dot(self.n).atan2(dir.dot(self.w())).to_degrees() }
    pub fn dir(&self, phi: f32) -> Vec2 { let r = phi.to_radians(); self.w() * r.cos() + self.n * r.sin() }
    /// Leque de direções (em graus, dentro dos 180°) que passam pelo vão a partir do olho, considerando a espessura da parede.
    pub fn fan(&self, eye: Vec2) -> Option<(f32, f32)> {
        let h = WALL_T / 2.0;
        if (eye - self.mid()).dot(self.n) > -h - 0.02 { return None; }
        let (an, bn, af, bf) = (self.a - self.n * h, self.b - self.n * h, self.a + self.n * h, self.b + self.n * h);
        let lo = self.phi(bn - eye).max(self.phi(bf - eye));
        let hi = self.phi(an - eye).min(self.phi(af - eye));
        if hi - lo > 0.2 { Some((lo.max(0.0), hi.min(180.0))) } else { None }
    }
}

/* ------------------------------------------------------------------ mapas ------------------------------------------------------------------
   Cada mapa é uma tabela de dados. Todos cabem na mesma área (0..21 m em x, 0..18 m em z) e têm no máximo 8 cômodos, 8 vãos de entrada
   e 4 portas com folha (sempre em parede horizontal, vão de 1,1 m). O mapa 0 é a casa do tutorial. */
/// Parede: coordenada fixa, início, fim e vãos (portas) ao longo dela.
pub type WallSpec = (f32, f32, f32, &'static [(f32, f32)]);
/// Posição possível de um alvo: cômodo, x, z e para onde ele olha.
pub type Slot = (&'static str, f32, f32, f32, f32);
pub struct MapDef {
    pub name: &'static str, pub blurb: &'static str,
    pub zones: &'static [Zone], pub doorways: &'static [Doorway], pub openings: &'static [Opening], pub lights: &'static [LightDef],
    pub targets: &'static [TargetDef], pub slots: &'static [Slot],
    pub walls_h: &'static [WallSpec], pub walls_v: &'static [WallSpec], pub props: &'static [Prop],
    pub doors: &'static [(&'static str, f32, f32)], pub corners: &'static [(&'static str, f32, f32)],
    /// Onde o operador começa (x, z) e para onde olha (ângulo).
    pub start: (f32, f32, f32),
    /// Aparece na escolha de mapa do percurso livre; mapa de defesa (ondas de invasores, sem percurso livre);
    /// passagens entre dois pontos do mapa (escada entre andares desenhados lado a lado).
    pub pick: bool, pub defense: bool, pub portals: &'static [Portal],
    /// Escadas de verdade: trechos em rampa, o vão aberto no piso do andar de cima (x1, z1, x2, z2, profundidade) e os vãos
    /// escuros por onde a escada continua para o outro andar (caixas x1, y1, z1, x2, y2, z2).
    pub ramps: &'static [Ramp], pub pit: Option<(f32, f32, f32, f32, f32)>, pub darks: &'static [(f32, f32, f32, f32, f32, f32)],
}
/// Passagem entre andares: quem pisa no retângulo (x1, z1, x2, z2) sai em (ex, ez) olhando para `yaw`. `player` = o
/// operador também usa (escada); senão só os invasores (escada de mão até a sacada). `alt` = só metade dos invasores
/// conhece este caminho. `ha` = para onde o invasor some na partida (atrás do vão escuro da escada; na escada de mão, o pé
/// dela) e `hb` = de onde ele sai na chegada (atrás do vão escuro de lá).
#[derive(Clone, Copy, Debug)]
pub struct Portal { pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32, pub ex: f32, pub ez: f32, pub yaw: f32, pub player: bool, pub alt: bool,
    pub ha: (f32, f32), pub hb: (f32, f32),
    /// Escada de verdade: pontos (x, z, altura) por onde o invasor segue depois de entrar no vão (`pa`, até sumir escada
    /// acima ou depois da curva) e por onde ele vem antes de sair no outro andar (`pb`). Vazios = some/aparece em `ha`/`hb`.
    pub pa: &'static [(f32, f32, f32)], pub pb: &'static [(f32, f32, f32)] }
/// Trecho de escada: dentro do retângulo, a altura do piso vai de `ha` (na coordenada `a`) a `hb` (na coordenada `b`), ao
/// longo de x (`along_x`) ou de z; além de `a` e de `b` fica constante. Vale para os olhos do operador e para os invasores.
#[derive(Clone, Copy, Debug)]
pub struct Ramp { pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32, pub along_x: bool, pub a: f32, pub b: f32, pub ha: f32, pub hb: f32 }
/// Altura do piso em (x, z): zero fora das escadas.
pub fn ramp_h(x: f32, z: f32) -> f32 {
    for r in map().ramps {
        if x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2 {
            let c = if r.along_x { x } else { z };
            let k = if (r.b - r.a).abs() < 1e-4 { 0.0 } else { ((c - r.a) / (r.b - r.a)).clamp(0.0, 1.0) };
            return r.ha + (r.hb - r.ha) * k;
        }
    }
    0.0
}
pub const MAX_ZONES: usize = 12;
pub const MAX_DOORWAYS: usize = 12;
pub const MAX_DOORS: usize = 4;

static CUR_MAP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub fn map_index() -> usize { CUR_MAP.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_map(i: usize) { CUR_MAP.store(i.min(MAPS.len() - 1), std::sync::atomic::Ordering::Relaxed); }
pub fn map() -> &'static MapDef { &MAPS[map_index()] }
pub fn zones() -> &'static [Zone] { map().zones }
pub fn doorways() -> &'static [Doorway] { map().doorways }
pub fn openings() -> &'static [Opening] { map().openings }
pub fn lights() -> &'static [LightDef] { map().lights }
pub fn target_defs() -> &'static [TargetDef] { map().targets }
pub fn slots() -> &'static [Slot] { map().slots }
/// O mech do projeto ("Walker"): alvo cujo identificador começa com M. Só entra em cena no cenário dele.
pub fn is_mech(def: &TargetDef) -> bool { def.id.starts_with('M') }
pub fn zone_index(id: &str) -> usize { zones().iter().position(|z| z.id == id).unwrap_or(0) }
/// Paredes de OSB (a kill house de madeira): escondem, mas não param projétil.
pub fn soft_walls() -> bool { map_index() == 5 }
pub fn zone_at(x: f32, z: f32) -> Option<usize> { zones().iter().position(|q| x >= q.x1 && x < q.x2 && z >= q.z1 && z < q.z2) }

pub const WARM: [f32; 3] = [1.0, 0.89, 0.75];
pub const COOL: [f32; 3] = [0.9, 0.93, 1.0];
pub const NEUTRAL: [f32; 3] = [1.0, 0.94, 0.86];
pub const fn zn(id: &'static str, name: &'static str, x1: f32, z1: f32, x2: f32, z2: f32) -> Zone { Zone { id, name, x1, z1, x2, z2 } }
/// Vão em parede horizontal (em z = `z`, de `a` a `b` em x); `s` = +1 se o cômodo fica do lado de z maior.
pub const fn dh(zone: &'static str, a: f32, b: f32, z: f32, s: f32, opening: usize) -> Doorway {
    Doorway { zone, x1: a, x2: b, z1: z - 0.45, z2: z + 0.45, cx: (a + b) / 2.0, cz: z, axis_z: true, v: z + s * 0.12, s, opening }
}
/// Vão em parede vertical (em x = `x`, de `a` a `b` em z); `s` = +1 se o cômodo fica do lado de x maior.
pub const fn dv(zone: &'static str, x: f32, a: f32, b: f32, s: f32, opening: usize) -> Doorway {
    Doorway { zone, x1: x - 0.45, x2: x + 0.45, z1: a, z2: b, cx: x, cz: (a + b) / 2.0, axis_z: false, v: x + s * 0.12, s, opening }
}
#[allow(clippy::too_many_arguments)]
pub const fn op(name: &'static str, zone: &'static str, ax: f32, az: f32, bx: f32, bz: f32, nx: f32, nz: f32, door: Option<usize>) -> Opening {
    Opening { name, zone, a: Vec2::new(ax, az), b: Vec2::new(bx, bz), n: Vec2::new(nx, nz), door }
}
pub const fn lt(x: f32, z: f32, color: [f32; 3], i: f32, rot: bool) -> LightDef { LightDef { x, z, color, i, rot, flicker: false } }
pub const fn pr(kind: PropKind, x1: f32, z1: f32, x2: f32, z2: f32, h: f32) -> Prop { Prop { kind, x1, z1, x2, z2, h } }
#[allow(clippy::too_many_arguments)]
pub const fn tg(id: &'static str, kind: TKind, v: u8, x: f32, z: f32, fx: f32, fz: f32, zone: &'static str, passive: bool) -> TargetDef {
    TargetDef { id, kind, v, x, z, fx, fz, zone, passive }
}
pub use crate::maps::MAPS;

#[derive(Resource)]
pub struct Level {
    pub walls: Vec<Solid>,
    pub lintels: Vec<Solid>,
    pub gaps: Vec<Gap>,
    pub props: Vec<Prop>,
    pub solids: Vec<Solid>,
    pub doors: Vec<Door>,
    pub corners: Vec<Corner>,
}

impl Level {
    /// Monta paredes, móveis, portas e cantos do mapa atual.
    pub fn new() -> Self {
        let m = map();
        let mut walls = Vec::new();
        let mut lintels = Vec::new();
        let mut gaps = Vec::new();
        let t = WALL_T / 2.0;
        let mk = |x1: f32, z1: f32, x2: f32, z2: f32| Solid { x1, z1, x2, z2, h: WALL_H, on: true, kind: SolidKind::Wall };
        for &(z, xa, xb, gs) in m.walls_h {
            let mut cur = xa - t;
            let end = xb + t;
            for g in gs {
                if g.0 > cur { walls.push(mk(cur, z - t, g.0, z + t)); }
                lintels.push(mk(g.0, z - t, g.1, z + t));
                gaps.push(Gap { horizontal: true, c: z, a: g.0, b: g.1 });
                cur = g.1;
            }
            if end > cur { walls.push(mk(cur, z - t, end, z + t)); }
        }
        for &(x, za, zb, gs) in m.walls_v {
            let mut cur = za - t;
            let end = zb + t;
            for g in gs {
                if g.0 > cur { walls.push(mk(x - t, cur, x + t, g.0)); }
                lintels.push(mk(x - t, g.0, x + t, g.1));
                gaps.push(Gap { horizontal: false, c: x, a: g.0, b: g.1 });
                cur = g.1;
            }
            if end > cur { walls.push(mk(x - t, cur, x + t, end)); }
        }
        let props = m.props.to_vec();
        let mut solids: Vec<Solid> = walls.clone();
        for q in &props { solids.push(Solid { x1: q.x1, z1: q.z1, x2: q.x2, z2: q.z2, h: q.h, on: true, kind: SolidKind::Prop }); }
        let mut doors = Vec::new();
        for &(id, hx, hz) in m.doors {
            let len = 1.1;
            solids.push(Solid { x1: hx, z1: hz - 0.05, x2: hx + len, z2: hz + 0.05, h: DOOR_H, on: true, kind: SolidKind::Door });
            doors.push(Door { id, hx, hz, len, cx: hx + len / 2.0, cz: hz, t: 0.0, open: 0.0, opening: false, solid: solids.len() - 1 });
        }
        let corners = m.corners.iter().map(|&(zone, x, z)| Corner { zone, x, z, seen: false }).collect();
        Level { walls, lintels, gaps, props, solids, doors, corners }
    }

    /// Confere os dados do mapa atual: alvos dentro do cômodo certo e fora de paredes e móveis, vagas suficientes por cômodo,
    /// portas sobre vãos e tudo alcançável a pé a partir do ponto de partida. Devolve a lista de problemas.
    pub fn validate(&self) -> Vec<String> {
        let m = map();
        let mut errs = Vec::new();
        let blocked = |x: f32, z: f32, r: f32, doors: bool| self.solids.iter().any(|s| s.on && (doors || s.kind != SolidKind::Door) && x > s.x1 - r && x < s.x2 + r && z > s.z1 - r && z < s.z2 + r);
        for (zone, x, z, _, _) in m.slots {
            if zone_at(*x, *z) != Some(zone_index(zone)) { errs.push(format!("vaga ({x},{z}) fora do cômodo {zone}")); }
            if blocked(*x, *z, 0.28, true) { errs.push(format!("vaga ({x},{z}) do cômodo {zone} colada em parede ou móvel")); }
        }
        for z in m.zones {
            let need = m.targets.iter().filter(|t| t.zone == z.id && !t.passive).count();
            let have = m.slots.iter().filter(|s| s.0 == z.id).count();
            if have < need { errs.push(format!("cômodo {} tem {need} alvos e só {have} vagas", z.id)); }
        }
        for t in m.targets { if zone_at(t.x, t.z) != Some(zone_index(t.zone)) { errs.push(format!("alvo {} fora do cômodo {}", t.id, t.zone)); } }
        for &(id, hx, hz) in m.doors {
            if !self.gaps.iter().any(|g| g.horizontal && (g.c - hz).abs() < 0.01 && (g.a - hx).abs() < 0.01 && (g.b - hx - 1.1).abs() < 0.02) { errs.push(format!("porta {id} sem vão de 1,1 m em ({hx},{hz})")); }
        }
        // alcance a pé (portas contam como abertas)
        let (cell, w, h) = (0.2f32, ((MAP_X2 - MAP_X1) / 0.2) as usize, ((MAP_Z2 - MAP_Z1) / 0.2) as usize);
        let free = |i: usize, j: usize| !blocked(MAP_X1 + (i as f32 + 0.5) * cell, MAP_Z1 + (j as f32 + 0.5) * cell, PLAYER_R, false) && zone_at(MAP_X1 + (i as f32 + 0.5) * cell, MAP_Z1 + (j as f32 + 0.5) * cell).is_some();
        let idx = |x: f32, z: f32| (((x - MAP_X1) / cell) as usize, ((z - MAP_Z1) / cell) as usize);
        let mut seen = vec![false; w * h];
        let (si, sj) = idx(m.start.0, m.start.1);
        if !free(si, sj) { errs.push("ponto de partida bloqueado".into()); }
        let mut stack = vec![(si, sj)];
        seen[sj * w + si] = true;
        while let Some((i, j)) = stack.pop() {
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni >= w as i32 || nj >= h as i32 { continue; }
                let (ni, nj) = (ni as usize, nj as usize);
                if !seen[nj * w + ni] && free(ni, nj) { seen[nj * w + ni] = true; stack.push((ni, nj)); }
            }
        }
        for z in m.zones {
            let (i, j) = idx((z.x1 + z.x2) / 2.0, (z.z1 + z.z2) / 2.0);
            let near = (-4i32..=4).any(|a| (-4i32..=4).any(|b| { let (x, y) = ((i as i32 + a).clamp(0, w as i32 - 1) as usize, (j as i32 + b).clamp(0, h as i32 - 1) as usize); seen[y * w + x] }));
            if !near && m.portals.is_empty() { errs.push(format!("cômodo {} inalcançável", z.id)); }
        }
        for (zone, x, z, _, _) in m.slots {
            let (i, j) = idx(*x, *z);
            let near = (-2i32..=2).any(|a| (-2i32..=2).any(|b| { let (xx, yy) = ((i as i32 + a).clamp(0, w as i32 - 1) as usize, (j as i32 + b).clamp(0, h as i32 - 1) as usize); seen[yy * w + xx] }));
            if !near && m.portals.is_empty() { errs.push(format!("vaga ({x},{z}) do cômodo {zone} inalcançável")); }
        }
        errs
    }

    pub fn reset(&mut self) {
        for i in 0..self.doors.len() {
            let s = self.doors[i].solid;
            self.doors[i].t = 0.0; self.doors[i].open = 0.0; self.doors[i].opening = false;
            self.solids[s].on = true;
        }
        for c in &mut self.corners { c.seen = false; }
    }

    /// Distância até o primeiro sólido mais alto que `eye_h` (use 0.0 para considerar todos).
    pub fn cast_ray(&self, ox: f32, oz: f32, dx: f32, dz: f32, max_d: f32, eye_h: f32) -> f32 {
        let mut best = max_d;
        for b in &self.solids {
            if !b.on || b.h <= eye_h { continue; }
            if let Some((t0, _)) = ray_box(ox, oz, dx, dz, b) { if t0 < best { best = t0; } }
        }
        best
    }

    /// Linha de visada entre dois pontos com alturas: um móvel só bloqueia se a linha passar abaixo do topo dele.
    pub fn los(&self, ax: f32, az: f32, ah: f32, bx: f32, bz: f32, bh: f32) -> bool {
        let (dx, dz) = (bx - ax, bz - az);
        let d = (dx * dx + dz * dz).sqrt();
        if d < 1e-4 { return true; }
        let (ux, uz) = (dx / d, dz / d);
        for b in &self.solids {
            if !b.on { continue; }
            if let Some((t0, t1)) = ray_box(ax, az, ux, uz, b) {
                if t0 >= d - 0.06 { continue; }
                let t1 = t1.min(d);
                let h0 = ah + (bh - ah) * (t0 / d);
                let h1 = ah + (bh - ah) * (t1 / d);
                if h0.min(h1) < b.h { return false; }
            }
        }
        true
    }

    pub fn resolve_circle(&self, p: &mut Vec2, r: f32) {
        for _ in 0..3 {
            for b in &self.solids {
                if !b.on { continue; }
                let cx = p.x.clamp(b.x1, b.x2);
                let cz = p.y.clamp(b.z1, b.z2);
                let (dx, dz) = (p.x - cx, p.y - cz);
                let d2 = dx * dx + dz * dz;
                if d2 >= r * r { continue; }
                if d2 > 1e-10 {
                    let d = d2.sqrt();
                    p.x = cx + dx / d * r; p.y = cz + dz / d * r;
                } else {
                    let (l, rg, t, bt) = (p.x - b.x1, b.x2 - p.x, p.y - b.z1, b.z2 - p.y);
                    let m = l.min(rg).min(t).min(bt);
                    if m == l { p.x = b.x1 - r } else if m == rg { p.x = b.x2 + r } else if m == t { p.y = b.z1 - r } else { p.y = b.z2 + r }
                }
            }
        }
    }
}

/// Interseção raio × caixa no plano: devolve (entrada, saída) ao longo do raio.
pub fn ray_box(ox: f32, oz: f32, dx: f32, dz: f32, b: &Solid) -> Option<(f32, f32)> {
    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    if dx.abs() < 1e-9 { if ox < b.x1 || ox > b.x2 { return None; } } else {
        let (mut t1, mut t2) = ((b.x1 - ox) / dx, (b.x2 - ox) / dx);
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1); tmax = tmax.min(t2);
        if tmin > tmax { return None; }
    }
    if dz.abs() < 1e-9 { if oz < b.z1 || oz > b.z2 { return None; } } else {
        let (mut t1, mut t2) = ((b.z1 - oz) / dz, (b.z2 - oz) / dz);
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1); tmax = tmax.min(t2);
        if tmin > tmax { return None; }
    }
    Some((tmin, tmax))
}

/* ---------- polígono de visibilidade ---------- */
#[derive(Default, Clone)]
pub struct Vis { pub ox: f32, pub oz: f32, pub ang: Vec<f32>, pub dist: Vec<f32> }

pub fn compute_vis(level: &Level, ox: f32, oz: f32, eye_h: f32) -> Vis {
    let e = 2e-4f32;
    let mut ang: Vec<f32> = Vec::with_capacity(600);
    for b in &level.solids {
        if !b.on || b.h <= eye_h { continue; }
        for (cx, cz) in [(b.x1, b.z1), (b.x2, b.z1), (b.x2, b.z2), (b.x1, b.z2)] {
            let a = (cz - oz).atan2(cx - ox);
            ang.push(a - e); ang.push(a); ang.push(a + e);
        }
    }
    ang.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let dist = ang.iter().map(|a| level.cast_ray(ox, oz, a.cos(), a.sin(), VIS_RANGE, eye_h)).collect();
    Vis { ox, oz, ang, dist }
}

/// Borda livre: a linha de visada raspa a quina `a` e segue até `b`. `hidden_deg` é o ângulo ainda oculto atrás dela.
#[derive(Clone, Copy, Debug)]
pub struct Edge { pub a: Vec2, pub b: Vec2, pub sdir: Vec2, pub hdir: Vec2, pub hidden_deg: f32, pub dist: f32 }

#[derive(Default, Clone)]
pub struct Wedge { pub pts: Vec<Vec2>, pub rel: Vec<f32>, pub d: Vec<f32>, pub edges: Vec<Edge>, pub c: f32, pub h: f32 }

pub fn compute_wedge(level: &Level, vis: &Vis, c: f32, h: f32, eye_h: f32) -> Wedge {
    let (ox, oz) = (vis.ox, vis.oz);
    let mut w = Wedge { c, h, ..default() };
    let mut push = |w: &mut Wedge, rel: f32, d: f32| {
        let a = c + rel;
        w.rel.push(rel); w.d.push(d); w.pts.push(Vec2::new(ox + a.cos() * d, oz + a.sin() * d));
    };
    let d0 = level.cast_ray(ox, oz, (c - h).cos(), (c - h).sin(), VIS_RANGE, eye_h);
    push(&mut w, -h, d0);
    for pass in 0..3 {
        for i in 0..vis.ang.len() {
            let a = vis.ang[i];
            let raw = a - c;
            let rel = ang_diff(a, c);
            if rel <= -h || rel >= h { continue; }
            let grp = if (raw - rel).abs() < 1e-4 { 1 } else if rel < 0.0 { 0 } else { 2 };
            if grp == pass { push(&mut w, rel, vis.dist[i]); }
        }
    }
    let d1 = level.cast_ray(ox, oz, (c + h).cos(), (c + h).sin(), VIS_RANGE, eye_h);
    push(&mut w, h, d1);
    let eye = Vec2::new(ox, oz);
    for i in 0..w.pts.len().saturating_sub(1) {
        if w.rel[i + 1] - w.rel[i] < 1e-3 && (w.d[i + 1] - w.d[i]).abs() > 0.35 {
            let (j, k) = if w.d[i] < w.d[i + 1] { (i, i + 1) } else { (i + 1, i) };
            let a = w.pts[j];
            if w.edges.iter().any(|e| e.a.distance(a) < 0.3) { continue; }
            let sdir = (a - eye).normalize_or_zero();
            // face que continua atrás da quina: define o setor que ainda não se vê
            let mut best: Option<(f32, Vec2)> = None;
            for b in &level.solids {
                if !b.on || b.h <= eye_h { continue; }
                for (cx, cz) in [(b.x1, b.z1), (b.x2, b.z1), (b.x2, b.z2), (b.x1, b.z2)] {
                    if (cx - a.x).abs() > 0.03 || (cz - a.y).abs() > 0.03 { continue; }
                    let ex = Vec2::new(if cx == b.x1 { 1.0 } else { -1.0 }, 0.0);
                    let ez = Vec2::new(0.0, if cz == b.z1 { 1.0 } else { -1.0 });
                    let (wx, wz) = (b.x2 - b.x1, b.z2 - b.z1);
                    let e = if wx.min(wz) <= 0.3 { if wx > wz { ex } else { ez } } else if sdir.dot(ex) > sdir.dot(ez) { ex } else { ez };
                    // cada quina esconde no máximo um quadrante: 90° é "não abri nada", 0° é "vejo rente à parede"
                    let ang = sdir.dot(e).clamp(-1.0, 1.0).acos().to_degrees().min(90.0);
                    if best.map_or(true, |(ba, _)| ang < ba) { best = Some((ang, e)); }
                }
            }
            if let Some((ang, e)) = best {
                w.edges.push(Edge { a, b: w.pts[k], sdir, hdir: e, hidden_deg: ang, dist: (a - eye).length() });
            }
        }
    }
    w
}

impl Wedge {
    /// Distância visível na direção `rel` (interpolada entre os dois raios vizinhos).
    pub fn reach(&self, eye: Vec2, p: Vec2) -> Option<f32> {
        let rel = ang_diff((p.y - eye.y).atan2(p.x - eye.x), self.c);
        if rel.abs() > self.h || self.rel.len() < 2 { return None; }
        let i = match self.rel.binary_search_by(|v| v.partial_cmp(&rel).unwrap()) { Ok(i) => i.min(self.rel.len() - 2), Err(i) => i.clamp(1, self.rel.len() - 1) - 1 };
        Some(ray_seg(eye, p - eye, self.pts[i], self.pts[i + 1]).unwrap_or(self.d[i].min(self.d[i + 1])))
    }
    pub fn sees(&self, eye: Vec2, p: Vec2, margin: f32) -> bool {
        match self.reach(eye, p) { Some(r) => (p - eye).length() <= r + margin, None => false }
    }
}

/// Parâmetro (em múltiplos de |dir|) onde o raio cruza o segmento, já convertido para distância.
fn ray_seg(o: Vec2, dir: Vec2, a: Vec2, b: Vec2) -> Option<f32> {
    let len = dir.length();
    if len < 1e-6 { return None; }
    let d = dir / len;
    let s = b - a;
    let den = d.perp_dot(s);
    if den.abs() < 1e-7 { return None; }
    let t = (a - o).perp_dot(s) / den;
    if t < 0.0 { None } else { Some(t) }
}

/// Mesmo teste para o polígono de 360° (usado para a "exposição": o que tem linha de tiro para o operador).
pub fn vis_sees(vis: &Vis, p: Vec2, margin: f32) -> bool {
    let n = vis.ang.len();
    if n < 2 { return false; }
    let eye = Vec2::new(vis.ox, vis.oz);
    let a = (p.y - eye.y).atan2(p.x - eye.x);
    let i = match vis.ang.binary_search_by(|v| v.partial_cmp(&a).unwrap()) { Ok(i) => i, Err(i) => i };
    let (i0, i1) = if i == 0 || i >= n { (n - 1, 0) } else { (i - 1, i) };
    let p0 = eye + Vec2::new(vis.ang[i0].cos(), vis.ang[i0].sin()) * vis.dist[i0];
    let p1 = eye + Vec2::new(vis.ang[i1].cos(), vis.ang[i1].sin()) * vis.dist[i1];
    let r = ray_seg(eye, p - eye, p0, p1).unwrap_or(vis.dist[i0].min(vis.dist[i1]));
    (p - eye).length() <= r + margin
}
