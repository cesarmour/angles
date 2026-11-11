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
pub enum PropKind { Bench, Crate, Barrel, Sofa, Table, Shelf, Locker, Cover }

#[derive(Clone, Copy, Debug)]
pub struct Prop { pub kind: PropKind, pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32, pub h: f32 }

#[derive(Clone, Copy, Debug)]
pub struct Gap { pub horizontal: bool, pub c: f32, pub a: f32, pub b: f32 }

#[derive(Clone, Debug)]
pub struct Door { pub id: &'static str, pub hx: f32, pub hz: f32, pub len: f32, pub cx: f32, pub cz: f32, pub t: f32, pub open: f32, pub opening: bool, pub solid: usize }

#[derive(Clone, Copy, Debug)]
pub struct Zone { pub id: &'static str, pub name: &'static str, pub x1: f32, pub z1: f32, pub x2: f32, pub z2: f32 }

#[derive(Clone, Copy, Debug)]
pub struct Doorway { pub zone: &'static str, pub x1: f32, pub x2: f32, pub z1: f32, pub z2: f32, pub cx: f32, pub cz: f32, pub axis_z: bool, pub v: f32, pub s: f32 }

#[derive(Clone, Copy, Debug)]
pub struct LightDef { pub x: f32, pub z: f32, pub color: [f32; 3], pub i: f32, pub rot: bool, pub flicker: bool }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TKind { Threat, Civil }

#[derive(Clone, Copy, Debug)]
pub struct TargetDef { pub id: &'static str, pub kind: TKind, pub v: u8, pub x: f32, pub z: f32, pub fx: f32, pub fz: f32, pub zone: &'static str, pub passive: bool }

#[derive(Clone, Copy, Debug)]
pub struct Corner { pub zone: &'static str, pub x: f32, pub z: f32, pub seen: bool }

pub const ZONES: [Zone; 7] = [
    Zone { id: "S", name: "Staging area", x1: 0.0, z1: 12.0, x2: 6.0, z2: 18.0 },
    Zone { id: "C1", name: "Corridor 1", x1: 4.0, z1: 6.0, x2: 6.0, z2: 12.0 },
    Zone { id: "C2", name: "Corridor 2", x1: 4.0, z1: 4.0, x2: 14.0, z2: 6.0 },
    Zone { id: "A", name: "Room A", x1: 6.0, z1: 6.0, x2: 14.0, z2: 12.0 },
    Zone { id: "B", name: "Room B", x1: 14.0, z1: 6.0, x2: 21.0, z2: 12.0 },
    Zone { id: "C3", name: "Corridor 3", x1: 14.0, z1: 4.0, x2: 21.0, z2: 6.0 },
    Zone { id: "C", name: "Room C", x1: 15.0, z1: 0.0, x2: 21.0, z2: 4.0 },
];
pub fn zone_index(id: &str) -> usize { ZONES.iter().position(|z| z.id == id).unwrap_or(0) }
pub fn zone_at(x: f32, z: f32) -> Option<usize> { ZONES.iter().position(|q| x >= q.x1 && x < q.x2 && z >= q.z1 && z < q.z2) }

pub const DOORWAYS: [Doorway; 4] = [
    Doorway { zone: "A", x1: 9.4, x2: 10.6, z1: 5.55, z2: 6.45, cx: 10.0, cz: 6.0, axis_z: true, v: 6.12, s: 1.0 },
    Doorway { zone: "B", x1: 13.55, x2: 14.45, z1: 10.4, z2: 11.5, cx: 14.0, cz: 10.95, axis_z: false, v: 14.12, s: 1.0 },
    Doorway { zone: "C3", x1: 17.0, x2: 18.1, z1: 5.55, z2: 6.45, cx: 17.55, cz: 6.0, axis_z: true, v: 5.88, s: -1.0 },
    Doorway { zone: "C", x1: 19.0, x2: 20.1, z1: 3.55, z2: 4.45, cx: 19.55, cz: 4.0, axis_z: true, v: 3.88, s: -1.0 },
];

const WARM: [f32; 3] = [1.0, 0.89, 0.75];
const COOL: [f32; 3] = [0.9, 0.93, 1.0];
const NEUTRAL: [f32; 3] = [1.0, 0.94, 0.86];
pub const LIGHTS: [LightDef; 8] = [
    LightDef { x: 3.0, z: 15.0, color: WARM, i: 1.0, rot: false, flicker: false },
    LightDef { x: 5.0, z: 9.0, color: COOL, i: 0.8, rot: true, flicker: false },
    LightDef { x: 6.6, z: 5.0, color: COOL, i: 0.75, rot: false, flicker: false },
    LightDef { x: 11.4, z: 5.0, color: COOL, i: 0.75, rot: false, flicker: false },
    LightDef { x: 10.0, z: 9.0, color: NEUTRAL, i: 1.05, rot: false, flicker: false },
    LightDef { x: 17.5, z: 9.0, color: NEUTRAL, i: 1.05, rot: false, flicker: false },
    LightDef { x: 17.5, z: 5.0, color: COOL, i: 0.8, rot: false, flicker: false },
    LightDef { x: 19.0, z: 2.0, color: WARM, i: 0.9, rot: false, flicker: false },
];

const fn tg(id: &'static str, kind: TKind, v: u8, x: f32, z: f32, fx: f32, fz: f32, zone: &'static str, passive: bool) -> TargetDef {
    TargetDef { id, kind, v, x, z, fx, fz, zone, passive }
}
/// Abertura a ser fatiada (porta ou boca de corredor). `a` e `b` são as pontas do vão na linha da parede e
/// `n` aponta para o espaço que fica além dela. O espaço além de uma abertura ocupa um leque de 180°.
#[derive(Clone, Copy, Debug)]
pub struct Opening { pub name: &'static str, pub zone: &'static str, pub a: Vec2, pub b: Vec2, pub n: Vec2, pub door: Option<usize> }
pub const OPENINGS: [Opening; 5] = [
    Opening { name: "Corridor corner", zone: "C2", a: Vec2::new(6.0, 4.1), b: Vec2::new(6.0, 5.9), n: Vec2::new(1.0, 0.0), door: None },
    Opening { name: "Room A door", zone: "A", a: Vec2::new(9.4, 6.0), b: Vec2::new(10.6, 6.0), n: Vec2::new(0.0, 1.0), door: None },
    Opening { name: "Room B door", zone: "B", a: Vec2::new(14.0, 10.4), b: Vec2::new(14.0, 11.5), n: Vec2::new(1.0, 0.0), door: None },
    Opening { name: "Corridor 3 door", zone: "C3", a: Vec2::new(17.0, 6.0), b: Vec2::new(18.1, 6.0), n: Vec2::new(0.0, -1.0), door: Some(0) },
    Opening { name: "Room C door", zone: "C", a: Vec2::new(19.0, 4.0), b: Vec2::new(20.1, 4.0), n: Vec2::new(0.0, -1.0), door: Some(1) },
];
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

/// Posições alternativas por zona (x, z, olha-para-x, olha-para-z), sorteadas no percurso livre e na missão 2.
pub const SLOTS: [(&str, f32, f32, f32, f32); 23] = [
    ("C2", 12.9, 5.0, 6.0, 5.0), ("C2", 11.0, 4.5, 6.0, 5.3), ("C2", 13.3, 5.5, 7.0, 4.8),
    ("A", 13.2, 11.2, 10.0, 7.0), ("A", 13.45, 6.5, 10.6, 9.2), ("A", 6.6, 6.5, 9.5, 9.0), ("A", 6.7, 11.3 - 0.9, 10.0, 7.5), ("A", 11.6, 9.6, 10.0, 6.5),
    ("B", 20.2, 6.85, 16.0, 10.0), ("B", 14.75, 6.85, 15.4, 11.0), ("B", 17.6, 7.7, 15.5, 10.6), ("B", 20.3, 11.2, 16.0, 9.6), ("B", 19.6, 9.9, 15.0, 11.0), ("B", 15.6, 8.3, 14.6, 11.0),
    ("C3", 14.6, 4.7, 17.5, 5.6), ("C3", 20.5, 4.6, 17.6, 5.6), ("C3", 15.6, 5.4, 17.5, 5.5), ("C3", 19.2, 5.5, 17.6, 5.6),
    ("C", 15.8, 3.2, 19.0, 3.5), ("C", 16.2, 0.9, 17.6, 3.4), ("C", 20.4, 0.7, 19.5, 3.8), ("C", 18.6, 0.6, 19.5, 3.6), ("C", 15.6, 1.9, 17.0, 3.3),
];
pub const TARGETS: [TargetDef; 13] = [
    tg("P0", TKind::Threat, 0, 3.0, 12.8, 3.0, 16.0, "S", true),
    tg("T1", TKind::Threat, 1, 12.9, 5.0, 6.0, 5.0, "C2", false),
    tg("T2", TKind::Threat, 0, 13.2, 11.2, 10.0, 7.0, "A", false),
    tg("T3", TKind::Threat, 1, 13.45, 6.5, 10.6, 9.2, "A", false),
    tg("T4", TKind::Threat, 0, 20.2, 6.85, 16.0, 10.0, "B", false),
    tg("T5", TKind::Threat, 1, 14.75, 6.85, 15.4, 11.0, "B", false),
    tg("N1", TKind::Civil, 0, 17.6, 7.7, 15.5, 10.6, "B", false),
    tg("N2", TKind::Civil, 1, 20.3, 11.2, 16.0, 9.6, "B", false),
    tg("T6", TKind::Threat, 0, 14.6, 4.7, 17.5, 5.6, "C3", false),
    tg("T7", TKind::Threat, 1, 20.5, 4.6, 17.6, 5.6, "C3", false),
    tg("N3", TKind::Civil, 0, 15.8, 3.2, 19.0, 3.5, "C", false),
    tg("T8", TKind::Threat, 1, 16.2, 0.9, 17.6, 3.4, "C", false),
    tg("T9", TKind::Threat, 0, 20.4, 0.7, 19.5, 3.8, "C", false),
];

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
    pub fn new() -> Self {
        let mut walls = Vec::new();
        let mut lintels = Vec::new();
        let mut gaps = Vec::new();
        let t = WALL_T / 2.0;
        let mk = |x1: f32, z1: f32, x2: f32, z2: f32| Solid { x1, z1, x2, z2, h: WALL_H, on: true, kind: SolidKind::Wall };
        let mut wall_h = |z: f32, xa: f32, xb: f32, gs: &[(f32, f32)]| {
            let mut cur = xa - t;
            let end = xb + t;
            for g in gs {
                if g.0 > cur { walls.push(mk(cur, z - t, g.0, z + t)); }
                lintels.push(mk(g.0, z - t, g.1, z + t));
                gaps.push(Gap { horizontal: true, c: z, a: g.0, b: g.1 });
                cur = g.1;
            }
            if end > cur { walls.push(mk(cur, z - t, end, z + t)); }
        };
        // Percurso: Preparação → Corredor 1 → quina → Corredor 2 → Sala A → Sala B → Corredor 3 (T) → Sala C.
        wall_h(18.0, 0.0, 6.0, &[]);
        wall_h(12.0, 0.0, 4.0, &[]);
        wall_h(4.0, 4.0, 21.0, &[(19.0, 20.1)]);
        wall_h(6.0, 6.0, 21.0, &[(9.4, 10.6), (17.0, 18.1)]);
        wall_h(12.0, 6.0, 21.0, &[]);
        wall_h(0.0, 15.0, 21.0, &[]);
        let mut wall_v = |x: f32, za: f32, zb: f32, gs: &[(f32, f32)]| {
            let mut cur = za - t;
            let end = zb + t;
            for g in gs {
                if g.0 > cur { walls.push(mk(x - t, cur, x + t, g.0)); }
                lintels.push(mk(x - t, g.0, x + t, g.1));
                gaps.push(Gap { horizontal: false, c: x, a: g.0, b: g.1 });
                cur = g.1;
            }
            if end > cur { walls.push(mk(x - t, cur, x + t, end)); }
        };
        wall_v(0.0, 12.0, 18.0, &[]);
        wall_v(6.0, 6.0, 18.0, &[]);
        wall_v(4.0, 4.0, 12.0, &[]);
        wall_v(14.0, 4.0, 12.0, &[(10.4, 11.5)]);
        wall_v(21.0, 0.0, 12.0, &[]);
        wall_v(15.0, 0.0, 4.0, &[]);

        let p = |kind: PropKind, x1: f32, z1: f32, x2: f32, z2: f32, h: f32| Prop { kind, x1, z1, x2, z2, h };
        // as medidas seguem os modelos 3D usados em cada móvel
        let props = vec![
            p(PropKind::Bench, 0.2, 13.1, 1.12, 15.62, 0.77),
            p(PropKind::Crate, 4.4, 17.1, 5.64, 17.62, 0.46),
            p(PropKind::Crate, 0.3, 16.3, 0.82, 17.54, 0.46),
            p(PropKind::Barrel, 13.2, 4.22, 13.72, 4.74, 0.88),
            // cobertura de 1,25 m: em pé você vê por cima; agachado, ela esconde você e tapa a sua visão
            p(PropKind::Cover, 9.4, 9.2, 10.6, 9.75, 1.25),
            p(PropKind::Sofa, 6.9, 11.1, 8.7, 11.86, 0.9),
            p(PropKind::Barrel, 6.25, 9.6, 6.77, 10.12, 0.88),
            p(PropKind::Table, 16.6, 8.45, 18.6, 9.4, 0.79),
            p(PropKind::Shelf, 16.2, 11.42, 18.18, 11.87, 1.93),
            p(PropKind::Locker, 17.2, 0.1, 17.87, 2.12, 2.24),
            p(PropKind::Crate, 20.3, 2.3, 20.82, 3.54, 0.46),
            p(PropKind::Barrel, 14.25, 9.0, 14.77, 9.52, 0.88),
            p(PropKind::Cover, 1.9, 16.2, 3.1, 16.75, 1.25),
        ];
        let mut solids: Vec<Solid> = walls.clone();
        for q in &props { solids.push(Solid { x1: q.x1, z1: q.z1, x2: q.x2, z2: q.z2, h: q.h, on: true, kind: SolidKind::Prop }); }
        let mut doors = Vec::new();
        for (id, hx, hz) in [("D1", 17.0f32, 6.0f32), ("D2", 19.0, 4.0)] {
            let len = 1.1;
            solids.push(Solid { x1: hx, z1: hz - 0.05, x2: hx + len, z2: hz + 0.05, h: DOOR_H, on: true, kind: SolidKind::Door });
            doors.push(Door { id, hx, hz, len, cx: hx + len / 2.0, cz: hz, t: 0.0, open: 0.0, opening: false, solid: solids.len() - 1 });
        }
        let c = |zone: &'static str, x: f32, z: f32| Corner { zone, x, z, seen: false };
        let corners = vec![c("A", 6.5, 6.5), c("A", 13.5, 6.5), c("B", 14.5, 6.5), c("B", 14.5, 11.6), c("C", 20.6, 3.6), c("C", 15.4, 3.6)];
        Level { walls, lintels, gaps, props, solids, doors, corners }
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
