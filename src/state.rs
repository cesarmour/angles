//! Estado compartilhado do jogo.
use crate::map::*;
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { Menu, Card, Play, Pause, Debrief, Replay, Credits }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View { Fp, Bird }

#[derive(Default, Clone, Debug)]
pub struct Stats { pub shots: u32, pub hits: u32, pub civ_hit: u32, pub hits_taken: u32, pub failed: u32, pub threats_down: u32, pub zone_a: u32, pub zone_c: u32, pub zone_d: u32 }

#[derive(Resource, Clone)]
pub struct Game {
    pub mode: Mode, pub time: f32, pub view: View, pub tac: bool, pub flash: bool, pub free: bool, pub mission2: bool, pub seed: u32,
    pub lesson: usize, pub obj: Vec<u8>, pub lesson_end: f32, pub debriefed: bool,
    pub flag_tac: bool, pub flag_bird: bool, pub flag_bird_back: bool,
    pub entry: [Option<f32>; 4], pub entry_pie: [Option<f32>; 4], pub dwell: [f32; 4], pub door_side: [Option<bool>; 2],
    pub stats: Stats, pub bird_zoom: f32, pub sens: f32,
    pub toast: Option<(String, u8, f32)>, pub hit_t: f32, pub dmg_t: f32,
    pub ui_dirty: bool, pub tracers: Vec<(Vec3, Vec3, f32)>, pub shake: f32, pub near_door: Option<usize>,
    /// Amostras do trajeto da lição atual (tempo, posição, direção, ângulo oculto) para o replay.
    pub track: Vec<(f32, Vec2, f32, f32)>, pub replay_t: f32, pub replay_next: Option<usize>,
    /// Demonstração do traçado ideal (o operador anda sozinho) e linha-guia no chão.
    pub demo: bool, pub demo_req: bool, pub demo_end: bool, pub demo_idle: f32, pub guide_i: usize,
    /// Traçado ideal no chão: 1 = sempre, 2 = dica (aparece por alguns segundos e some), 0 = desligado. `guide_t` = tempo restante da dica.
    pub guide: u8, pub guide_t: f32,
    /// Caixa da lição (canto superior esquerdo) oculta pelo jogador.
    pub hide_box: bool,
    /// Pontuação: cada ameaça derrubada, distância média ao traçado (total e da lição atual), chances de sucesso ao vivo e badges ganhos.
    pub kills: Vec<Kill>, pub route_sum: f32, pub route_t: f32, pub lroute_sum: f32, pub lroute_t: f32, pub odds: f32, pub odds_route: f32, pub odds_angle: f32, pub badges: u32,
    /// Tempo sob a linha de tiro de uma ameaça viva: sequência atual e total do percurso.
    pub expo_now: f32, pub expo_total: f32,
    /// Aviso de técnica em exibição (texto, tipo 1 = acerto / 2 = correção, tempo restante), intervalo mínimo e contagem por tipo.
    pub coach: Option<(String, u8, f32)>, pub coach_cd: f32, pub warns: [u32; 6], pub flag_flash: bool,
}
impl Default for Game {
    fn default() -> Self {
        Game { mode: Mode::Menu, time: 0.0, view: View::Fp, tac: false, flash: true, free: false, mission2: false, seed: 1, lesson: 0, obj: Vec::new(), lesson_end: 0.0, debriefed: false,
            flag_tac: false, flag_bird: false, flag_bird_back: false, entry: [None; 4], entry_pie: [None; 4], dwell: [0.0; 4], door_side: [None; 2], stats: Stats::default(),
            bird_zoom: 13.0, sens: 1.0, toast: None, hit_t: 0.0, dmg_t: 0.0, ui_dirty: true, tracers: Vec::new(), shake: 0.0, near_door: None, track: Vec::new(), replay_t: 0.0, replay_next: None,
            demo: false, demo_req: false, demo_end: false, demo_idle: 0.0, guide_i: 0, guide: 1, guide_t: 0.0, hide_box: false,
            kills: Vec::new(), route_sum: 0.0, route_t: 0.0, lroute_sum: 0.0, lroute_t: 0.0, odds: 100.0, odds_route: 100.0, odds_angle: 100.0, badges: 0, expo_now: 0.0, expo_total: 0.0, coach: None, coach_cd: 0.0, warns: [0; 6], flag_flash: false }
    }
}
/// Uma ameaça derrubada: ângulo da fatia (graus entre o alvo e a borda mais próxima do que o encobria), largura do corpo
/// que o alvo via (cm), tempo desde o primeiro contato e pontos (0 a 100).
#[derive(Clone, Copy, Debug)]
pub struct Kill { pub angle: f32, pub expo_cm: f32, pub dt: f32, pub pts: f32, pub a_zone: bool }
pub const TIP_SECS: f32 = 8.0;

impl Game {
    /// O traçado está sendo desenhado agora?
    pub fn route_visible(&self) -> bool { self.demo || self.guide == 1 || (self.guide == 2 && self.guide_t > 0.0) }
    pub fn next_guide(&mut self) { self.guide = match self.guide { 1 => 2, 2 => 0, _ => 1 }; if self.guide == 2 { self.guide_t = TIP_SECS; } self.ui_dirty = true; }
    /// Nota do traçado (0 a 100): 100 até 20 cm de distância média, zero a 1 m ou mais.
    pub fn route_score(&self) -> Option<f32> { if self.route_t < 2.0 { None } else { Some((1.0 - (self.route_sum / self.route_t - 0.2) / 0.8).clamp(0.0, 1.0) * 100.0) } }
    /// Nota média das derrubadas (ângulo + exposição).
    pub fn kill_score(&self) -> Option<f32> { if self.kills.is_empty() { None } else { Some(self.kills.iter().map(|k| k.pts).sum::<f32>() / self.kills.len() as f32) } }
    /// Nota-base do percurso, antes das penalidades: 40% traçado, 60% ângulo e exposição nas derrubadas.
    pub fn base_score(&self) -> f32 { match (self.route_score(), self.kill_score()) { (Some(r), Some(k)) => 0.4 * r + 0.6 * k, (None, Some(k)) => k, (Some(r), None) => r, _ => 100.0 } }
    pub fn toast(&mut self, msg: &str, kind: u8, ttl: f32) { self.toast = Some((msg.to_string(), kind, ttl)); self.ui_dirty = true; }
}

#[derive(Resource, Clone)]
pub struct Player {
    pub pos: Vec2, pub vel: Vec2, pub yaw: f32, pub pitch: f32, pub lean: f32, pub crouch: f32, pub crouching: bool,
    pub ads: bool, pub fast: bool, pub speed: f32, pub step_acc: f32, pub bob: f32, pub eye: Vec3,
    pub mag: u32, pub reserve: u32, pub reloading: bool, pub reload_t: f32, pub fire_cool: f32, pub zone: usize,
    pub look_acc: f32, pub move_acc: f32, pub ads_acc: f32, pub reloads: u32, pub shot_n: u32, pub lean_acc: f32, pub crouch_acc: f32,
    pub g_ads: f32, pub g_kick: f32, pub g_block: f32, pub g_lower: f32, pub g_sway: Vec2, pub recoil: Vec2, pub last_look: Vec2,
    /// O jogador pediu para inclinar, mas a parede ao lado não deixa.
    pub lean_blocked: bool,
}
impl Default for Player {
    fn default() -> Self {
        Player { pos: Vec2::new(3.0, 15.4), vel: Vec2::ZERO, yaw: 0.0, pitch: 0.0, lean: 0.0, crouch: 0.0, crouching: false, ads: false, fast: false, speed: 0.0,
            step_acc: 0.0, bob: 0.0, eye: Vec3::new(3.0, EYE_STAND, 15.4), mag: 8, reserve: 64, reloading: false, reload_t: 0.0, fire_cool: 0.0, zone: 0,
            look_acc: 0.0, move_acc: 0.0, ads_acc: 0.0, reloads: 0, shot_n: 0, lean_acc: 0.0, crouch_acc: 0.0,
            g_ads: 0.0, g_kick: 0.0, g_block: 0.0, g_lower: 0.0, g_sway: Vec2::ZERO, recoil: Vec2::ZERO, last_look: Vec2::ZERO, lean_blocked: false }
    }
}
impl Player {
    pub fn fwd(&self) -> Vec2 { Vec2::new(-self.yaw.sin(), -self.yaw.cos()) }
    pub fn right(&self) -> Vec2 { Vec2::new(self.yaw.cos(), -self.yaw.sin()) }
    pub fn eye2(&self) -> Vec2 { Vec2::new(self.eye.x, self.eye.z) }
    /// Altura do topo da cabeça, usada para saber se um móvel baixo encobre o operador.
    pub fn head_h(&self) -> f32 { self.eye.y + 0.1 }
}

/// Comandos do quadro: vêm do teclado e mouse ou do roteiro automático.
#[derive(Resource, Default, Clone)]
pub struct Controls {
    pub mv: Vec2, pub look: Vec2, pub fire: bool, pub ads: bool, pub lean: f32, pub crouch_toggle: bool, pub crouch_hold: bool, pub fast: bool,
    pub reload: bool, pub interact: bool, pub tac: bool, pub view: bool, pub flash: bool, pub help: bool, pub confirm: bool, pub pause: bool, pub zoom: f32,
    pub world_move: Option<Vec2>,
    /// Liga/desliga a linha do traçado ideal; `pace` multiplica a velocidade (0 = normal), usado na demonstração.
    pub guide: bool, pub pace: f32,
    /// Mostra/oculta a caixa da lição.
    pub hide_box: bool,
}

pub struct TargetState {
    pub def: TargetDef, pub alive: bool, pub fall: f32, pub hits: u8, pub seen: bool, pub vis: bool, pub react: f32,
    pub contact: bool, pub contact_t: f32, pub first_facing: Option<bool>, pub fwd: Vec2, pub fall_dir: f32, pub fall_max: f32,
    pub pivot: Entity, pub board: Entity, pub mask: usize, pub marks: Vec<Entity>, pub fall_v: f32, pub sw: f32, pub swv: f32, pub landed: bool, pub root: Entity, pub off: bool,
}
#[derive(Resource, Default)]
pub struct Targets { pub list: Vec<TargetState>, pub masks: Vec<Vec<u8>> }
impl Targets {
    pub fn get(&self, id: &str) -> &TargetState { self.list.iter().find(|t| t.def.id == id).expect("alvo") }
    pub fn down(&self, ids: &[&str]) -> bool { ids.iter().all(|id| !self.get(id).alive) }
}

/// Eventos sonoros disparados pela simulação.
#[derive(Event, Clone, Copy)]
pub enum Sfx { Shot, EnemyShot(Vec3), Dry, Click, Reload, Step(bool), Impact(Vec3), Metal(Vec3), Paper(Vec3), Thwack(Vec3), Fall(Vec3), Door(Vec3), Casing(Vec3), Good, Bad, Hurt, Ui, Rustle }
