//! Ranking mundial e contagem de uso. O jogo manda para o servidor do site (angles-kz.vercel.app) a pontuação de cada
//! partida terminada, com o codinome escolhido, e avisos de uso (abriu o jogo, começou e terminou uma etapa). Nada de
//! dado pessoal: cada instalação tem só um identificador sorteado. Tudo acontece em segundo plano e, sem internet, o
//! jogo segue igual. Para desligar: variável ANGLES_OFFLINE=1 ou um arquivo chamado "offline" na pasta de dados do jogo.
use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use std::sync::{Arc, Mutex};
use crate::state::*;

pub const API: &str = "https://angles-kz.vercel.app/api";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Resource)]
pub struct Net {
    pub id: String, pub name: String, pub on: bool, dir: Option<std::path::PathBuf>,
    /// O jogador já escolheu um codinome (existe callsign.txt)? Sem isso, o menu pede o nome na primeira abertura.
    pub named: bool,
    /// Resposta do ranking mundial (JSON) e a colocação da última partida enviada, quando chegam.
    pub world: Arc<Mutex<Option<serde_json::Value>>>, rank: Arc<Mutex<Option<(u64, u64)>>>, fresh: Arc<Mutex<bool>>,
}

fn data_dir() -> Option<std::path::PathBuf> { std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join("Library/Application Support/Angles")) }

impl Net {
    pub fn load() -> Self {
        let script = std::env::var("CQB_SCRIPT").is_ok();
        let dir = if script { None } else { data_dir() };
        let on = !script && std::env::var("ANGLES_OFFLINE").is_err() && !dir.as_ref().is_some_and(|d| d.join("offline").exists());
        let read = |f: &str| dir.as_ref().and_then(|d| std::fs::read_to_string(d.join(f)).ok()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        let id = read("install.txt").filter(|t| t.len() >= 12 && t.chars().all(|c| c.is_ascii_hexdigit())).unwrap_or_else(|| {
            // identificador sorteado, sem relação com a máquina nem com a pessoa
            use std::hash::{BuildHasher, Hasher};
            let mut s = String::new();
            for k in 0..2u64 { let mut h = std::collections::hash_map::RandomState::new().build_hasher(); h.write_u64(k); s.push_str(&format!("{:016x}", h.finish())); }
            if let Some(d) = &dir { let _ = std::fs::create_dir_all(d); let _ = std::fs::write(d.join("install.txt"), &s); }
            s
        });
        let saved = read("callsign.txt").map(|t| clean_name(&t)).filter(|t| !t.is_empty());
        let named = saved.is_some() || !on;
        let name = saved.unwrap_or_else(|| "OPERATOR".to_string());
        Net { id, name, on, dir, named, world: Arc::new(Mutex::new(None)), rank: Arc::new(Mutex::new(None)), fresh: Arc::new(Mutex::new(false)) }
    }
    fn save_name(&self) { if let Some(d) = &self.dir { let _ = std::fs::create_dir_all(d); let _ = std::fs::write(d.join("callsign.txt"), &self.name); } }
    fn post(&self, path: &'static str, body: serde_json::Value, rank: Option<Arc<Mutex<Option<(u64, u64)>>>>) {
        if !self.on { return; }
        let fresh = self.fresh.clone();
        std::thread::spawn(move || {
            let r = ureq::post(&format!("{API}/{path}")).timeout(std::time::Duration::from_secs(8)).set("content-type", "application/json").send_string(&body.to_string());
            if let (Ok(r), Some(slot)) = (r, rank) {
                if let Ok(v) = r.into_json::<serde_json::Value>() {
                    if let (Some(k), Some(n)) = (v["rank"].as_u64(), v["total"].as_u64()) { *slot.lock().unwrap() = Some((k, n)); *fresh.lock().unwrap() = true; }
                }
            }
        });
    }
    /// Aviso de uso: "launch", "start" ou "end".
    pub fn event(&self, kind: &str, board: Option<&str>, passed: bool, time: f32) {
        self.post("event", serde_json::json!({ "kind": kind, "id": self.id, "v": VERSION, "board": board, "passed": passed, "time": time }), None);
    }
    /// Manda a partida para o ranking mundial; a colocação chega depois e vira um aviso na tela.
    pub fn submit(&self, board: &str, score: i32, time: f32, diff: u8) {
        self.post("scores", serde_json::json!({ "board": board, "name": self.name, "score": score, "time": time, "diff": diff, "id": self.id, "v": VERSION }), Some(self.rank.clone()));
    }
    /// Busca os melhores do mundo em cada tabela.
    pub fn fetch(&self) {
        if !self.on { return; }
        let (world, fresh) = (self.world.clone(), self.fresh.clone());
        std::thread::spawn(move || {
            if let Ok(r) = ureq::get(&format!("{API}/scores?n=5")).timeout(std::time::Duration::from_secs(8)).call() {
                if let Ok(v) = r.into_json::<serde_json::Value>() { *world.lock().unwrap() = Some(v); *fresh.lock().unwrap() = true; }
            }
        });
    }
    /// Os melhores do mundo numa tabela: (codinome, pontos, tempo).
    pub fn top(&self, board: &str) -> Vec<(String, i64, f32)> {
        let g = self.world.lock().unwrap();
        g.as_ref().and_then(|v| v["boards"][board].as_array().cloned()).unwrap_or_default().iter()
            .map(|r| (r["name"].as_str().unwrap_or("?").to_string(), r["score"].as_i64().unwrap_or(0), r["time"].as_f64().unwrap_or(0.0) as f32)).collect()
    }
    pub fn boards(&self) -> Vec<String> {
        let g = self.world.lock().unwrap();
        g.as_ref().and_then(|v| v["boards"].as_object().map(|o| o.keys().cloned().collect())).unwrap_or_default()
    }
}

/// Codinome: só letras, números, espaço, hífen e sublinhado, em maiúsculas, até 12 caracteres.
pub fn clean_name(t: &str) -> String { t.to_uppercase().chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '_').take(12).collect::<String>().trim_start().to_string() }

#[derive(Default)]
pub struct NetState { launched: bool, started: bool, last_time: f32, last_mode: Option<Mode>, grace: u8 }

/// Avisos de uso (abriu, começou uma etapa), busca do ranking mundial ao abrir a tela dele, edição do codinome ali,
/// e o aviso da colocação mundial quando o servidor responde.
pub fn net_system(mut game: ResMut<Game>, mut net: ResMut<Net>, mut keys: EventReader<KeyboardInput>, mut st: Local<NetState>) {
    if !st.launched { st.launched = true; net.event("launch", None, false, 0.0); }
    if game.time < st.last_time || matches!(game.mode, Mode::Menu | Mode::Scenarios) { st.started = false; }
    st.last_time = game.time;
    if game.mode == Mode::Play && !st.started && !game.demo {
        st.started = true;
        net.event("start", Some(&crate::scenario::board(&game)), false, 0.0);
    }
    let entered = st.last_mode != Some(game.mode);
    st.last_mode = Some(game.mode);
    // digitar o codinome: na tela de ranking, ou no menu enquanto o jogador ainda não escolheu um
    let ask = game.mode == Mode::Menu && !net.named;
    if game.mode == Mode::Ranking && entered { net.fetch(); }
    // com a tela da frase na frente, as teclas só servem para pular a frase
    if (game.mode == Mode::Ranking || ask) && game.quote_t <= 0.0 {
        let (mut changed, mut confirm) = (false, false);
        for k in keys.read() {
            if !k.state.is_pressed() { continue; }
            match &k.logical_key {
                // o primeiro toque no menu substitui o nome de fábrica em vez de emendar nele
                Key::Character(s) => { let base = if ask && net.name == "OPERATOR" { String::new() } else { net.name.clone() }; let t = clean_name(&format!("{base}{s}")); if t != net.name { net.name = t; changed = true; } }
                Key::Space => { if net.name.len() < 12 && !net.name.is_empty() { net.name.push(' '); changed = true; } }
                Key::Backspace => { if net.name.pop().is_some() { changed = true; } }
                Key::Enter => confirm = true,
                _ => {}
            }
        }
        if changed { if !ask { net.save_name(); } game.ui_dirty = true; }
        if ask && confirm && !net.name.trim().is_empty() { net.name = net.name.trim().to_string(); net.named = true; net.save_name(); st.grace = 20; game.ui_dirty = true; }
    } else { keys.clear(); }
    // o Enter que confirma o nome não pode também começar o curso: o menu fica "ocupado" por alguns quadros
    st.grace = st.grace.saturating_sub(1);
    let naming = ask || st.grace > 0;
    if game.naming != naming { game.naming = naming; game.ui_dirty = true; }
    let fresh = { let mut f = net.fresh.lock().unwrap(); std::mem::replace(&mut *f, false) };
    if fresh {
        game.ui_dirty = true;
        let r = net.rank.lock().unwrap().take();
        if let Some((k, n)) = r { if game.mode == Mode::Debrief || game.mode == Mode::Down { game.toast(&format!("World ranking: #{k} of {n} runs on this board."), 1, 7.0); } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        assert_eq!(clean_name("cesar m!@#"), "CESAR M");
        assert_eq!(clean_name("  abcdefghijklmnopq"), "ABCDEFGHIJ");
        assert_eq!(clean_name("<script>"), "SCRIPT");
    }
    /// Conversa de verdade com o servidor (só com ANGLES_NET_TEST=1): manda uma partida na tabela de teste e lê de volta.
    #[test]
    fn live_roundtrip() {
        if std::env::var("ANGLES_NET_TEST").is_err() { return; }
        let net = Net { id: "abcdef0123456789".into(), name: "RUSTTEST".into(), on: true, dir: None, named: true, world: Arc::new(Mutex::new(None)), rank: Arc::new(Mutex::new(None)), fresh: Arc::new(Mutex::new(false)) };
        net.submit("__test", 88, 33.3, 1);
        net.event("launch", None, false, 0.0);
        for _ in 0..100 { if net.rank.lock().unwrap().is_some() { break; } std::thread::sleep(std::time::Duration::from_millis(150)); }
        let r = net.rank.lock().unwrap().clone();
        println!("COLOCACAO {r:?}");
        assert!(r.is_some(), "o servidor não respondeu com a colocação");
        *net.fresh.lock().unwrap() = false;
        net.fetch();
        for _ in 0..100 { if *net.fresh.lock().unwrap() { break; } std::thread::sleep(std::time::Duration::from_millis(150)); }
        let top = net.top("__test");
        println!("TOPO {top:?}");
        assert!(top.iter().any(|(n, s, _)| n == "RUSTTEST" && *s == 88));
    }
}
