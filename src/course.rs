//! O curso: cinco fases dentro da kill house, em ordem. Cada etapa ensina uma coisa só sobre ângulos e movimentação e
//! destrava a seguinte. As etapas usam o que o jogo já tem (as seis lições e os cenários); aqui ficam a ordem, o que cada
//! uma ensina, a arma e o tipo de alvo de cada fase, e o progresso do aluno (guardado em disco).
use crate::state::*;
use bevy::prelude::*;

pub enum What { Tutorial, Scn(usize) }
pub struct Stage {
    /// Fase (0 a 4), título, o que ensina (uma frase), o conteúdo (lições ou um cenário), tipo de alvo
    /// (0 = papelão, 1 = oposição que reage) e arma (0 = pistola, 1 = carabina).
    pub phase: usize, pub title: &'static str, pub teaches: &'static str, pub what: What, pub diff: u8, pub weapon: usize,
}
pub const PHASES: [(&str, &str); 5] = [
    ("CARDBOARD", "nobody fires back: learn to move, to lean and to read the doorway bar"),
    ("DOORS", "one room at a time: open the doorway from outside before you step in"),
    ("HALLWAYS", "doors facing each other, crossings and a closed door: never turn your back on an angle you have not opened"),
    ("OPPOSITION", "the targets become people who think, and some of them must not be shot"),
    ("FINAL EXAM", "everything at once, against a team that slices doorways and learns from its losses"),
];
pub const STAGES: [Stage; 9] = [
    Stage { phase: 0, title: "Fundamentals", teaches: "Six lessons against cardboard: move, lean, slice a corner in an arc, cross a doorway, identify before you fire.", what: What::Tutorial, diff: 0, weapon: 0 },
    Stage { phase: 1, title: "Threshold", teaches: "Small rooms are cleared from the door. Walk the arc outside; step in only for the corner you could not see.", what: What::Scn(2), diff: 0, weapon: 0 },
    Stage { phase: 1, title: "Clean shot", teaches: "A threat stands behind a hostage. Step sideways until the angle gives you a shot that touches only the threat.", what: What::Scn(0), diff: 0, weapon: 0 },
    Stage { phase: 2, title: "Opposing doors", teaches: "Doors on both sides and a four-way crossing. Every opening you pass without slicing is an angle at your back.", what: What::Scn(1), diff: 0, weapon: 1 },
    Stage { phase: 2, title: "Locked door", teaches: "Breaching is slow and loud. Work it from the side of the frame, off the line of the doorway.", what: What::Scn(4), diff: 0, weapon: 1 },
    Stage { phase: 3, title: "Rear security", teaches: "Threats leave their rooms and come out of areas you already cleared. A cleared angle does not stay cleared.", what: What::Scn(3), diff: 1, weapon: 1 },
    Stage { phase: 3, title: "Second look", teaches: "Some armed people give up, and some with their hands up are waiting. Identify at the moment of the shot.", what: What::Scn(5), diff: 1, weapon: 1 },
    Stage { phase: 3, title: "Hostage rescue", teaches: "One hostage, one captor and a clock. Open only the angles that are on your way.", what: What::Scn(6), diff: 1, weapon: 1 },
    Stage { phase: 4, title: "Home defense", teaches: "Five waves that slice your doorways, fire through the OSB and learn the door that hurt them. Shoot, move, repeat.", what: What::Scn(7), diff: 1, weapon: 1 },
];

/// Progresso do aluno: quantas etapas já venceu (a seguinte fica destravada) e a etapa em curso.
#[derive(Resource)]
pub struct Progress { pub done: usize, pub current: Option<usize>, counted: bool, path: Option<std::path::PathBuf> }
impl Progress {
    pub fn load() -> Self {
        // nos testes automáticos o progresso do jogador não é tocado e todas as etapas ficam abertas
        let script = std::env::var("CQB_SCRIPT").is_ok();
        let path = if script { None } else { std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Angles/course.txt")) };
        let done = path.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| t.trim().parse::<usize>().ok()).unwrap_or(if script { STAGES.len() } else { 0 });
        Progress { done: done.min(STAGES.len()), current: None, counted: false, path }
    }
    fn save(&self) {
        if let Some(f) = &self.path {
            if let Some(d) = f.parent() { let _ = std::fs::create_dir_all(d); }
            let _ = std::fs::write(f, format!("{}\n", self.done));
        }
    }
    pub fn unlocked(&self, i: usize) -> bool { i <= self.done }
}

/// Etapa vencida: chegou à avaliação com a casa resolvida (as lições até o fim; os cenários com nota de 60 para cima; a
/// defesa da casa com as cinco ondas derrubadas). Vencer a etapa mais adiantada destrava a seguinte.
pub fn course_system(game: Res<Game>, targets: Res<Targets>, mut pr: ResMut<Progress>, mut p: ResMut<Player>) {
    let Some(i) = pr.current else { return };
    let st = &STAGES[i];
    // a arma faz parte da etapa: entra com ela
    if game.mode == Mode::Card && game.time < 0.05 && p.weapon != st.weapon && !p.switching() { p.weapon = st.weapon; }
    if game.mode != Mode::Debrief { pr.counted = false; return; }
    if pr.counted { return; }
    pr.counted = true;
    let passed = match st.what {
        What::Tutorial => true,
        What::Scn(k) if crate::scenario::SCENARIOS[k].kind == crate::scenario::Kind::Defense => game.won,
        What::Scn(_) => crate::scenario::final_score(&game, &targets) >= 60,
    };
    if passed && i == pr.done { pr.done = (i + 1).min(STAGES.len()); pr.save(); }
}

/* ---------- frases da tela de fim de partida ---------- */

/// Quanto tempo a frase fica na tela.
pub const QUOTE_SECS: f32 = 3.6;
/// Frases de comandantes e autores militares (texto, autor). As marcadas "attributed" são de atribuição tradicional,
/// sem fonte primária segura.
pub const QUOTES: [(&str, &str); 16] = [
    ("Plans are worthless, but planning is everything.", "Dwight D. Eisenhower"),
    ("A good plan violently executed now is better than a perfect plan executed next week.", "George S. Patton"),
    ("Never interrupt your enemy when he is making a mistake.", "Napoleon Bonaparte (attributed)"),
    ("We will either find a way, or make one.", "Hannibal (attributed)"),
    ("Know the enemy and know yourself; in a hundred battles you will never be in peril.", "Sun Tzu"),
    ("Everything in war is very simple, but the simplest thing is difficult.", "Carl von Clausewitz"),
    ("No plan of operations extends with certainty beyond the first encounter with the enemy's main strength.", "Helmuth von Moltke"),
    ("The whole art of war consists in getting at what is on the other side of the hill.", "Duke of Wellington"),
    ("He who defends everything defends nothing.", "Frederick the Great"),
    ("Train hard, fight easy.", "Alexander Suvorov"),
    ("If you want peace, prepare for war.", "Vegetius"),
    ("Do nothing which is of no use.", "Miyamoto Musashi"),
    ("Sweat saves blood.", "Erwin Rommel (attributed)"),
    ("I came, I saw, I conquered.", "Julius Caesar"),
    ("In war, the moral is to the physical as three is to one.", "Napoleon Bonaparte"),
    ("The more you sweat in peace, the less you bleed in war.", "Norman Schwarzkopf (attributed)"),
];

/* ---------- voz do instrutor ---------- */

/// Roteiro do instrutor: identificador da fala e o texto (que também é a legenda). O áudio de cada fala está em
/// assets/voice/<identificador>.mp3.
const SCRIPT: &str = include_str!("../assets/voice/script.tsv");
fn line(id: &str) -> Option<&'static str> { SCRIPT.lines().find_map(|l| l.split_once('\t').filter(|(k, _)| *k == id).map(|(_, t)| t)) }

#[derive(Default)]
pub struct VoiceState { deb_said: bool, hit_cd: f32, quotes: usize, until: f32, clock: f32, last_mode: Option<Mode>, hits: u32, civ: u32, nade: bool, stall: f32, deg: f32, door: Option<usize>, said_open: Option<usize>, hint_cd: f32, passed: bool, done_seen: usize }

/// Fala do instrutor nos momentos que ensinam: a instrução de cada etapa no cartão, para que lado andar quando o aluno
/// para com a porta pela metade, a porta aberta, tiro levado, refém atingido, granada, e o resultado da etapa.
/// Uma fala por vez; cada uma aparece também como legenda.
#[allow(clippy::too_many_arguments)]
pub fn voice_system(time: Res<Time<Real>>, mut game: ResMut<Game>, pr: Res<Progress>, tac: Res<crate::tactical::Tac>, asset: Res<AssetServer>, mut commands: Commands, mut st: Local<VoiceState>, mut sfx: EventWriter<Sfx>) {
    let dt = time.delta_secs().min(0.1);
    st.clock += dt;
    st.hint_cd = (st.hint_cd - dt).max(0.0);
    st.hit_cd = (st.hit_cd - dt).max(0.0);
    if let Some((_, t)) = game.sub.as_mut() { *t -= dt; if *t <= 0.0 { game.sub = None; } }
    let mut say: Option<String> = None;
    let mode = game.mode;
    let entered = st.last_mode != Some(mode);
    let prev = st.last_mode;
    st.last_mode = Some(mode);
    game.talk = (game.talk - dt).max(0.0);
    // fim da partida: uma tela rápida com uma frase histórica antes da avaliação (qualquer tecla pula)
    // a mesma tela aparece ao abrir o jogo e quando o jogador cai (não só quando a casa é limpa)
    let over = matches!(mode, Mode::Debrief | Mode::Down) && !game.demo && prev != Some(Mode::Down) && prev != Some(Mode::Ranking);
    if entered && (over || prev.is_none()) && std::env::var("CQB_SCRIPT").is_err() {
        st.quotes = st.quotes.wrapping_add(1);
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as usize).unwrap_or(0);
        game.quote_i = (st.quotes + secs) % QUOTES.len();
        game.quote_t = QUOTE_SECS; game.ui_dirty = true;
    }
    if entered && game.quote_t < QUOTE_SECS { game.quote_t = 0.0; game.ui_dirty = true; }
    if game.quote_t > 0.0 { game.quote_t = (game.quote_t - dt).max(0.0); if game.quote_t <= 0.0 { game.ui_dirty = true; } }
    if mode != Mode::Debrief { st.deb_said = false; }
    // contagem antes de começar: um bipe por segundo e a partida abre no zero
    if game.ready_t > 0.0 {
        let before = game.ready_t.ceil();
        game.ready_t = (game.ready_t - dt).max(0.0);
        if game.ready_t <= 0.0 || game.ready_t.ceil() < before { game.ui_dirty = true; }
        if game.mode != Mode::Card { game.ready_t = 0.0; }
        else if game.ready_t <= 0.0 { game.mode = Mode::Play; sfx.write(Sfx::Good); }
        else if game.ready_t.ceil() < before { sfx.write(Sfx::Ui); }
    }
    if let Some(i) = pr.current {
        if entered && mode == Mode::Card { say = Some(format!("stage{}", i + 1)); st.hits = 0; st.civ = 0; st.said_open = None; }
        // o resultado só é dito depois que a tela da frase sai
        if mode == Mode::Debrief && game.quote_t <= 0.0 && !st.deb_said {
            st.deb_said = true;
            let passed = pr.done > st.done_seen || pr.done > i;
            say = Some(if pr.done >= STAGES.len() && pr.done > st.done_seen { "course_done" } else if passed { "stage_passed" } else { "stage_failed" }.to_string());
            st.passed = passed;
        }
    }
    st.done_seen = pr.done;
    if mode == Mode::Play && !game.demo {
        if game.stats.civ_hit > st.civ { say = Some("hostage_hit".into()); }
        else if game.stats.hits_taken > st.hits && st.hit_cd <= 0.0 { say = Some("hit".into()); st.hit_cd = 14.0; }
        else if game.nade.is_some() && !st.nade { say = Some("grenade".into()); }
        st.civ = game.stats.civ_hit; st.hits = game.stats.hits_taken; st.nade = game.nade.is_some();
        // porta em foco: aberta de vez, ou o aluno parado com ela pela metade
        if let Some(d) = tac.pie_active {
            let deg = tac.pie_deg(d);
            if st.door != Some(d) { st.door = Some(d); st.deg = deg; st.stall = 0.0; }
            if deg > st.deg + 1.0 { st.deg = deg; st.stall = 0.0; } else { st.stall += dt; }
            if say.is_none() && deg >= 150.0 && st.said_open != Some(d) { st.said_open = Some(d); say = Some("door_open".into()); }
            else if say.is_none() && game.tac && st.stall > 3.5 && st.hint_cd <= 0.0 && deg < 140.0 && (tac.bar_left > 0 || tac.bar_right > 0) {
                say = Some(if tac.bar_left >= tac.bar_right { "step_left" } else { "step_right" }.to_string());
                st.hint_cd = 9.0; st.stall = 0.0;
            }
        } else { st.door = None; }
    }
    let Some(id) = say else { return };
    // não atropela a fala anterior, a não ser para avisos que não podem esperar
    // (e não fala por cima de um personagem: espera a vez)
    let urgent = matches!(id.as_str(), "grenade" | "hostage_hit") || id.starts_with("stage");
    if (st.clock < st.until || game.talk > 0.0) && !urgent { return; }
    let Some(text) = line(&id) else { return };
    let secs = 1.0 + text.len() as f32 * 0.062;
    st.until = st.clock + secs;
    game.talk = secs;
    game.sub = Some((text.to_string(), secs + 0.4)); game.ui_dirty = true;
    commands.spawn((AudioPlayer::new(asset.load::<AudioSource>(format!("voice/{id}.mp3"))), PlaybackSettings { mode: bevy::audio::PlaybackMode::Despawn, volume: bevy::audio::Volume::Linear(0.9), ..default() }));
}
