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
        let path = if script { None } else { std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Point of Entry/course.txt")) };
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
