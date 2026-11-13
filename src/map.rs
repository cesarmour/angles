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

const WARM: [f32; 3] = [1.0, 0.89, 0.75];
const COOL: [f32; 3] = [0.9, 0.93, 1.0];
const NEUTRAL: [f32; 3] = [1.0, 0.94, 0.86];
const fn zn(id: &'static str, name: &'static str, x1: f32, z1: f32, x2: f32, z2: f32) -> Zone { Zone { id, name, x1, z1, x2, z2 } }
/// Vão em parede horizontal (em z = `z`, de `a` a `b` em x); `s` = +1 se o cômodo fica do lado de z maior.
const fn dh(zone: &'static str, a: f32, b: f32, z: f32, s: f32, opening: usize) -> Doorway {
    Doorway { zone, x1: a, x2: b, z1: z - 0.45, z2: z + 0.45, cx: (a + b) / 2.0, cz: z, axis_z: true, v: z + s * 0.12, s, opening }
}
/// Vão em parede vertical (em x = `x`, de `a` a `b` em z); `s` = +1 se o cômodo fica do lado de x maior.
const fn dv(zone: &'static str, x: f32, a: f32, b: f32, s: f32, opening: usize) -> Doorway {
    Doorway { zone, x1: x - 0.45, x2: x + 0.45, z1: a, z2: b, cx: x, cz: (a + b) / 2.0, axis_z: false, v: x + s * 0.12, s, opening }
}
#[allow(clippy::too_many_arguments)]
const fn op(name: &'static str, zone: &'static str, ax: f32, az: f32, bx: f32, bz: f32, nx: f32, nz: f32, door: Option<usize>) -> Opening {
    Opening { name, zone, a: Vec2::new(ax, az), b: Vec2::new(bx, bz), n: Vec2::new(nx, nz), door }
}
const fn lt(x: f32, z: f32, color: [f32; 3], i: f32, rot: bool) -> LightDef { LightDef { x, z, color, i, rot, flicker: false } }
const fn pr(kind: PropKind, x1: f32, z1: f32, x2: f32, z2: f32, h: f32) -> Prop { Prop { kind, x1, z1, x2, z2, h } }
#[allow(clippy::too_many_arguments)]
const fn tg(id: &'static str, kind: TKind, v: u8, x: f32, z: f32, fx: f32, fz: f32, zone: &'static str, passive: bool) -> TargetDef {
    TargetDef { id, kind, v, x, z, fx, fz, zone, passive }
}
use PropKind::*;
use TKind::{Civil, Threat};

pub static MAPS: [MapDef; 6] = [
    /* ---------- 0 · casa do tutorial: Preparação → Corredor 1 → quina → Corredor 2 → Sala A → Sala B → Corredor 3 (T) → Sala C ---------- */
    MapDef {
        name: "Training house", blurb: "the tutorial shoot house",
        zones: &[
            zn("S", "Staging area", 0.0, 12.0, 6.0, 18.0), zn("C1", "Corridor 1", 4.0, 6.0, 6.0, 12.0), zn("C2", "Corridor 2", 4.0, 4.0, 14.0, 6.0),
            zn("A", "Room A", 6.0, 6.0, 14.0, 12.0), zn("B", "Room B", 14.0, 6.0, 21.0, 12.0), zn("C3", "Corridor 3", 14.0, 4.0, 21.0, 6.0), zn("C", "Room C", 15.0, 0.0, 21.0, 4.0),
        ],
        doorways: &[dh("A", 9.4, 10.6, 6.0, 1.0, 1), dv("B", 14.0, 10.4, 11.5, 1.0, 2), dh("C3", 17.0, 18.1, 6.0, -1.0, 3), dh("C", 19.0, 20.1, 4.0, -1.0, 4)],
        openings: &[
            op("Corridor corner", "C2", 6.0, 4.1, 6.0, 5.9, 1.0, 0.0, None), op("Room A door", "A", 9.4, 6.0, 10.6, 6.0, 0.0, 1.0, None),
            op("Room B door", "B", 14.0, 10.4, 14.0, 11.5, 1.0, 0.0, None), op("Corridor 3 door", "C3", 17.0, 6.0, 18.1, 6.0, 0.0, -1.0, Some(0)),
            op("Room C door", "C", 19.0, 4.0, 20.1, 4.0, 0.0, -1.0, Some(1)),
        ],
        lights: &[lt(3.0, 15.0, WARM, 1.0, false), lt(5.0, 9.0, COOL, 0.8, true), lt(6.6, 5.0, COOL, 0.75, false), lt(11.4, 5.0, COOL, 0.75, false),
            lt(10.0, 9.0, NEUTRAL, 1.05, false), lt(17.5, 9.0, NEUTRAL, 1.05, false), lt(17.5, 5.0, COOL, 0.8, false), lt(19.0, 2.0, WARM, 0.9, false)],
        targets: &[
            tg("P0", Threat, 0, 3.0, 12.8, 3.0, 16.0, "S", true), tg("T1", Threat, 1, 12.9, 5.0, 6.0, 5.0, "C2", false),
            tg("T2", Threat, 0, 13.2, 11.2, 10.0, 7.0, "A", false), tg("T3", Threat, 1, 13.45, 6.5, 10.6, 9.2, "A", false),
            tg("T4", Threat, 0, 20.2, 6.85, 16.0, 10.0, "B", false), tg("T5", Threat, 1, 14.75, 6.85, 15.4, 11.0, "B", false),
            tg("N1", Civil, 0, 17.6, 7.7, 15.5, 10.6, "B", false), tg("N2", Civil, 1, 20.3, 11.2, 16.0, 9.6, "B", false),
            tg("T6", Threat, 0, 14.6, 4.7, 17.5, 5.6, "C3", false), tg("T7", Threat, 1, 20.5, 4.6, 17.6, 5.6, "C3", false),
            tg("N3", Civil, 0, 15.8, 3.2, 19.0, 3.5, "C", false), tg("T8", Threat, 1, 16.2, 0.9, 17.6, 3.4, "C", false), tg("T9", Threat, 0, 20.4, 0.7, 19.5, 3.8, "C", false),
        ],
        slots: &[
            ("C2", 12.9, 5.0, 6.0, 5.0), ("C2", 11.0, 4.5, 6.0, 5.3), ("C2", 13.3, 5.5, 7.0, 4.8),
            ("A", 13.2, 11.2, 10.0, 7.0), ("A", 13.45, 6.5, 10.6, 9.2), ("A", 6.6, 6.5, 9.5, 9.0), ("A", 6.7, 10.4, 10.0, 7.5), ("A", 11.6, 9.6, 10.0, 6.5),
            ("B", 20.2, 6.85, 16.0, 10.0), ("B", 14.75, 6.85, 15.4, 11.0), ("B", 17.6, 7.7, 15.5, 10.6), ("B", 20.3, 11.2, 16.0, 9.6), ("B", 19.6, 9.9, 15.0, 11.0), ("B", 15.6, 8.3, 14.6, 11.0),
            ("C3", 14.6, 4.7, 17.5, 5.6), ("C3", 20.5, 4.6, 17.6, 5.6), ("C3", 15.6, 5.4, 17.5, 5.5), ("C3", 19.2, 5.5, 17.6, 5.6),
            ("C", 15.8, 3.2, 19.0, 3.5), ("C", 16.2, 0.9, 17.6, 3.4), ("C", 20.4, 0.7, 19.5, 3.8), ("C", 18.6, 0.6, 19.5, 3.6), ("C", 15.6, 1.9, 17.0, 3.3),
        ],
        walls_h: &[(18.0, 0.0, 6.0, &[]), (12.0, 0.0, 4.0, &[]), (4.0, 4.0, 21.0, &[(19.0, 20.1)]), (6.0, 6.0, 21.0, &[(9.4, 10.6), (17.0, 18.1)]), (12.0, 6.0, 21.0, &[]), (0.0, 15.0, 21.0, &[])],
        walls_v: &[(0.0, 12.0, 18.0, &[]), (6.0, 6.0, 18.0, &[]), (4.0, 4.0, 12.0, &[]), (14.0, 4.0, 12.0, &[(10.4, 11.5)]), (21.0, 0.0, 12.0, &[]), (15.0, 0.0, 4.0, &[])],
        // as medidas seguem os modelos 3D usados em cada móvel; a cobertura de 1,25 m deixa ver por cima em pé e esconde quem agacha
        props: &[
            pr(Bench, 0.2, 13.1, 1.12, 15.62, 0.77), pr(Crate, 4.4, 17.1, 5.64, 17.62, 0.46), pr(Crate, 0.3, 16.3, 0.82, 17.54, 0.46), pr(Barrel, 13.2, 4.22, 13.72, 4.74, 0.88),
            pr(Cover, 9.4, 9.2, 10.6, 9.75, 1.25), pr(Sofa, 6.9, 11.1, 8.7, 11.86, 0.9), pr(Barrel, 6.25, 9.6, 6.77, 10.12, 0.88), pr(Table, 16.6, 8.45, 18.6, 9.4, 0.79),
            pr(Shelf, 16.2, 11.42, 18.18, 11.87, 1.93), pr(Locker, 17.2, 0.1, 17.87, 2.12, 2.24), pr(Crate, 20.3, 2.3, 20.82, 3.54, 0.46), pr(Barrel, 14.25, 9.0, 14.77, 9.52, 0.88),
            pr(Cover, 1.9, 16.2, 3.1, 16.75, 1.25),
        ],
        doors: &[("D1", 17.0, 6.0), ("D2", 19.0, 4.0)],
        corners: &[("A", 6.5, 6.5), ("A", 13.5, 6.5), ("B", 14.5, 6.5), ("B", 14.5, 11.6), ("C", 20.6, 3.6), ("C", 15.4, 3.6)],
        start: (3.0, 15.4, 0.0), pick: true, defense: false, portals: &[], ramps: &[], pit: None, darks: &[],
    },
    /* ---------- 1 · apartamento: hall → corredor central com três cômodos de um lado e dois do outro ---------- */
    MapDef {
        name: "Apartment", blurb: "tight rooms off a central corridor, two closed doors",
        zones: &[
            zn("H", "Entry hall", 0.0, 8.0, 4.0, 14.0), zn("K", "Corridor", 0.0, 6.0, 16.0, 8.0), zn("L", "Living room", 0.0, 0.0, 7.0, 6.0), zn("Kt", "Kitchen", 7.0, 0.0, 11.0, 6.0),
            zn("B1", "Bedroom 1", 11.0, 0.0, 16.0, 6.0), zn("B2", "Bedroom 2", 4.0, 8.0, 10.0, 14.0), zn("St", "Storage", 10.0, 8.0, 16.0, 14.0),
        ],
        doorways: &[dh("L", 3.0, 4.2, 6.0, -1.0, 1), dh("Kt", 8.4, 9.5, 6.0, -1.0, 2), dh("B1", 13.0, 14.1, 6.0, -1.0, 3), dh("B2", 6.0, 7.1, 8.0, 1.0, 4), dh("St", 12.4, 13.5, 8.0, 1.0, 5)],
        openings: &[
            op("Corridor mouth", "K", 0.1, 8.0, 3.9, 8.0, 0.0, -1.0, None), op("Living room door", "L", 3.0, 6.0, 4.2, 6.0, 0.0, -1.0, None),
            op("Kitchen door", "Kt", 8.4, 6.0, 9.5, 6.0, 0.0, -1.0, Some(0)), op("Bedroom 1 door", "B1", 13.0, 6.0, 14.1, 6.0, 0.0, -1.0, None),
            op("Bedroom 2 door", "B2", 6.0, 8.0, 7.1, 8.0, 0.0, 1.0, None), op("Storage door", "St", 12.4, 8.0, 13.5, 8.0, 0.0, 1.0, Some(1)),
        ],
        lights: &[lt(2.0, 11.0, WARM, 0.9, true), lt(4.0, 7.0, COOL, 0.75, false), lt(11.0, 7.0, COOL, 0.75, false), lt(3.5, 3.0, NEUTRAL, 1.0, false),
            lt(9.0, 3.0, COOL, 0.9, true), lt(13.5, 3.0, WARM, 0.95, false), lt(7.0, 11.0, NEUTRAL, 1.0, false), lt(13.0, 11.0, COOL, 0.9, false)],
        targets: &[
            tg("T1", Threat, 1, 15.3, 6.6, 5.0, 7.0, "K", false),
            tg("T2", Threat, 0, 0.7, 0.7, 3.6, 5.0, "L", false), tg("T3", Threat, 1, 6.3, 0.7, 3.6, 5.0, "L", false), tg("N1", Civil, 0, 5.5, 3.0, 3.6, 5.6, "L", false),
            tg("T4", Threat, 0, 10.4, 0.7, 9.0, 5.0, "Kt", false),
            tg("T5", Threat, 1, 15.4, 0.7, 13.5, 5.0, "B1", false), tg("N2", Civil, 1, 13.5, 1.5, 13.5, 5.5, "B1", false),
            tg("T6", Threat, 0, 9.3, 13.3, 6.5, 9.0, "B2", false), tg("T7", Threat, 1, 4.7, 13.3, 6.5, 9.0, "B2", false),
            tg("T8", Threat, 0, 15.3, 13.3, 13.0, 9.0, "St", false), tg("N3", Civil, 0, 13.0, 12.5, 13.0, 8.5, "St", false),
        ],
        slots: &[
            ("K", 15.3, 6.6, 5.0, 7.0), ("K", 12.0, 6.6, 4.0, 7.2), ("K", 9.0, 7.4, 3.0, 7.0),
            ("L", 0.7, 0.7, 3.6, 5.0), ("L", 6.3, 0.7, 3.6, 5.0), ("L", 1.2, 3.0, 3.6, 5.0), ("L", 6.3, 5.2, 4.0, 4.0), ("L", 3.5, 1.0, 3.6, 5.5), ("L", 5.5, 3.0, 3.6, 5.6),
            ("Kt", 7.6, 0.7, 9.0, 5.0), ("Kt", 10.4, 0.7, 9.0, 5.0), ("Kt", 10.4, 5.2, 9.0, 4.0), ("Kt", 7.6, 4.6, 9.0, 5.0),
            ("B1", 11.6, 0.7, 13.5, 5.0), ("B1", 15.4, 0.7, 13.5, 5.0), ("B1", 15.4, 5.3, 13.0, 3.0), ("B1", 12.4, 5.3, 14.0, 3.0), ("B1", 13.5, 1.5, 13.5, 5.5),
            ("B2", 4.7, 13.3, 6.5, 9.0), ("B2", 9.3, 13.3, 6.5, 9.0), ("B2", 9.3, 8.7, 6.0, 11.0), ("B2", 4.6, 8.7, 7.0, 11.0), ("B2", 7.0, 12.2, 6.5, 8.5),
            ("St", 10.7, 13.0, 13.0, 9.0), ("St", 15.3, 13.3, 13.0, 9.0), ("St", 15.3, 8.7, 12.5, 11.0), ("St", 10.7, 8.7, 13.5, 11.0), ("St", 13.0, 12.5, 13.0, 8.5),
        ],
        walls_h: &[(0.0, 0.0, 16.0, &[]), (14.0, 0.0, 16.0, &[]), (6.0, 0.0, 16.0, &[(3.0, 4.2), (8.4, 9.5), (13.0, 14.1)]), (8.0, 4.0, 16.0, &[(6.0, 7.1), (12.4, 13.5)])],
        walls_v: &[(0.0, 0.0, 14.0, &[]), (16.0, 0.0, 14.0, &[]), (4.0, 8.0, 14.0, &[]), (7.0, 0.0, 6.0, &[]), (11.0, 0.0, 6.0, &[]), (10.0, 8.0, 14.0, &[])],
        props: &[
            pr(Sofa, 0.6, 5.1, 2.4, 5.86, 0.9), pr(Table, 8.2, 2.4, 10.2, 3.35, 0.79), pr(Locker, 11.2, 2.0, 11.87, 4.02, 2.24), pr(Cover, 6.4, 10.6, 7.6, 11.15, 1.25),
            pr(Shelf, 12.0, 13.42, 13.98, 13.87, 1.93), pr(Crate, 10.3, 10.0, 10.82, 11.24, 0.46), pr(Barrel, 15.2, 7.3, 15.72, 7.82, 0.88), pr(Crate, 0.3, 12.4, 0.82, 13.64, 0.46),
        ],
        doors: &[("D1", 8.4, 6.0), ("D2", 12.4, 8.0)],
        corners: &[],
        start: (2.0, 12.8, 0.0), pick: true, defense: false, portals: &[], ramps: &[], pit: None, darks: &[],
    },
    /* ---------- 2 · escritório: recepção, salão aberto com divisórias baixas, corredor e salas fechadas ---------- */
    MapDef {
        name: "Office", blurb: "open-plan floor with low partitions, corridor and three closed doors",
        zones: &[
            zn("Lb", "Lobby", 0.0, 8.0, 5.0, 16.0), zn("Of", "Open office", 5.0, 8.0, 14.0, 16.0), zn("Co", "Corridor", 0.0, 6.0, 20.0, 8.0), zn("M", "Meeting room", 0.0, 0.0, 8.0, 6.0),
            zn("S", "Server room", 8.0, 0.0, 13.0, 6.0), zn("G", "Manager's office", 13.0, 0.0, 20.0, 6.0), zn("E", "Storage", 14.0, 8.0, 20.0, 16.0),
        ],
        doorways: &[dh("Co", 1.9, 3.1, 8.0, -1.0, 0), dv("Of", 5.0, 12.4, 13.6, 1.0, 1), dh("Of", 9.0, 10.2, 8.0, 1.0, 2), dh("M", 3.4, 4.5, 6.0, -1.0, 3),
            dh("S", 10.0, 11.1, 6.0, -1.0, 4), dh("G", 16.0, 17.2, 6.0, -1.0, 5), dh("E", 16.4, 17.5, 8.0, 1.0, 6)],
        openings: &[
            op("Lobby door", "Co", 1.9, 8.0, 3.1, 8.0, 0.0, -1.0, None), op("Office door (lobby)", "Of", 5.0, 12.4, 5.0, 13.6, 1.0, 0.0, None),
            op("Office door (corridor)", "Of", 9.0, 8.0, 10.2, 8.0, 0.0, 1.0, None), op("Meeting room door", "M", 3.4, 6.0, 4.5, 6.0, 0.0, -1.0, Some(0)),
            op("Server room door", "S", 10.0, 6.0, 11.1, 6.0, 0.0, -1.0, Some(1)), op("Manager's office door", "G", 16.0, 6.0, 17.2, 6.0, 0.0, -1.0, None),
            op("Storage door", "E", 16.4, 8.0, 17.5, 8.0, 0.0, 1.0, Some(2)),
        ],
        lights: &[lt(2.5, 12.0, WARM, 0.95, true), lt(8.0, 12.0, NEUTRAL, 1.0, false), lt(11.5, 12.0, NEUTRAL, 1.0, false), lt(4.0, 7.0, COOL, 0.75, false), lt(10.0, 7.0, COOL, 0.75, false),
            lt(16.0, 7.0, COOL, 0.75, false), lt(4.0, 3.0, NEUTRAL, 1.0, false), lt(10.5, 3.0, COOL, 0.85, true), lt(16.5, 3.0, WARM, 0.95, false), lt(17.0, 12.0, COOL, 0.9, true)],
        targets: &[
            tg("T1", Threat, 1, 19.3, 7.0, 10.0, 7.0, "Co", false),
            tg("T2", Threat, 0, 13.3, 15.3, 8.0, 10.0, "Of", false), tg("T3", Threat, 1, 5.7, 8.8, 10.0, 12.0, "Of", false), tg("T4", Threat, 0, 9.5, 15.3, 9.5, 9.0, "Of", false), tg("N1", Civil, 0, 11.5, 11.2, 6.0, 13.0, "Of", false),
            tg("T5", Threat, 1, 0.7, 0.7, 4.0, 5.0, "M", false), tg("N2", Civil, 1, 4.0, 1.0, 4.0, 5.5, "M", false),
            tg("T6", Threat, 0, 12.3, 0.7, 10.5, 5.0, "S", false),
            tg("T7", Threat, 1, 19.3, 0.7, 16.5, 5.0, "G", false), tg("N3", Civil, 0, 16.5, 1.2, 16.6, 5.5, "G", false),
            tg("T8", Threat, 0, 19.3, 15.3, 17.0, 9.0, "E", false), tg("T9", Threat, 1, 14.7, 8.7, 18.0, 12.0, "E", false),
        ],
        slots: &[
            ("Co", 19.3, 7.0, 10.0, 7.0), ("Co", 0.7, 7.0, 8.0, 7.0), ("Co", 12.5, 6.6, 5.0, 7.2), ("Co", 6.5, 7.4, 12.0, 7.0),
            ("Of", 13.3, 15.3, 8.0, 10.0), ("Of", 13.3, 8.7, 8.0, 13.0), ("Of", 5.7, 15.3, 9.0, 11.0), ("Of", 5.7, 8.8, 10.0, 12.0), ("Of", 9.5, 15.3, 9.5, 9.0), ("Of", 11.5, 11.2, 6.0, 13.0), ("Of", 7.5, 9.8, 6.0, 13.2),
            ("M", 0.7, 0.7, 4.0, 5.0), ("M", 7.3, 0.7, 4.0, 5.0), ("M", 7.3, 5.3, 3.0, 3.0), ("M", 0.7, 5.3, 5.0, 3.0), ("M", 4.0, 1.0, 4.0, 5.5),
            ("S", 8.7, 0.7, 10.5, 5.0), ("S", 12.3, 0.7, 10.5, 5.0), ("S", 12.3, 5.3, 10.0, 3.0), ("S", 8.7, 5.3, 11.0, 3.0),
            ("G", 13.7, 0.7, 16.5, 5.0), ("G", 19.3, 0.7, 16.5, 5.0), ("G", 19.3, 5.3, 15.0, 3.0), ("G", 14.6, 5.3, 18.0, 3.0), ("G", 16.5, 1.2, 16.6, 5.5),
            ("E", 14.7, 15.0, 17.0, 9.0), ("E", 19.3, 15.3, 17.0, 9.0), ("E", 19.3, 8.7, 16.0, 12.0), ("E", 14.7, 8.7, 18.0, 12.0), ("E", 17.0, 14.0, 17.0, 8.5),
        ],
        walls_h: &[(0.0, 0.0, 20.0, &[]), (16.0, 0.0, 20.0, &[]), (6.0, 0.0, 20.0, &[(3.4, 4.5), (10.0, 11.1), (16.0, 17.2)]), (8.0, 0.0, 20.0, &[(1.9, 3.1), (9.0, 10.2), (16.4, 17.5)])],
        walls_v: &[(0.0, 0.0, 16.0, &[]), (20.0, 0.0, 16.0, &[]), (5.0, 8.0, 16.0, &[(12.4, 13.6)]), (14.0, 8.0, 16.0, &[]), (8.0, 0.0, 6.0, &[]), (13.0, 0.0, 6.0, &[])],
        props: &[
            pr(Cover, 7.0, 10.6, 8.2, 11.15, 1.25), pr(Cover, 10.0, 12.6, 11.2, 13.15, 1.25), pr(Cover, 7.0, 14.0, 8.2, 14.55, 1.25), pr(Table, 10.5, 9.3, 12.5, 10.25, 0.79),
            pr(Table, 3.0, 2.4, 5.0, 3.35, 0.79), pr(Shelf, 8.3, 2.5, 10.28, 2.95, 1.93), pr(Table, 15.5, 2.0, 17.5, 2.95, 0.79), pr(Locker, 13.2, 2.0, 13.87, 4.02, 2.24),
            pr(Crate, 14.3, 11.0, 14.82, 12.24, 0.46), pr(Shelf, 16.0, 15.42, 17.98, 15.87, 1.93), pr(Barrel, 19.2, 11.5, 19.72, 12.02, 0.88), pr(Sofa, 0.4, 15.1, 2.2, 15.86, 0.9),
        ],
        doors: &[("D1", 3.4, 6.0), ("D2", 10.0, 6.0), ("D3", 16.4, 8.0)],
        corners: &[],
        start: (2.5, 14.2, 0.0), pick: true, defense: false, portals: &[], ramps: &[], pit: None, darks: &[],
    },
    /* ---------- 3 · galpão: doca, salão grande com fileiras de estantes altas (espaço morto), escritório, corredor de fundo e três salas ---------- */
    MapDef {
        name: "Warehouse", blurb: "big floor with tall shelving rows and dead space, office, back corridor and three rooms",
        zones: &[
            zn("D", "Loading dock", 0.0, 12.0, 6.0, 18.0), zn("W", "Warehouse floor", 6.0, 6.0, 21.0, 18.0), zn("O", "Office", 0.0, 6.0, 6.0, 12.0), zn("Bk", "Back corridor", 0.0, 4.0, 21.0, 6.0),
            zn("C", "Cold room", 0.0, 0.0, 9.0, 4.0), zn("P", "Parts room", 9.0, 0.0, 15.0, 4.0), zn("R", "Boiler room", 15.0, 0.0, 21.0, 4.0),
        ],
        doorways: &[dv("W", 6.0, 14.4, 15.8, 1.0, 0), dh("O", 2.4, 3.5, 12.0, -1.0, 1), dv("O", 6.0, 8.4, 9.6, -1.0, 2), dh("Bk", 12.0, 13.4, 6.0, -1.0, 3),
            dh("Bk", 1.0, 2.2, 6.0, -1.0, 4), dh("C", 4.0, 5.1, 4.0, -1.0, 5), dh("P", 11.4, 12.6, 4.0, -1.0, 6), dh("R", 17.4, 18.5, 4.0, -1.0, 7)],
        openings: &[
            op("Warehouse floor door", "W", 6.0, 14.4, 6.0, 15.8, 1.0, 0.0, None), op("Office door", "O", 2.4, 12.0, 3.5, 12.0, 0.0, -1.0, Some(0)),
            op("Office side door", "O", 6.0, 8.4, 6.0, 9.6, -1.0, 0.0, None), op("Back corridor door", "Bk", 12.0, 6.0, 13.4, 6.0, 0.0, -1.0, None),
            op("Back corridor door (office)", "Bk", 1.0, 6.0, 2.2, 6.0, 0.0, -1.0, None), op("Cold room door", "C", 4.0, 4.0, 5.1, 4.0, 0.0, -1.0, Some(1)),
            op("Parts room door", "P", 11.4, 4.0, 12.6, 4.0, 0.0, -1.0, None), op("Boiler room door", "R", 17.4, 4.0, 18.5, 4.0, 0.0, -1.0, Some(2)),
        ],
        lights: &[lt(3.0, 15.0, WARM, 0.95, false), lt(9.5, 9.0, NEUTRAL, 1.05, false), lt(9.5, 15.0, NEUTRAL, 1.05, false), lt(14.0, 12.0, NEUTRAL, 1.05, true), lt(18.0, 9.0, NEUTRAL, 1.05, false),
            lt(18.0, 15.0, NEUTRAL, 1.05, false), lt(3.0, 9.0, COOL, 0.9, false), lt(5.0, 5.0, COOL, 0.75, false), lt(13.0, 5.0, COOL, 0.75, false), lt(4.5, 2.0, COOL, 0.9, false),
            lt(12.0, 2.0, WARM, 0.9, false), lt(18.0, 2.0, WARM, 0.9, false)],
        targets: &[
            tg("T1", Threat, 1, 20.3, 17.3, 14.0, 12.0, "W", false), tg("T2", Threat, 0, 9.5, 10.2, 9.5, 14.0, "W", false), tg("T3", Threat, 1, 15.5, 13.3, 15.0, 17.0, "W", false),
            tg("T4", Threat, 0, 17.0, 10.2, 17.0, 14.0, "W", false), tg("N1", Civil, 0, 13.0, 10.3, 13.0, 14.0, "W", false),
            tg("T5", Threat, 1, 0.7, 11.3, 3.0, 9.0, "O", false), tg("N2", Civil, 1, 5.3, 6.7, 3.0, 10.0, "O", false),
            tg("T6", Threat, 0, 20.3, 5.0, 10.0, 5.0, "Bk", false), tg("T7", Threat, 1, 0.7, 3.3, 4.5, 3.0, "C", false),
            tg("T8", Threat, 0, 14.4, 3.3, 12.0, 3.5, "P", false), tg("N3", Civil, 0, 11.0, 0.8, 12.0, 3.6, "P", false), tg("T9", Threat, 1, 20.4, 3.3, 18.0, 3.5, "R", false),
            // o mech patrulha o corredor entre as fileiras de estantes (fora de cena nos outros modos)
            tg("M1", Threat, 0, 12.8, 10.7, 7.0, 10.7, "W", false),
        ],
        slots: &[
            ("W", 20.3, 17.3, 14.0, 12.0), ("W", 20.3, 6.8, 14.0, 12.0), ("W", 7.0, 7.0, 12.0, 12.0), ("W", 13.0, 10.3, 13.0, 14.0), ("W", 9.5, 10.2, 9.5, 14.0), ("W", 17.0, 10.2, 17.0, 14.0),
            ("W", 9.5, 13.3, 9.5, 17.0), ("W", 15.5, 13.3, 15.0, 17.0), ("W", 11.0, 16.4, 8.0, 16.0), ("W", 16.5, 16.5, 12.0, 16.6), ("W", 19.0, 13.5, 12.0, 13.0),
            ("O", 0.7, 11.3, 3.0, 9.0), ("O", 5.3, 6.7, 3.0, 10.0), ("O", 5.3, 11.3, 2.5, 9.0), ("O", 2.5, 7.0, 3.0, 11.0),
            ("Bk", 20.3, 5.0, 10.0, 5.0), ("Bk", 0.7, 5.0, 10.0, 5.0), ("Bk", 8.0, 4.6, 12.7, 5.6),
            ("C", 0.7, 3.3, 4.5, 3.0), ("C", 8.3, 3.3, 4.5, 3.5), ("C", 8.3, 2.0, 5.0, 3.0), ("C", 4.5, 0.8, 4.5, 3.6),
            ("P", 9.6, 3.3, 12.0, 3.6), ("P", 14.4, 3.3, 12.0, 3.5), ("P", 14.4, 0.6, 12.0, 3.5), ("P", 11.0, 0.8, 12.0, 3.6),
            ("R", 15.6, 3.3, 18.0, 3.6), ("R", 20.4, 3.3, 18.0, 3.5), ("R", 17.5, 0.8, 18.0, 3.6), ("R", 15.7, 1.8, 18.0, 3.5),
        ],
        walls_h: &[(0.0, 0.0, 21.0, &[]), (18.0, 0.0, 21.0, &[]), (12.0, 0.0, 6.0, &[(2.4, 3.5)]), (6.0, 0.0, 21.0, &[(1.0, 2.2), (12.0, 13.4)]), (4.0, 0.0, 21.0, &[(4.0, 5.1), (11.4, 12.6), (17.4, 18.5)])],
        walls_v: &[(0.0, 0.0, 18.0, &[]), (21.0, 0.0, 18.0, &[]), (6.0, 6.0, 18.0, &[(8.4, 9.6), (14.4, 15.8)]), (9.0, 0.0, 4.0, &[]), (15.0, 0.0, 4.0, &[])],
        props: &[
            pr(Shelf, 8.5, 9.0, 10.48, 9.45, 1.93), pr(Shelf, 12.0, 9.0, 13.98, 9.45, 1.93), pr(Shelf, 16.0, 9.0, 17.98, 9.45, 1.93), pr(Shelf, 8.5, 12.0, 10.48, 12.45, 1.93),
            pr(Shelf, 14.5, 12.0, 16.48, 12.45, 1.93), pr(Shelf, 10.0, 15.0, 11.98, 15.45, 1.93), pr(Shelf, 15.5, 15.0, 17.48, 15.45, 1.93),
            pr(Cover, 12.2, 13.4, 13.4, 13.95, 1.25), pr(Crate, 19.6, 11.0, 20.12, 12.24, 0.46), pr(Barrel, 7.0, 16.8, 7.52, 17.32, 0.88), pr(Barrel, 19.0, 6.4, 19.52, 6.92, 0.88), pr(Crate, 18.5, 16.8, 19.74, 17.32, 0.46),
            pr(Table, 1.5, 8.5, 3.5, 9.45, 0.79), pr(Locker, 0.2, 6.4, 0.87, 8.42, 2.24), pr(Shelf, 1.0, 0.3, 2.98, 0.75, 1.93), pr(Barrel, 7.8, 0.5, 8.32, 1.02, 0.88),
            pr(Crate, 9.4, 0.4, 9.92, 1.64, 0.46), pr(Table, 12.5, 1.0, 14.5, 1.95, 0.79), pr(Locker, 20.1, 0.4, 20.77, 2.42, 2.24), pr(Barrel, 15.5, 0.5, 16.02, 1.02, 0.88),
            pr(Cover, 2.4, 14.0, 3.6, 14.55, 1.25), pr(Bench, 0.2, 15.0, 1.12, 17.52, 0.77),
        ],
        doors: &[("D1", 2.4, 12.0), ("D2", 4.0, 4.0), ("D3", 17.4, 4.0)],
        corners: &[],
        start: (3.0, 16.8, 0.0), pick: true, defense: false, portals: &[], ramps: &[], pit: None, darks: &[],
    },
    /* ---------- 4 · corredor: portas frente a frente dos dois lados e um cruzamento em X ---------- */
    MapDef {
        name: "Hallway", blurb: "one long hallway, doors facing each other on both sides and a four-way crossing",
        zones: &[
            zn("E", "Entry", 8.0, 14.0, 12.0, 18.0), zn("K", "Hallway", 9.0, 2.0, 11.0, 14.0), zn("L1", "Room 1", 3.0, 9.0, 9.0, 14.0), zn("R1", "Room 2", 11.0, 9.0, 17.0, 14.0),
            zn("X1", "West passage", 3.0, 7.0, 9.0, 9.0), zn("X2", "East passage", 11.0, 7.0, 17.0, 9.0), zn("L2", "Room 3", 3.0, 2.0, 9.0, 7.0), zn("R2", "Room 4", 11.0, 2.0, 17.0, 7.0),
        ],
        doorways: &[dh("K", 9.1, 10.9, 14.0, -1.0, 0), dv("L1", 9.0, 11.0, 12.2, -1.0, 1), dv("R1", 11.0, 11.0, 12.2, 1.0, 2), dv("X1", 9.0, 7.1, 8.9, -1.0, 3), dv("X2", 11.0, 7.1, 8.9, 1.0, 4),
            dv("L2", 9.0, 4.0, 5.2, -1.0, 5), dv("R2", 11.0, 4.0, 5.2, 1.0, 6)],
        openings: &[
            op("Hallway mouth", "K", 9.1, 14.0, 10.9, 14.0, 0.0, -1.0, None), op("Room 1 door", "L1", 9.0, 11.0, 9.0, 12.2, -1.0, 0.0, None), op("Room 2 door", "R1", 11.0, 11.0, 11.0, 12.2, 1.0, 0.0, None),
            op("West passage", "X1", 9.0, 7.1, 9.0, 8.9, -1.0, 0.0, None), op("East passage", "X2", 11.0, 7.1, 11.0, 8.9, 1.0, 0.0, None),
            op("Room 3 door", "L2", 9.0, 4.0, 9.0, 5.2, -1.0, 0.0, None), op("Room 4 door", "R2", 11.0, 4.0, 11.0, 5.2, 1.0, 0.0, None),
        ],
        lights: &[lt(10.0, 16.0, WARM, 0.9, false), lt(10.0, 12.0, COOL, 0.75, true), lt(10.0, 8.0, COOL, 0.75, true), lt(10.0, 4.0, COOL, 0.75, true), lt(6.0, 11.5, NEUTRAL, 1.0, false),
            lt(14.0, 11.5, NEUTRAL, 1.0, false), lt(6.0, 4.5, WARM, 0.95, false), lt(14.0, 4.5, NEUTRAL, 1.0, false), lt(5.0, 8.0, COOL, 0.7, false), lt(15.0, 8.0, COOL, 0.7, false)],
        targets: &[
            tg("T1", Threat, 1, 10.0, 2.7, 10.0, 13.0, "K", false),
            tg("T2", Threat, 0, 3.6, 13.3, 8.5, 11.6, "L1", false), tg("T3", Threat, 1, 8.3, 9.6, 5.0, 12.0, "L1", false), tg("N1", Civil, 0, 5.0, 11.0, 8.5, 11.6, "L1", false),
            tg("T4", Threat, 0, 16.4, 13.3, 11.5, 11.6, "R1", false), tg("N2", Civil, 1, 14.0, 10.5, 11.5, 11.6, "R1", false),
            tg("T5", Threat, 1, 3.6, 8.0, 9.0, 8.0, "X1", false), tg("T6", Threat, 0, 16.4, 8.0, 11.0, 8.0, "X2", false),
            tg("T7", Threat, 1, 3.6, 2.6, 8.5, 4.6, "L2", false), tg("T8", Threat, 0, 8.3, 6.4, 5.0, 3.5, "L2", false),
            tg("T9", Threat, 1, 16.4, 2.6, 11.5, 4.6, "R2", false), tg("N3", Civil, 0, 13.5, 5.5, 11.5, 4.6, "R2", false),
        ],
        slots: &[
            ("K", 10.0, 2.7, 10.0, 13.0), ("K", 9.5, 3.4, 10.0, 13.0), ("K", 10.5, 9.8, 10.0, 13.5),
            ("L1", 3.6, 13.3, 8.5, 11.6), ("L1", 8.3, 9.6, 5.0, 12.0), ("L1", 3.6, 9.6, 8.0, 11.6), ("L1", 8.3, 13.4, 5.0, 11.0), ("L1", 5.0, 11.0, 8.5, 11.6), ("L1", 6.8, 13.3, 8.5, 11.6),
            ("R1", 16.4, 13.3, 11.5, 11.6), ("R1", 11.7, 9.6, 14.0, 12.0), ("R1", 16.4, 9.6, 11.5, 11.6), ("R1", 11.7, 13.4, 15.0, 11.0), ("R1", 14.0, 10.5, 11.5, 11.6),
            ("X1", 3.6, 8.0, 9.0, 8.0), ("X1", 5.2, 7.5, 9.0, 8.0), ("X1", 4.4, 8.5, 9.0, 8.0),
            ("X2", 16.4, 8.0, 11.0, 8.0), ("X2", 14.8, 8.5, 11.0, 8.0), ("X2", 15.6, 7.5, 11.0, 8.0),
            ("L2", 3.6, 2.6, 8.5, 4.6), ("L2", 8.3, 6.4, 5.0, 3.5), ("L2", 3.6, 6.4, 8.5, 4.6), ("L2", 8.3, 2.6, 5.0, 5.0), ("L2", 6.0, 2.6, 8.5, 4.6),
            ("R2", 16.4, 2.6, 11.5, 4.6), ("R2", 11.7, 6.4, 15.0, 3.5), ("R2", 16.4, 6.4, 11.5, 4.6), ("R2", 11.7, 2.6, 15.0, 5.0), ("R2", 13.5, 5.5, 11.5, 4.6),
        ],
        walls_h: &[(18.0, 8.0, 12.0, &[]), (14.0, 3.0, 17.0, &[(9.1, 10.9)]), (9.0, 3.0, 9.0, &[]), (9.0, 11.0, 17.0, &[]), (7.0, 3.0, 9.0, &[]), (7.0, 11.0, 17.0, &[]), (2.0, 3.0, 17.0, &[])],
        walls_v: &[(8.0, 14.0, 18.0, &[]), (12.0, 14.0, 18.0, &[]), (3.0, 2.0, 14.0, &[]), (17.0, 2.0, 14.0, &[]),
            (9.0, 2.0, 14.0, &[(4.0, 5.2), (7.1, 8.9), (11.0, 12.2)]), (11.0, 2.0, 14.0, &[(4.0, 5.2), (7.1, 8.9), (11.0, 12.2)])],
        props: &[
            pr(Cover, 5.4, 11.7, 6.6, 12.25, 1.25), pr(Sofa, 12.6, 13.1, 14.4, 13.86, 0.9), pr(Crate, 6.3, 8.38, 7.54, 8.9, 0.46), pr(Table, 5.0, 4.2, 7.0, 5.15, 0.79),
            pr(Locker, 16.2, 3.5, 16.87, 5.52, 2.24), pr(Barrel, 13.0, 7.2, 13.52, 7.72, 0.88), pr(Crate, 8.3, 16.6, 8.82, 17.84, 0.46),
        ],
        doors: &[],
        corners: &[],
        start: (8.6, 15.2, 0.0), pick: true, defense: false, portals: &[], ramps: &[], pit: None, darks: &[],
    },
    /* ---------- 5 · casa de dois andares (defesa): andar de cima em z 0..6, terraço em z 6..7, térreo em z 7..13, quintal em
       z 13..17,2 e rua em z 17,2..18,4. Os invasores nascem na rua, atrás do muro, e entram pelos portões. A escada liga os dois
       andares (degraus de verdade nas duas pontas e uma passagem no meio); parte dos que entram pela garagem sobe pela laje até o terraço e entra pelas portas-balcão ---------- */
    MapDef {
        name: "Family home", blurb: "single floor: bedrooms at the back, a corridor, living area, yard and street",
        zones: &[
            zn("Hl", "Bedroom hall", 0.0, 0.0, 9.3, 1.4), zn("S1", "Suite 1", 0.0, 1.4, 4.7, 6.0), zn("S2", "Suite 2", 4.7, 1.4, 9.3, 6.0), zn("Ms", "Master suite", 9.3, 0.0, 14.0, 6.0),
            // planta térrea: um corredor largo liga os quartos (ao fundo) à sala; o braço leste dele dá na suíte principal e na área gourmet
            zn("Cr", "Corridor", 0.0, 6.0, 10.0, 8.0), zn("Ce", "Side corridor", 10.0, 6.0, 14.0, 7.0),
            zn("Lv", "Living room", 0.0, 8.0, 10.0, 13.0), zn("Gm", "Gourmet area", 10.0, 7.0, 14.0, 13.0), zn("Yd", "Yard", 0.0, 13.0, 14.0, 17.2),
            zn("Rd", "Street", 0.0, 17.2, 14.0, 18.4),
        ],
        doorways: &[dh("S1", 3.6, 4.5, 1.4, 1.0, 0), dh("S2", 6.6, 7.5, 1.4, 1.0, 1), dv("Ms", 9.3, 0.2, 1.2, 1.0, 2), dh("Lv", 3.3, 4.4, 13.0, -1.0, 3), dh("Gm", 10.6, 13.4, 13.0, -1.0, 4),
            dh("Yd", 2.6, 5.4, 17.2, -1.0, 5)],
        openings: &[
            op("Suite 1 door", "S1", 3.6, 1.4, 4.5, 1.4, 0.0, 1.0, None), op("Suite 2 door", "S2", 6.6, 1.4, 7.5, 1.4, 0.0, 1.0, None), op("Master suite door", "Ms", 9.3, 0.2, 9.3, 1.2, 1.0, 0.0, None),
            op("Front door", "Lv", 3.3, 13.0, 4.4, 13.0, 0.0, -1.0, None), op("Gourmet opening", "Gm", 10.6, 13.0, 13.4, 13.0, 0.0, -1.0, None), op("Garage gate", "Yd", 2.6, 17.2, 5.4, 17.2, 0.0, -1.0, None),
        ],
        lights: &[lt(1.9, 0.7, WARM, 0.9, false), lt(4.3, 0.7, WARM, 0.7, false), lt(7.6, 0.7, WARM, 0.7, false), lt(2.6, 3.7, WARM, 0.85, false), lt(6.2, 3.7, WARM, 0.85, false), lt(11.0, 4.1, WARM, 0.9, false), lt(11.6, 1.1, WARM, 0.6, false),
            lt(2.6, 7.0, NEUTRAL, 0.8, false), lt(7.4, 7.0, NEUTRAL, 0.8, false), lt(12.0, 6.5, NEUTRAL, 0.5, false),
            lt(3.0, 10.5, NEUTRAL, 0.95, false), lt(7.4, 10.5, NEUTRAL, 0.95, false), lt(12.0, 10.0, NEUTRAL, 0.9, false), lt(3.6, 15.1, COOL, 0.6, false), lt(10.6, 15.1, COOL, 0.6, false)],
        // dez invasores de reserva: esperam na rua, fora da vista de quem está na casa, até a onda chamá-los
        targets: &[
            tg("I1", Threat, 0, 0.5, 17.8, 4.0, 17.2, "Rd", false), tg("I2", Threat, 1, 10.4, 17.8, 8.9, 17.2, "Rd", false), tg("I3", Threat, 0, 1.2, 17.8, 4.0, 17.2, "Rd", false),
            tg("I4", Threat, 1, 11.3, 17.8, 8.9, 17.2, "Rd", false), tg("I5", Threat, 0, 1.9, 17.8, 4.0, 17.2, "Rd", false), tg("I6", Threat, 1, 12.2, 17.8, 8.9, 17.2, "Rd", false),
            tg("I7", Threat, 0, 6.1, 17.8, 4.0, 17.2, "Rd", false), tg("I8", Threat, 1, 13.1, 17.8, 8.9, 17.2, "Rd", false), tg("I9", Threat, 0, 6.8, 17.8, 4.0, 17.2, "Rd", false),
            tg("I10", Threat, 1, 7.5, 17.8, 8.9, 17.2, "Rd", false),
        ],
        slots: &[
            ("Rd", 0.5, 17.8, 4.0, 17.2), ("Rd", 10.4, 17.8, 8.9, 17.2), ("Rd", 1.2, 17.8, 4.0, 17.2), ("Rd", 11.3, 17.8, 8.9, 17.2), ("Rd", 1.9, 17.8, 4.0, 17.2),
            ("Rd", 12.2, 17.8, 8.9, 17.2), ("Rd", 6.1, 17.8, 4.0, 17.2), ("Rd", 13.1, 17.8, 8.9, 17.2), ("Rd", 6.8, 17.8, 4.0, 17.2), ("Rd", 7.5, 17.8, 8.9, 17.2),
        ],
        walls_h: &[(0.0, 0.0, 14.0, &[]), (6.0, 0.0, 14.0, &[(1.5, 2.4), (3.0, 4.2), (5.0, 6.2), (6.8, 7.6), (10.4, 11.3)]), (1.4, 0.0, 9.3, &[(1.6, 2.8), (3.6, 4.5), (5.0, 6.2), (6.6, 7.5)]), (2.2, 10.7, 14.0, &[]), (3.7, 7.7, 9.3, &[]), (3.6, 0.0, 1.3, &[]),
            (7.0, 10.0, 14.0, &[(11.6, 12.5)]), (8.0, 0.0, 10.0, &[(2.0, 3.1), (7.0, 8.1)]), (13.0, 0.0, 14.0, &[(3.3, 4.4), (10.6, 13.4)]), (17.2, 0.0, 14.0, &[(2.6, 5.4), (8.3, 9.5)]), (18.4, 0.0, 14.0, &[])],
        walls_v: &[(0.0, 0.0, 18.4, &[]), (14.0, 0.0, 18.4, &[]), (4.7, 1.4, 6.0, &[(3.0, 4.2)]), (7.7, 1.4, 6.0, &[(2.2, 3.1), (4.4, 5.3)]), (9.3, 0.0, 6.0, &[(0.2, 1.2), (4.3, 5.5)]),
            (10.7, 0.0, 2.2, &[(0.5, 1.5)]), (12.7, 2.2, 6.0, &[(2.9, 3.8)]), (1.3, 3.6, 6.0, &[(4.4, 5.3)]),
            (10.0, 7.0, 13.0, &[(8.6, 9.7)]), (7.5, 13.0, 17.2, &[]),
            // a rua é dividida em dois lados: quem chega pelo lado da piscina só tem o portão lateral, e a casa é atacada por duas frentes
            (7.95, 17.2, 18.4, &[])],
        props: &[
            // kill house vazia: sem camas nem móveis. Peitoris das janelas dos quartos (para os dois corredores e de um
            // quarto para o outro): por elas os invasores veem, atiram e jogam granada, e ninguém passa
            pr(Sill, 1.6, 1.3, 2.8, 1.5, 0.95), pr(Sill, 5.0, 1.3, 6.2, 1.5, 0.95), pr(Sill, 3.0, 5.9, 4.2, 6.1, 0.95), pr(Sill, 5.0, 5.9, 6.2, 6.1, 0.95),
            pr(Sill, 9.2, 4.3, 9.4, 5.5, 0.95), pr(Sill, 4.6, 3.0, 4.8, 4.2, 0.95),
            pr(Pool, 10.7, 14.0, 13.3, 16.4, 0.12),
        ],
        doors: &[],
        corners: &[],
        start: (12.1, 5.0, 1.5708), pick: false, defense: true,
        // planta térrea: sem escada, sem passagem entre andares, sem vãos escuros
        portals: &[], ramps: &[], pit: None, darks: &[],
    },
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
