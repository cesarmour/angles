//! Estado compartilhado do jogo.
use crate::map::*;
use bevy::prelude::*;

/// Pedido de troca de mapa (índice em `map::MAPS`), atendido por `world::map_switch`.
#[derive(Resource, Default)]
pub struct MapReq(pub Option<usize>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { Menu, Card, Play, Pause, Debrief, Replay, Credits, Down, Scenarios, Ranking }
impl Default for Mode { fn default() -> Self { Mode::Menu } }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View { Fp, Bird }

#[derive(Default, Clone, Debug)]
pub struct Stats { pub shots: u32, pub hits: u32, pub civ_hit: u32, pub hits_taken: u32, pub failed: u32, pub threats_down: u32, pub zone_a: u32, pub zone_c: u32, pub zone_d: u32 }

#[derive(Resource, Clone)]
pub struct Game {
    pub mode: Mode, pub time: f32, pub view: View, pub tac: bool, pub flash: bool, pub free: bool, pub mission2: bool, pub seed: u32,
    pub lesson: usize, pub obj: Vec<u8>, pub lesson_end: f32, pub debriefed: bool,
    pub flag_tac: bool, pub flag_bird: bool, pub flag_bird_back: bool,
    pub entry: [Option<f32>; MAX_DOORWAYS], pub entry_pie: [Option<f32>; MAX_DOORWAYS], pub dwell: [f32; MAX_DOORWAYS], pub door_side: [Option<bool>; MAX_DOORS],
    pub stats: Stats, pub bird_zoom: f32, pub sens: f32,
    pub toast: Option<(String, u8, f32)>, pub hit_t: f32, pub dmg_t: f32,
    pub ui_dirty: bool, pub tracers: Vec<(Vec3, Vec3, f32)>, pub burst: Vec<(f32, Vec3, Vec3)>, pub shake: f32, pub near_door: Option<usize>,
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
    /// Cômodos já limpos (um bit por zona), quantos cômodos contam, e o carimbo "ROOM CLEAR" em exibição (texto, tempo restante).
    pub cleared: u32, pub rooms: u32, pub stamp: String, pub stamp_t: f32,
    /// Tiros levados desde o último ponto de retomada (dois encerram a partida), tempo que resta da queda do operador antes
    /// da tela "OPERATOR DOWN", o motivo mostrado nela e o pedido de guardar o ponto de retomada da lição que começa.
    pub wounds: u8, pub down_t: f32, pub down_why: String, pub ckpt_req: bool,
    /// Cenário em curso (índice em `scenario::SCENARIOS`), relógio dele (contagem regressiva ou espera entre ondas), ajuste de
    /// pontos (positivo ou negativo), dois contadores de uso livre do cenário, título da tela de fim quando não é "OPERATOR DOWN"
    /// e se o cenário foi vencido.
    pub scn: Option<usize>, pub scn_t: f32, pub scn_adj: i32, pub scn_a: u32, pub scn_b: u32, pub down_title: String, pub won: bool,
    /// Aberturas não fatiadas pelas quais o operador passou direto; ameaças derrubadas de fora do cômodo delas.
    pub passes: u32, pub outside_kills: u32,
    /// Portas trancadas (uma por porta com folha) e arrombamento em andamento (porta, progresso de 0 a 1).
    pub locked: [bool; MAX_DOORS], pub breach: Option<(usize, f32)>,
    /// Defesa da casa: onda atual (0 = ainda não começou), invasores derrubados no total; escurecimento da tela e pausa ao
    /// trocar de andar pela escada.
    pub wave: u32, pub wave_kills: u32, pub fade_t: f32, pub portal_cd: f32,
    /// Colocação desta partida no ranking (posição, total de partidas na tabela), preenchida ao terminar.
    pub rank: Option<(usize, usize)>, pub ranked: bool,
    /// Quanto falta para o canhão do mech disparar (0 = sem mira, 1 = disparo): vira o aviso "CANNON LOCK" na tela.
    pub lock: f32,
    /// Granada em cena: onde ela está no chão e quantos segundos faltam para estourar (aviso na tela).
    pub nade: Option<(Vec2, f32)>,
    /// Onde caíram, neste quadro, os invasores derrubados (o cenário conta para a porta mais próxima) e granadas pedidas
    /// por quem vai entrar por uma porta que já custou caro (de onde, para onde).
    pub fell: Vec<Vec2>, pub nade_req: Vec<(Vec3, Vec3)>,
    /// Legenda da fala do instrutor que está tocando (texto e segundos que restam).
    pub sub: Option<(String, f32)>,
    /// Modo em terceira pessoa (câmera por cima do ombro, com o corpo do operador à vista).
    pub third: bool,
}
/// Dificuldade escolhida no menu: 0 = EASY (alvos IPSC de papelão, sem bonecos; o alvo não vira nem anda, o tiro levado é só
/// simulado e não encerra a partida), 1 = NORMAL, 2 = HARD (ameaças atiram 50% mais rápido e acertam 95% dos disparos).
static DIFF: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);
pub fn difficulty() -> u8 { DIFF.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_difficulty(d: u8) { DIFF.store(d.min(2), std::sync::atomic::Ordering::Relaxed); }
/// Cenários com alvos que andam ou se rendem pedem bonecos mesmo no nível EASY; `figs_built` diz como os alvos em cena foram montados.
static FORCE_FIGS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static FIGS_BUILT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
pub fn force_figs() -> bool { FORCE_FIGS.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_force_figs(on: bool) { FORCE_FIGS.store(on, std::sync::atomic::Ordering::Relaxed); }
/// Os alvos devem ser bonecos (e não papelão) agora?
pub fn want_figs() -> bool { difficulty() != 0 || force_figs() }
pub fn figs_built() -> bool { FIGS_BUILT.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_figs_built(on: bool) { FIGS_BUILT.store(on, std::sync::atomic::Ordering::Relaxed); }
pub const DIFF_NAMES: [&str; 3] = ["EASY", "NORMAL", "HARD"];
/// Nome do nível gravado numa partida do ranking (3 = fase "very hard", que tem dificuldade própria).
pub fn diff_label(d: u8) -> &'static str { if d >= 3 { "VERY HARD" } else { DIFF_NAMES[d as usize] } }
pub const DIFF_NOTES: [&str; 3] = [
    "IPSC cardboard targets, no people. Standing in front of one costs a simulated hit, but never ends the run.",
    "armed threats turn to face you and fire in about a second. Two hits end the run.",
    "threats fire 50% faster and hit 95% of their shots. Two hits end the run.",
];
/// Quantos tiros o operador aguenta: no segundo, a partida acaba.
pub const MAX_WOUNDS: u8 = 2;
/// Quanto dura a queda do operador abatido, antes de a tela de fim aparecer.
pub const DOWN_SECS: f32 = 1.3;
impl Default for Game {
    fn default() -> Self {
        Game { mode: Mode::Menu, time: 0.0, view: View::Fp, tac: true, flash: false, free: false, mission2: false, seed: 1, lesson: 0, obj: Vec::new(), lesson_end: 0.0, debriefed: false,
            flag_tac: true, flag_bird: false, flag_bird_back: false, entry: [None; MAX_DOORWAYS], entry_pie: [None; MAX_DOORWAYS], dwell: [0.0; MAX_DOORWAYS], door_side: [None; MAX_DOORS], stats: Stats::default(),
            bird_zoom: 13.0, sens: 1.0, toast: None, hit_t: 0.0, dmg_t: 0.0, ui_dirty: true, tracers: Vec::new(), burst: Vec::new(), shake: 0.0, near_door: None, track: Vec::new(), replay_t: 0.0, replay_next: None,
            demo: false, demo_req: false, demo_end: false, demo_idle: 0.0, guide_i: 0, guide: 1, guide_t: 0.0, hide_box: false,
            kills: Vec::new(), route_sum: 0.0, route_t: 0.0, lroute_sum: 0.0, lroute_t: 0.0, odds: 100.0, odds_route: 100.0, odds_angle: 100.0, badges: 0, expo_now: 0.0, expo_total: 0.0, coach: None, coach_cd: 0.0, warns: [0; 6], flag_flash: false, cleared: 0, rooms: 0, stamp: String::new(), stamp_t: 0.0, wounds: 0, down_t: 0.0, down_why: String::new(), ckpt_req: false,
            scn: None, scn_t: 0.0, scn_adj: 0, scn_a: 0, scn_b: 0, down_title: String::new(), won: false, passes: 0, outside_kills: 0, locked: [false; MAX_DOORS], breach: None,
            wave: 0, wave_kills: 0, fade_t: 0.0, portal_cd: 0.0, rank: None, ranked: false, lock: 0.0, nade: None, fell: Vec::new(), nade_req: Vec::new(), sub: None, third: std::env::var("CQB_THIRD").is_ok() }
    }
}
/// Uma ameaça derrubada: ângulo da fatia (graus entre o alvo e a borda mais próxima do que o encobria), largura do corpo
/// que o alvo via (cm), tempo desde o primeiro contato e pontos (0 a 100).
#[derive(Clone, Copy, Debug)]
pub struct Kill { pub angle: f32, pub expo_cm: f32, pub dt: f32, pub pts: f32, pub a_zone: bool }

/// Estados do alvo que pode mudar de ideia (`TargetState::mind`): comum; vai se render quando for visto de perto; rendido (não
/// atire); de mãos para o alto mas vai sacar; já sacou (ameaça comum).
pub const MIND_NONE: u8 = 0;
pub const MIND_WILL_YIELD: u8 = 1;
pub const MIND_YIELDED: u8 = 2;
pub const MIND_FAKE: u8 = 3;
pub const MIND_DREW: u8 = 4;
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
    /// Arma em uso (índice em `WEAPONS`), munição guardada de cada arma (carregador, reserva) e troca em andamento (destino, progresso 0..1).
    pub weapon: usize, pub ammo: [(u32, u32); 2], pub switch_to: Option<usize>, pub switch_t: f32,
    /// Recarga em andamento: de emergência (carregador vazio) ou tática, e quanto dura.
    pub reload_empty: bool, pub reload_dur: f32, pub warned_empty: bool,
}
impl Default for Player {
    fn default() -> Self {
        Player { pos: Vec2::new(3.0, 15.4), vel: Vec2::ZERO, yaw: 0.0, pitch: 0.0, lean: 0.0, crouch: 0.0, crouching: false, ads: false, fast: false, speed: 0.0,
            step_acc: 0.0, bob: 0.0, eye: Vec3::new(3.0, EYE_STAND, 15.4), mag: 8, reserve: 64, reloading: false, reload_t: 0.0, fire_cool: 0.0, zone: 0,
            look_acc: 0.0, move_acc: 0.0, ads_acc: 0.0, reloads: 0, shot_n: 0, lean_acc: 0.0, crouch_acc: 0.0,
            g_ads: 0.0, g_kick: 0.0, g_block: 0.0, g_lower: 0.0, g_sway: Vec2::ZERO, recoil: Vec2::ZERO, last_look: Vec2::ZERO, lean_blocked: false,
            weapon: 0, ammo: [(WEAPONS[0].mag, WEAPONS[0].reserve), (WEAPONS[1].mag, WEAPONS[1].reserve)], switch_to: None, switch_t: 0.0,
            reload_empty: false, reload_dur: 2.1, warned_empty: false }
    }
}
impl Player {
    /// Estado inicial mantendo a arma que o jogador tinha escolhido.
    pub fn with_weapon(w: usize) -> Self { let mut p = Player::default(); p.weapon = w.min(WEAPONS.len() - 1); p.mag = WEAPONS[p.weapon].mag; p.reserve = WEAPONS[p.weapon].reserve; p }
    pub fn def(&self) -> &'static WeaponDef { &WEAPONS[self.weapon] }
    pub fn switching(&self) -> bool { self.switch_to.is_some() }
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
    /// Troca de arma: índice pedido (teclas 1 e 2) ou a outra (X).
    pub weapon: Option<usize>, pub weapon_next: bool,
}

/// Táticas de quem caça o operador (`Brain::tactic`).
pub const TAC_NONE: u8 = 0;
/// Indo para o lado do batente, antes de fatiar a porta.
pub const TAC_STACK: u8 = 1;
/// Fatiando a porta: cruza o vão devagar, inclinado, de arma apontada para a quina.
pub const TAC_SLICE: u8 = 2;
/// Recuando para um ponto fora da vista do operador.
pub const TAC_RETREAT: u8 = 3;
/// Abrigado, recompondo-se antes de voltar.
pub const TAC_HOLD: u8 = 4;

/// Cabeça de quem caça o operador: a tática do momento, a inclinação do corpo e as reações (susto, recuo).
#[derive(Clone, Default)]
pub struct Brain {
    /// Tática e relógio dela; ponto onde ele para ao lado do batente (ou o abrigo), quina em que ele gira e fim do fatiar.
    pub tactic: u8, pub t: f32, pub anchor: Vec2, pub pivot: Vec2, pub to: Vec2,
    /// Caminho do recuo, casa a casa.
    pub path: Vec<Vec2>,
    /// Inclinação do corpo (radianos, positiva para a direita dele) e para onde ela vai.
    pub lean: f32, pub lean_to: f32,
    /// Susto: tempo que resta do tranco; motivo ainda não tratado (1 = tiro rente, 2 = parceiro caiu perto, 3 = ferido);
    /// intervalo até poder se assustar de novo com o operador aparecendo de surpresa.
    pub flinch: f32, pub scare: f32, pub startle_cd: f32,
    /// Vão que ele fatiou por último (índice em `Level::gaps`) e por quanto tempo isso ainda vale; quantas vezes já recuou;
    /// arma apontada (pose de mira mesmo sem o operador à vista).
    pub door: i16, pub door_t: f32, pub retreats: u8, pub ready: bool,
    /// Tempo que ele já passou inclinado na quina com o operador à vista (depois de um segundo, sai de trás da parede).
    pub peek: f32,
    /// Vai jogar uma granada pelo vão antes de entrar (porta que já derrubou dois do grupo).
    pub flush: bool,
    /// Tiro através da parede: onde ele viu o operador pela última vez, há quanto tempo perdeu a visada, quantas rajadas já
    /// deu às cegas desde então e intervalo até a próxima; tempo parado depois de uma rajada dessas.
    pub last: Vec2, pub lost: f32, pub bangs: u8, pub bang_cd: f32, pub hold: f32,
    /// Tempo que resta do gesto de arremessar a granada (só para a animação).
    pub toss: f32,
}

pub struct TargetState {
    pub def: TargetDef, pub alive: bool, pub fall: f32, pub hits: u8, pub seen: bool, pub vis: bool, pub react: f32,
    pub contact: bool, pub contact_t: f32, pub first_facing: Option<bool>, pub fwd: Vec2, pub fall_dir: f32, pub fall_max: f32,
    pub pivot: Entity, pub board: Entity, pub mask: usize, pub marks: Vec<Entity>, pub fall_v: f32, pub sw: f32, pub swv: f32, pub landed: bool, pub root: Entity, pub off: bool,
    /// Ameaça que troca de posição dentro do cômodo: vaga de destino (índice em `slots()`), espera antes de sair, se está
    /// andando, tempo sem conseguir avançar, quantas vezes já se mudou e intervalo até poder se mudar de novo.
    pub goal: Option<usize>, pub wait: f32, pub moving: bool, pub stuck: f32, pub moved: u8, pub move_cd: f32,
    /// Em alerta (segundos que restam): já viu o operador ou ouviu tiro, e vira para ele assim que tiver linha de tiro.
    /// `notice` enche enquanto o operador se aproxima por fora do campo de visão dela (0 a 1: em 1 ela percebe).
    pub alert: f32, pub notice: f32, pub engaged: bool,
    /// Ameaça que caça o operador pela casa (invasor ou ameaça que circula); espera até entrar em cena; estado de quem pode
    /// mudar de ideia e relógio desse estado; ameaça que usa um refém como escudo; captor do refém a resgatar; metade dos
    /// invasores conhece o caminho alternativo (laje).
    pub hunter: bool, pub pend: f32, pub mind: u8, pub mind_t: f32, pub shield: bool, pub captor: bool, pub alt: bool,
    /// Tempo que resta da subida depois de uma passagem (o invasor sai do vão da escada ou da laje, de baixo para cima).
    pub rise: f32, pub leave: f32, pub via: u8,
    pub ai: Brain,
}
impl TargetState {
    /// Ameaça viva que ainda precisa ser resolvida (quem se rendeu não conta).
    /// Deslocamento lateral do corpo inclinado à altura `h` (o pé fica no lugar): vale para o olho, a boca da arma e a área de acerto.
    pub fn lean_at(&self, h: f32) -> Vec2 { Vec2::new(-self.fwd.y, self.fwd.x) * (self.ai.lean.sin() * h) }
    pub fn hostile(&self) -> bool { self.alive && !self.off && !self.def.passive && self.def.kind == TKind::Threat && self.mind != MIND_YIELDED }
    /// Zera o que é próprio de cada partida.
    pub fn clear_run(&mut self) {
        self.alive = true; self.fall = 0.0; self.fall_v = 0.0; self.sw = 0.0; self.swv = 0.0; self.landed = false; self.hits = 0; self.seen = false; self.vis = false; self.react = 0.0;
        self.contact = false; self.first_facing = None; self.goal = None; self.wait = 0.0; self.moving = false; self.stuck = 0.0; self.moved = 0; self.move_cd = 0.0;
        self.alert = 0.0; self.notice = 0.0; self.engaged = false; self.hunter = false; self.pend = 0.0; self.mind = MIND_NONE; self.mind_t = 0.0; self.shield = false; self.captor = false; self.alt = false; self.rise = 0.0; self.leave = 0.0; self.via = 0;
        self.ai = Brain::default();
    }
}
#[derive(Resource, Default)]
pub struct Targets { pub list: Vec<TargetState>, pub masks: Vec<Vec<u8>> }
impl Targets {
    pub fn get(&self, id: &str) -> &TargetState { self.list.iter().find(|t| t.def.id == id).expect("alvo") }
    pub fn down(&self, ids: &[&str]) -> bool { ids.iter().all(|id| !self.get(id).alive) }
}

/// Eventos sonoros disparados pela simulação.
#[derive(Event, Clone, Copy)]
pub enum Sfx { Shot, Rifle, Voice(u8), EnemyShot(Vec3), EnemyRound(Vec3), Dry, Click, Reload, Step(bool), Impact(Vec3), Metal(Vec3), Paper(Vec3), Thwack(Vec3), Fall(Vec3), Door(Vec3), Casing(Vec3), Good, Bad, Hurt, Ui, Rustle }

/// Uma arma de primeira pessoa: modelo, clipes de animação e números de jogo.
/// Medidas em metros no espaço da câmera (X direita, Y cima, -Z frente), relativas à pose em que o autor do modelo deixou a arma.
pub struct WeaponDef {
    pub hud: &'static str,
    /// Arquivo do modelo e arquivo de onde vêm os clipes (pode ser o mesmo).
    pub file: &'static str, pub anim: &'static str,
    /// Escala do modelo e posição do olho do autor no arquivo (o modelo olha para +Z).
    pub scale: f32, pub eye: Vec3,
    /// Deslocamento em pronto e na mira, boca do cano e ponto em torno do qual a arma gira.
    pub hip: Vec3, pub ads: Vec3, pub muzzle: Vec3, pub wrist: Vec3,
    /// Inclinação extra na mira (radianos, negativo = cano para baixo) para alinhar alça e massa de mira com o olho.
    pub ads_pitch: f32, pub ads_yaw: f32,
    pub mag: u32, pub reserve: u32, pub cool: f32, pub kick: f32,
    /// Distância da parede à frente a partir da qual a arma precisa ser recolhida (arma longa pede mais recuo).
    pub block: f32,
    /// Duração da recarga tática (ainda há munição no carregador) e da de emergência (carregador vazio).
    pub reload_s: f32, pub reload_empty_s: f32,
    /// Nomes dos clipes no arquivo; vazio = não tem.
    pub idle: &'static str, pub shoot: &'static str, pub reload: &'static str, pub reload_empty: &'static str, pub take: &'static str, pub hide: &'static str, pub walk: &'static str, pub run: &'static str,
}
pub const WEAPONS: [WeaponDef; 2] = [
    WeaponDef { hud: "PISTOL .45", file: "models/pistola_colt.glb", anim: "models/pistola_colt_anim.glb", scale: 0.019, eye: Vec3::new(0.0, 34.0, 0.0),
        hip: Vec3::new(0.035, -0.035, 0.01), ads: Vec3::new(-0.0085, 0.0395, 0.0), muzzle: Vec3::new(0.0125, -0.05, -0.35), wrist: Vec3::new(0.05, -0.1, -0.16),
        ads_pitch: 0.0, ads_yaw: 0.0, mag: 8, reserve: 64, cool: 0.15, kick: 1.0, block: 0.62, reload_s: 2.1, reload_empty_s: 2.6,
        idle: "Idle", shoot: "Shoot", reload: "Reload", reload_empty: "Reload", take: "Take", hide: "Hide", walk: "", run: "" },
    WeaponDef { hud: "SCAR 5.56", file: "models/scar.glb", anim: "models/scar.glb", scale: 0.019, eye: Vec3::new(0.0, 34.0, 0.0),
        hip: Vec3::new(0.012, -0.022, 0.0), ads: Vec3::new(-0.0324, 0.0069, 0.03), muzzle: Vec3::new(0.038, -0.06, -0.55), wrist: Vec3::new(0.05, -0.12, -0.05),
        ads_pitch: -0.03, ads_yaw: 0.0, mag: 30, reserve: 90, cool: 0.12, kick: 0.6, block: 0.84, reload_s: 2.7, reload_empty_s: 3.3,
        idle: "Idle", shoot: "Shoot", reload: "Reload", reload_empty: "Reload", take: "Take", hide: "Hide", walk: "Walk", run: "Run" },
];
