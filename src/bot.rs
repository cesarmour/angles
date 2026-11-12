//! Roteiro automático para testar o percurso e capturar telas (variável de ambiente CQB_SCRIPT).
use crate::lessons::{ideal, Step};
use crate::map::{ang_diff, Level, TKind};
use crate::state::*;
use crate::tactical::Tac;
use crate::ui::ResetGame;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum Cmd {
    Wait(u32), Start(bool), Confirm, Tp(f32, f32, f32, f32), Go(f32, f32, Option<(f32, f32)>), Face(f32, f32), Shoot(&'static str), Spin, Interact, Reload,
    Ads(u32), Lean(f32, u32), Crouch(u32), Tac(bool), ViewTo(View), Flash(bool), Shot(&'static str), Snap(&'static str), Look(u32), Fire, Quit, Mission2, Pause, WaitReplay,
    Say(&'static str), OpenDoor(usize), DemoOf(usize), WaitDemo, Map(usize), Weapon(usize), Credits, Diff(u8), Menu, Scn(usize), Screen(Mode), Aim(bool, usize), Who(u8, f32), Auto(bool), Hold(u32), TpEnv, WaitNade,
    /// Espera um invasor começar a sair do vão de chegada de uma passagem entre andares.
    WaitClimb,
    /// Vai para a frente do n-ésimo refém (true) ou da n-ésima ameaça (false), à distância dada, olhando para ele.
    Near(bool, usize, f32),
}

#[derive(Resource, Default)]
pub struct Bot { pub fps: f32, pub auto_card: bool, pub card_n: u32, pub active: bool, pub queue: VecDeque<Cmd>, pub n: u32, pub out: String, pub hold_ads: bool, pub hold_lean: f32, pub hold_crouch: bool,
    /// Fila da demonstração (separada da fila do roteiro de teste), ritmo do passo e contador de quadros da demonstração.
    pub demo_q: VecDeque<Cmd>, pub pace: f32, pub demo_frames: u32, pub engage_n: u32,
    /// Tempo (real) desde que o operador automático começou a encarar uma ameaça e desde o último tiro dele.
    pub engage_t: f32, pub fire_t: f32,
    /// Posição de guarda (e para onde olhar), caminho de fuga de granada (CQB_FLEE) e quantos pontos dele já foram alcançados.
    pub home: Option<(Vec2, Vec2)>, pub flee: Vec<Vec2>, pub flee_i: usize }

/// Converte o traçado ideal de uma lição em comandos. Na demonstração entram também o ponto de partida e as legendas.
pub fn steps_cmds(l: usize, demo: bool) -> Vec<Cmd> {
    let mut v = Vec::new();
    for s in ideal(l) {
        match *s {
            Step::Start(x, z, lx, lz) => if demo { v.push(Cmd::Tp(x, z, (-(lx - x)).atan2(-(lz - z)), -0.04)); v.push(Cmd::Wait(50)); },
            Step::Go(x, z, lx, lz) => v.push(Cmd::Go(x, z, Some((lx, lz)))),
            Step::Mv(x, z) => v.push(Cmd::Go(x, z, None)),
            Step::Shoot(id) => v.push(Cmd::Shoot(id)),
            Step::Open(i) => { v.push(Cmd::OpenDoor(i)); v.push(Cmd::Wait(70)); }
            Step::OpenNow(i) => { v.push(Cmd::OpenDoor(i)); v.push(Cmd::Wait(4)); }
            Step::Lean(a) => v.push(if a != 0.0 { Cmd::Lean(a, 60) } else { Cmd::Lean(0.0, 0) }),
            Step::Say(m) => if demo { v.push(Cmd::Say(m)); v.push(Cmd::Wait(70)); },
            Step::Scan => v.push(Cmd::Spin),
            Step::Look(x, z) => { v.push(Cmd::Face(x, z)); v.push(Cmd::Wait(if demo { 40 } else { 10 })); }
        }
    }
    v
}

pub fn setup_bot(mut bot: ResMut<Bot>, mut time: ResMut<Time<Virtual>>) {
    let Ok(script) = std::env::var("CQB_SCRIPT") else { return };
    bot.active = true;
    bot.out = std::env::var("CQB_OUT").unwrap_or_else(|_| ".".into());
    bot.pace = std::env::var("CQB_PACE").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    time.set_relative_speed(std::env::var("CQB_SPEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0));
    use Cmd::*;
    let q: Vec<Cmd> = match script.as_str() {
        "shots" => vec![
            Wait(90), Shot("01-menu"), Snap("menu"), Start(false), Wait(20), Shot("02-cartao"), Confirm, Wait(40),
            Tp(3.0, 16.6, 0.0, -0.02), Wait(40), Shot("03-preparacao"),
            Ads(50), Shot("04-mira"), Ads(0),
            Tp(3.0, 14.8, 0.0, 0.0), Wait(20), Fire, Wait(4), Shot("05-disparo"), Wait(20),
            Tac(true), Tp(4.7, 7.3, -0.75, -0.12), Wait(50), Shot("06-quina-tatica"),
            Lean(-1.0, 50), Shot("07-quina-inclinado"), Lean(0.0, 0),
            Tp(4.55, 6.3, -1.1, -0.1), Wait(40), Shot("08-quina-mais-aberta"),
            ViewTo(View::Bird), Wait(60), Shot("09-quina-aerea"), ViewTo(View::Fp),
            Tp(9.0, 4.9, -2.5, -0.1), Wait(50), Shot("10-porta-sala-a"),
            ViewTo(View::Bird), Wait(60), Shot("11-porta-aerea"), ViewTo(View::Fp),
            Tac(false), Tp(10.0, 10.6, 0.0, -0.05), Wait(30), Shot("12-barricada-em-pe"), Crouch(50), Shot("13-barricada-agachado"), Crouch(0),
            Tp(18.6, 10.6, 0.9, -0.02), Wait(40), Shot("14-sala-b"),
            Tp(15.6, 9.0, 0.35, 0.0), Wait(10), Ads(50), Shot("15-identificacao"), Ads(0), Snap("sala b"),
            Quit,
        ],
        "extra" => vec![
            Wait(60), Mission2, Wait(60), Shot("30-missao2"), Tac(true), Wait(30), ViewTo(View::Bird), Wait(50), Shot("31-missao2-aerea"), ViewTo(View::Fp),
            Tp(18.6, 10.6, 0.9, -0.02), Wait(30), Face(20.2, 6.85), Wait(10), Fire, Wait(12), Fire, Wait(20), Shot("32-alvo-caindo"), Wait(60), Shot("33-alvo-caido"),
            Pause, Wait(20), Shot("34-pausa"), Quit,
        ],
        "cal" => vec![
            Wait(60), Start(false), Wait(10), Confirm, Wait(30), Tp(3.0, 16.6, 0.0, -0.02), Wait(40), Shot("c1-preparacao"),
            Tp(4.7, 7.3, -0.75, -0.12), Wait(40), Shot("c2-corredor"), Tac(true), Wait(20), Shot("c3-tatica"),
            ViewTo(View::Bird), Wait(40), Shot("c4-aerea"), ViewTo(View::Fp), Tac(false),
            Tp(18.6, 10.6, 0.9, -0.02), Wait(120), Shot("c5-sala-b"), Snap("sala b"), Tp(4.7, 7.3, -0.75, -0.12), Wait(120), Snap("corredor"), Quit,
        ],
        "spin" => vec![Wait(60), Start(true), Wait(30), Tp(3.0, 15.4, 0.0, 0.0), Wait(5), Spin, Tp(5.0, 8.0, 0.0, 0.0), Wait(5), Spin, Tp(10.0, 8.0, 0.0, 0.0), Wait(5), Spin, Tp(12.2, 10.95, 0.0, 0.0), Wait(5), Spin, Tp(17.0, 9.0, 0.0, 0.0), Wait(5), Spin, Tp(9.0, 4.45, 0.0, 0.0), Wait(5), Spin, Quit],
        "flick" => vec![Wait(60), Start(true), Wait(330), Tp(3.0, 16.6, 0.0, -0.1), Wait(120), Shot("f1a"), Shot("f1b"), Tp(4.7, 7.3, -0.75, -0.25), Wait(120), Shot("f2a"), Shot("f2b"), Tp(10.0, 10.6, 0.0, -0.3), Wait(120), Shot("f3a"), Shot("f3b"), Tp(15.6, 9.0, 0.35, -0.15), Wait(120), Shot("f4a"), Shot("f4b"), Quit],
        "flick2" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(30), Tp(10.0, 10.6, 0.0, -0.3), Wait(150), Shot("g00"), Shot("g01"), Shot("g02"), Shot("g03"), Shot("g04"), Shot("g05"), Shot("g06"), Shot("g07"), Shot("g08"), Shot("g09"), Shot("g10"), Shot("g11"), Shot("g12"), Shot("g13"), Shot("g14"), Shot("g15"), Shot("g16"), Shot("g17"), Shot("g18"), Shot("g19"), Shot("g20"), Shot("g21"), Shot("g22"), Shot("g23"), Quit],
        "flick3" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(30), Tp(10.5, 9.0, 0.0, 0.0), ViewTo(View::Bird), Wait(150), Shot("g00"), Shot("g01"), Shot("g02"), Shot("g03"), Shot("g04"), Shot("g05"), Shot("g06"), Shot("g07"), Shot("g08"), Shot("g09"), Shot("g10"), Shot("g11"), Shot("g12"), Shot("g13"), Shot("g14"), Shot("g15"), Shot("g16"), Shot("g17"), Shot("g18"), Shot("g19"), Shot("g20"), Shot("g21"), Shot("g22"), Shot("g23"), Quit],
        "blk" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(30), Tp(11.3, 5.3, 2.14, -0.05), Wait(90), Shot("blk1"), Tp(12.6, 5.2, 1.75, -0.1), Wait(60), Shot("blk2"), Tp(4.7, 7.3, -0.75, -0.12), Wait(60), Shot("blk3"), Flash(false), Tp(11.3, 5.3, 2.14, -0.05), Wait(90), Shot("blk1n"), Quit],
        "ang" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(30), Tac(true), Tp(5.0, 7.2, 0.0, -0.05), Face(8.0, 5.0), Wait(40), Snap("pronto"), Ads(50), Snap("mirando"), Ads(0), Wait(30), Lean(1.0, 50), Snap("inclina dir"), Lean(-1.0, 60), Snap("inclina esq"), Lean(0.0, 0), Wait(20),
            Tp(5.62, 6.6, 0.0, -0.05), Face(8.0, 5.0), Wait(40), Snap("colado pronto"), Lean(1.0, 50), Snap("colado dir"), Lean(-1.0, 60), Snap("colado esq"), Lean(0.0, 0), Quit],
        "flick4" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(30), Tp(3.0, 17.2, 0.0, 0.55), Wait(120), Shot("g00"), Shot("g01"), Shot("g02"), Shot("g03"), Shot("g04"), Shot("g05"), Shot("g06"), Shot("g07"), Shot("g08"), Shot("g09"), Shot("g10"), Shot("g11"), Tp(5.0, 11.5, 0.0, 0.18), Wait(120), Shot("g12"), Shot("g13"), Shot("g14"), Shot("g15"), Shot("g16"), Shot("g17"), Shot("g18"), Shot("g19"), Shot("g20"), Shot("g21"), Shot("g22"), Shot("g23"), Quit],
        // um mapa novo por vez: vista de cima sem névoa (CQB_NOFOW=1), primeira pessoa no início e um giro para medir desempenho
        "maps" => { let mut v = vec![Wait(60)]; for (i, (a, b, c)) in [("m1-top", "m1-fp", "m1-tac"), ("m2-top", "m2-fp", "m2-tac"), ("m3-top", "m3-fp", "m3-tac")].into_iter().enumerate() {
            v.extend([Map(i + 1), Wait(20), Start(true), Wait(240), Shot(b), Tac(true), ViewTo(View::Bird), Wait(60), Shot(a), ViewTo(View::Fp), Spin, Shot(c), Snap("mapa"), Tac(false), Pause, Wait(10)]);
            v.extend([Wait(5)]);
        }
            // porta fechada num mapa novo (cozinha do apartamento): abre de lado e atravessa
            v.extend([Map(1), Wait(20), Start(true), Wait(60), Tp(8.1, 6.9, 0.0, 0.0), Face(8.95, 6.0), Wait(10), Interact, Wait(90), Go(8.95, 6.9, None), Go(8.95, 4.8, None), Snap("porta"), Shot("m1-cozinha"), Quit]); v },
        // as duas armas: em pronto, na mira, atirando, no meio da troca e nas duas recargas
        "guns" => { let mut v = vec![Wait(60), Start(false), Wait(20), Confirm, Wait(40), Tp(3.0, 16.6, 0.0, -0.02), Wait(70)];
            for (i, n) in [(0usize, ["w0-hip", "w0-ads", "w0-fire", "w0-reload", "w0-switch"]), (1, ["w1-hip", "w1-ads", "w1-fire", "w1-reload", "w1-switch"])] {
                if i > 0 { v.extend([Weapon(i), Wait(40), Shot(n[4]), Wait(160)]); }
                v.extend([Shot(n[0]), Ads(90), Shot(n[1]), Fire, Wait(3), Shot(n[2]), Ads(0), Wait(60), Reload, Wait(150), Shot(n[3]), Snap("arma"), Wait(420)]);
            }
            v.extend([Go(3.0, 14.2, None), Shot("w1-walk"), Weapon(0), Wait(200), Go(3.0, 16.0, None), Shot("w0-walk"), Snap("fim"), Quit]); v },
        "ads" => vec![Wait(60), Start(false), Wait(20), Confirm, Wait(40), Tp(3.0, 16.6, 0.0, -0.02), Wait(70), Weapon(1), Wait(200), Ads(120), Shot("a1"), Ads(0), Quit],
        // esvazia o carregador e recarrega: pistola e SCAR
        "empty" => { let mut v = vec![Wait(60), Start(false), Wait(20), Confirm, Wait(40), Tp(3.0, 16.6, 0.0, -0.02), Wait(70)];
            for _ in 0..8 { v.extend([Fire, Wait(24)]); }
            v.extend([Reload, Wait(70), Shot("e0-emergency"), Snap("pistola recarregando"), Wait(420), Snap("pistola cheia"), Weapon(1), Wait(200)]);
            for _ in 0..30 { v.extend([Fire, Wait(20)]); }
            v.extend([Reload, Wait(220), Shot("e1-emergency"), Snap("scar recarregando"), Wait(420), Snap("scar cheio"), Fire, Wait(30), Reload, Wait(60), Snap("scar tática"), Wait(420), Snap("scar 30+1"), Quit]); v },
        "host" => vec![Wait(60), Start(true), Wait(120), Near(true, 0, 3.2), Wait(120), Shot("h1"), Near(true, 1, 2.0), Wait(120), Shot("h2"), Near(false, 1, 3.5), Wait(120), Shot("t1"), Near(false, 2, 2.2), Wait(90), Shot("t2"), Wait(200), Shot("t3"),
            Near(true, 0, 3.0), Wait(60), Fire, Wait(60), Shot("h3"), Wait(200), Shot("h4"), ViewTo(View::Bird), Wait(120), Shot("h5"), Snap("fim"), Quit],
        // ameaça de frente: levanta o fuzil ao ver o operador e atira; várias capturas seguidas para pegar o clarão
        "enemy" => vec![Wait(60), Start(true), Wait(120), Near(false, 1, 3.0), Wait(50), Shot("e01"), Wait(130), Shot("e02"), Shot("e03"), Shot("e04"), Shot("e05"), Shot("e06"), Shot("e07"), Shot("e08"), Shot("e09"),
            Near(true, 0, 2.6), Wait(140), Shot("e10"), Snap("fim"), Quit],
        // queda: derruba uma ameaça e um refém de perto e captura a sequência
        "fall" => vec![Wait(60), Start(true), Wait(120), Near(false, 1, 3.0), Wait(30), Ads(40), Fire, Wait(10), Shot("f1"), Wait(14), Fire, Shot("f2"), Wait(70), Shot("f3"), Wait(120), Shot("f4"), Wait(240), Ads(0),
            Near(true, 0, 3.0), Wait(60), Fire, Wait(14), Shot("f5"), Wait(80), Shot("f6"), Wait(140), Shot("f7"), Wait(240), ViewTo(View::Bird), Wait(120), Shot("f8"), Snap("fim"), Quit],
        // quedas sorteadas: derruba quatro ameaças seguidas e mostra cada uma caída
        "falls" => { let mut v = vec![Wait(60), Start(true), Wait(120)];
            for (i, n) in ["q0", "q1", "q2", "q3"].into_iter().enumerate() { v.extend([Near(false, i, 3.0), Wait(30), Fire, Wait(14), Fire, Wait(420), Shot(n)]); }
            v.extend([Snap("fim"), Quit]); v },
        // reféns de perto (cada uma com a sua combinação de cores) e o corpo 3D do jogador na vista de cima
        "looks" => vec![Wait(60), Start(true), Wait(150), Near(true, 0, 2.6), Wait(90), Shot("k0"), Near(true, 1, 2.6), Wait(90), Shot("k1"), Near(true, 2, 2.6), Wait(90), Shot("k2"), Near(true, 3, 2.6), Wait(90), Shot("k3"),
            Tp(3.0, 15.4, 0.6, 0.0), ViewTo(View::Bird), Wait(150), Shot("b0"), Ads(100), Shot("b1"), Fire, Wait(4), Shot("b2"), Ads(0), Go(3.0, 13.0, None), Shot("b3"), Quit],
        // refém baixa os braços quando o cômodo dela fica limpo: antes (ameaças vivas) e depois (todas derrubadas, cômodo visto)
        "relax" => { let mut v = vec![Wait(60), Start(true), Wait(150), Near(true, 0, 3.0), Wait(60), Spin, Near(true, 0, 3.0), Wait(120), Shot("x0")];
            for _ in 0..12 { v.extend([Near(false, 0, 2.4), Wait(20), Fire, Wait(16), Fire, Wait(16)]); }
            v.extend([Near(true, 0, 3.0), Wait(30), Spin, Near(true, 0, 3.0), Wait(200), Shot("x1"), Near(true, 1, 3.0), Wait(30), Spin, Near(true, 1, 3.0), Wait(200), Shot("x2"), Snap("fim"), Quit]); v },
        // inimigos trocando de posição: um tiro no início e várias capturas de cima; depois o operador andando visto de cima
        "move" => vec![Wait(60), Start(true), Wait(150), Fire, Wait(60), Snap("apos tiro"), Wait(120), Snap("2 s"), Wait(240), Snap("4 s"),
            Near(false, 0, 3.4), Wait(10), Shot("v0"), Fire, Wait(200), Near(false, 1, 3.4), Wait(20), Shot("v1"),
            Tp(3.0, 15.4, 0.0, 0.0), ViewTo(View::Bird), Wait(60), Go(3.0, 13.2, None), Shot("v2"), Go(3.0, 12.4, None), Shot("v3"), Snap("fim"), Quit],
        // tela de créditos (para conferir se o texto cabe)
        "credits" => vec![Wait(120), Credits, Wait(60), Shot("credits"), Quit],
        // cada cenário: cartão de instruções e primeiro quadro da partida; depois as telas de cenários e de ranking
        // vistas de cima (com CQB_NOFOW=1 CQB_NOREACT=1 CQB_ZOOM=24): refém como escudo, corredor, resgate e casa
        "tops" => vec![Wait(120),
            Scn(0), Wait(150), Confirm, Wait(30), Tp(15.0, 6.0, 0.0, 0.0), ViewTo(View::Bird), Wait(90), Shot("top-shield"),
            Scn(1), Wait(150), Confirm, Wait(30), Tp(10.0, 9.0, 0.0, 0.0), ViewTo(View::Bird), Wait(90), Shot("top-hall"),
            Scn(6), Wait(150), Confirm, Wait(30), Tp(10.0, 8.0, 0.0, 0.0), ViewTo(View::Bird), Wait(90), Shot("top-rescue"),
            Scn(7), Wait(150), Confirm, Wait(30), Tp(7.0, 9.0, 0.0, 0.0), ViewTo(View::Bird), Wait(90), Shot("top-home"), Quit],
        // porta trancada: arromba de frente e depois de lado
        "lock" => vec![Wait(120), Scn(4), Wait(150), Confirm, Wait(30), Tp(3.95, 7.1, 0.0, 0.0), Wait(30), Shot("lk-prompt"), Interact, Wait(60), Shot("lk-breaching"), Wait(200), Snap("frente"), Shot("lk-open"),
            Tp(9.3, 7.0, 0.0, 0.0), Wait(20), Interact, Wait(260), Snap("lado"), Quit],
        // resgate: relógio estourado (rodar com CQB_CLOCK=5) e, na segunda vez, captor derrubado
        "rescue" => vec![Wait(120), Scn(6), Wait(150), Confirm, Wait(360), Snap("r1"), Shot("rs-play"), Wait(480), Snap("r2"), Shot("rs-lost"), Confirm, Wait(150), Confirm, Wait(30),
            Who(0, 3.0), Wait(20), Ads(40), Fire, Wait(14), Fire, Wait(14), Fire, Wait(300), Snap("r3"), Shot("rs-won"), Quit],
        // refém como escudo: de frente para uma ameaça com escudo
        "shield" => vec![Wait(120), Scn(0), Wait(150), Confirm, Wait(30), Who(1, 3.2), Wait(60), Ads(60), Wait(60), Shot("sh-front"), Quit],
        // ameaças que circulam: derruba três e espera a que vem por trás (rodar com CQB_NOREACT=1 CQB_HUNTLOG=1)
        "rear" => vec![Wait(120), Scn(3), Wait(150), Confirm, Wait(30),
            Near(false, 0, 3.0), Wait(20), Ads(40), Fire, Wait(14), Fire, Wait(14), Fire, Wait(30), Ads(0),
            Near(false, 0, 3.0), Wait(20), Ads(40), Fire, Wait(14), Fire, Wait(14), Fire, Wait(30), Ads(0),
            Near(false, 0, 3.0), Wait(20), Ads(40), Fire, Wait(14), Fire, Wait(14), Fire, Wait(30), Ads(0), Snap("3 abatidas"),
            Wait(1200), Snap("depois"), Shot("rear-1"), Quit],
        // defesa da casa: operador parado onde começa, sem atirar (rodar com CQB_NODEATH=1 CQB_HUNTLOG=1)
        "def" => vec![Wait(120), Scn(7), Wait(150), Confirm, Wait(1500), Snap("onda1"), Shot("def-1"), Wait(600), Snap("fim"), Shot("def-2"), Quit],
        // alvos que mudam de ideia: uma ameaça que se rende ao ser vista e uma de mãos para o alto que saca (rodar com CQB_NODEATH=1)
        "mind" => vec![Wait(120), Scn(5), Wait(150), Confirm, Wait(30), Who(3, 3.2), Wait(20), Shot("mi-armed"), Wait(160), Snap("rendeu"), Shot("mi-yielded"),
            Who(2, 3.2), Wait(60), Shot("mi-fake"), Wait(500), Snap("sacou"), Shot("mi-drew"), Quit],
        // defesa da casa com o operador automático parado numa posição (CQB_POS) atirando no que aparece (CQB_BOTLAG = quadros até o 1º tiro)
        "hold" => vec![Wait(120), Scn(7), Wait(150), Confirm, Wait(20), Weapon(1), Wait(260), TpEnv, Ads(20), Auto(true), Hold(26000), Wait(60), Snap("fim"), Shot("hold-end"), Screen(Mode::Ranking), Wait(40), Shot("hold-rank"), Quit],
        // percurso livre no mapa do corredor (confere que o mapa novo funciona fora do cenário)
        "hallfree" => vec![Wait(120), Map(4), Wait(120), Start(true), Wait(240), Snap("corredor"), Shot("hallfree"), Quit],
        // mech: de frente (os tiros batem na placa), depois por trás até derrubar (rodar com CQB_NODEATH=1)
        "walker" => { let mut v = vec![Wait(120), Scn(8), Wait(150), Shot("wk-card"), Confirm, Wait(30), Weapon(1), Wait(260), Who(5, 4.5), Wait(60), Shot("wk-front"), Ads(20), Fire, Wait(20), Fire, Wait(20), Fire, Wait(60), Snap("frente"), Shot("wk-aim"), Who(5, -3.5), Wait(4)];
            // por trás: mira no mech a cada tiro (ele anda)
            for _ in 0..16 { v.extend([Aim(false, 4), Fire, Wait(16)]); }
            v.extend([Wait(90), Snap("atras"), Shot("wk-fall"), Wait(400), Snap("fim"), Shot("wk-end"), Quit]); v }
        // fase very hard: parado de frente para o mech até a mira travar e o canhão encerrar a partida
        "lockon" => vec![Wait(120), Scn(8), Wait(150), Confirm, Wait(30), Who(5, 6.0), Wait(36), Shot("lk-lock"), Wait(30), Shot("lk-lock2"), Wait(300), Snap("fim"), Shot("lk-down"), Quit],
        // captura limpa do mapa do corredor visto de cima, para a página (CQB_NOFOW=1 CQB_NOREACT=1 CQB_ZOOM=24)
        "sitehall" => vec![Wait(120), Map(4), Wait(120), Start(true), Wait(30), Tp(10.0, 9.4, 0.0, 0.0), ViewTo(View::Bird), Wait(1300), Shot("site-hall"), Quit],
        // alerta de área no percurso livre: visto por uma ameaça, as do cômodo saem atrás do operador (CQB_NODEATH=1 CQB_HUNTLOG=1)
        // inimigos andando com o fuzil, vistos de cima: alerta a área, afasta o operador e fotografa quem vem atrás dele
        "riflewalk" => vec![Wait(120), Map(1), Wait(120), Start(true), Wait(120), Near(false, 2, 4.0), Wait(120), Tp(2.0, 12.8, 0.0, 0.0), ViewTo(View::Bird), Wait(260), Shot("rw1"), Wait(110), Shot("rw2"),
            Wait(110), Shot("rw3"), Wait(110), Shot("rw4"), Quit],
        // troca de andar dos invasores: do corredor de cima, fotografa um deles saindo da caixa de escada; depois os degraus do
        // térreo, a escada de mão do quintal e o alçapão do terraço
        "climb" => vec![Wait(120), Scn(7), Wait(150), Confirm, Wait(20), Tp(6.0, 0.7, 1.5708, -0.22), Wait(20), Shot("cl-top"), WaitClimb, Wait(8), Shot("cl-a"), Wait(22), Shot("cl-b"), Wait(30), Shot("cl-c"), Wait(40), Shot("cl-d"),
            Tp(1.7, 0.7, 1.5708, -0.5), Wait(25), Shot("cl-down"), Tp(3.4, 9.9, 1.037, -0.05), Wait(25), Shot("cl-niche"), Tp(1.6, 8.55, 1.5708, 0.1), Wait(25), Shot("cl-up"),
            Tp(2.6, 15.6, 0.611, 0.1), Wait(25), Shot("cl-ladder"), Tp(3.2, 6.5, 1.5708, -0.1), Wait(25), Shot("cl-terrace"), Quit],
        // o operador desce a escada andando (do corredor de cima até o térreo) e sobe de volta
        "stairs" => vec![Wait(120), Scn(7), Wait(150), Confirm, Wait(20), Tp(4.4, 0.7, 1.5708, -0.45), Wait(20), Shot("st-1"), Go(2.2, 0.7, None), Shot("st-1b"), Go(0.3, 0.7, None), Wait(70), Snap("desceu"),
            Tp(0.45, 7.45, 3.1416, -0.35), Wait(20), Shot("st-2"), Go(0.6, 8.5, None), Go(3.0, 8.55, None), Tp(3.4, 9.9, 1.037, -0.05), Wait(20), Shot("st-3"),
            Tp(2.6, 8.55, 1.5708, 0.0), Go(0.6, 8.55, None), Go(0.6, 7.4, None), Go(1.0, 7.35, None), Wait(70), Snap("subiu"), Tp(0.8, 0.7, -1.5708, 0.25), Wait(20), Shot("st-4"), Quit],
        "alarm" => vec![Wait(120), Map(1), Wait(120), Start(true), Wait(120), Near(false, 2, 4.0), Wait(120), Tp(2.0, 12.8, 0.0, 0.0), Wait(1500), Snap("alarme"), Shot("alarm"), Quit],
        // granada: parado fora da vista de ameaças alertadas (um tiro avisa o cômodo) até alguém lançar (CQB_NODEATH=1)
        "nade" => vec![Wait(120), Map(1), Wait(120), Start(true), Wait(120), Near(false, 2, 4.0), Wait(100), Tp(4.6, 7.0, 0.0, 0.0), WaitNade, Wait(60), Shot("nade-air"), Wait(70), Shot("nade-fp"), ViewTo(View::Bird), Wait(25), Shot("nade-top"), Wait(400), Snap("n1"), Shot("nade-after"), Quit],
        "scns" => vec![Wait(120),
            Scn(0), Wait(150), Shot("s1-card"), Confirm, Wait(120), Snap("s1"), Shot("s1-play"),
            Scn(1), Wait(150), Shot("s2-card"), Confirm, Wait(120), Snap("s2"), Shot("s2-play"),
            Scn(2), Wait(150), Confirm, Wait(120), Snap("s3"), Shot("s3-play"),
            Scn(3), Wait(150), Confirm, Wait(120), Snap("s4"), Shot("s4-play"),
            Scn(4), Wait(150), Confirm, Wait(120), Snap("s5"), Shot("s5-play"),
            Scn(5), Wait(150), Confirm, Wait(120), Snap("s6"), Shot("s6-play"),
            Scn(6), Wait(150), Confirm, Wait(120), Snap("s7"), Shot("s7-play"),
            Scn(7), Wait(150), Shot("s8-card"), Confirm, Wait(120), Snap("s8"), Shot("s8-play"), ViewTo(View::Bird), Wait(60), Shot("s8-top"),
            Screen(Mode::Scenarios), Wait(40), Shot("scn-screen"), Screen(Mode::Ranking), Wait(40), Shot("rank-screen"), Quit],
        // pausa no meio da partida, volta ao menu e começa outra
        "pmenu" => vec![Wait(60), Start(true), Wait(240), Tac(true), ViewTo(View::Bird), Go(3.0, 12.0, None), Wait(60), Pause, Wait(40), Shot("pm-pause"), Menu, Wait(90), Snap("menu"), Shot("pm-menu"), Start(true), Wait(200), Snap("de novo"), Shot("pm-again"), Quit],
        // dificuldade: menu, EASY (papelão, tiro só simulado, sem fim de partida) e HARD (tiro mais rápido, operador abatido)
        "diff" => vec![Wait(200), Shot("df-menu"), Diff(0), Wait(90), Shot("df-menu-easy"), Start(true), Wait(240), Near(false, 1, 3.0), Wait(120), Shot("df-easy-threat"), Wait(500), Snap("easy"), Near(true, 0, 2.6), Wait(90), Shot("df-easy-hostage"),
            Diff(2), Wait(30), Start(true), Wait(240), Near(false, 1, 3.0), Wait(100), Shot("df-hard"), Wait(500), Snap("hard"), Shot("df-hard-down"), Quit],
        // operador parado na frente de uma ameaça até ser abatido; depois tenta de novo
        "down" => vec![Wait(60), Start(true), Wait(240), Near(false, 1, 3.0), Wait(150), Snap("d1"), Shot("d1"), Wait(100), Snap("d2"), Shot("d2"), Wait(90), Snap("d3"), Shot("d3"), Wait(120), Snap("d4"), Shot("d4"), Wait(600), Snap("d5"), Shot("d5"), Confirm, Wait(240), Snap("d6"), Shot("d6"), Quit],
        // operador parado atrás de uma ameaça: ela percebe, vira e atira
        "back" => vec![Wait(60), Start(true), Wait(240), Near(false, 1, -3.0), Wait(60), Snap("b1"), Shot("b1"), Wait(120), Snap("b2"), Shot("b2"), Wait(90), Snap("b3"), Shot("b3"), Wait(90), Snap("b4"), Shot("b4"), Wait(120), Snap("b5"), Shot("b5"), Quit],
        // o mesmo no tutorial: abatido na lição 2, volta ao começo dela
        "down2" => vec![Wait(60), Start(false), Wait(40), Confirm, Wait(80), Near(false, 0, 3.0), Wait(900), Snap("t1"), Shot("t1"), Confirm, Wait(120), Snap("t2"), Shot("t2"), Confirm, Wait(120), Snap("t3"), Shot("t3"), Quit],
        "menu" => vec![Wait(200), Shot("menu1"), Wait(400), Shot("menu2"), Wait(400), Shot("menu3"), Quit],
        "demo" => vec![Wait(60), Start(false), Wait(20), DemoOf(1), WaitDemo, DemoOf(2), WaitDemo, DemoOf(3), WaitDemo, DemoOf(4), WaitDemo, DemoOf(5), WaitDemo, Wait(20), Quit],
        _ => {
            // percurso completo: a lição 1 testa os controles; as demais seguem o mesmo traçado ideal que o jogador vê no chão
            let mut v = vec![
                Start(false), Wait(40),
                Look(110), ViewTo(View::Bird), Go(1.6, 15.6, None), Go(4.6, 15.6, None), Go(3.0, 14.9, None), Ads(50), Ads(0), Lean(1.0, 50), Lean(0.0, 0), Crouch(60), Crouch(0),
                Shoot("P0"), Reload, Wait(150), Flash(false), Wait(8), Flash(true), Tac(true), ViewTo(View::Fp), Wait(5), ViewTo(View::Bird), Wait(10), Snap("L0"), Wait(160),
            ];
            for (l, name) in [(1usize, "L1"), (2, "L2"), (3, "L3"), (4, "L4"), (5, "L5")] {
                v.extend(steps_cmds(l, false));
                match l {
                    1 => v.extend([Wait(30), Snap(name), WaitReplay, Shot("21-replay")]),
                    5 => v.push(Snap(name)),
                    _ => v.extend([Wait(20), Snap(name), Wait(160)]),
                }
            }
            v.extend([Wait(200), Snap("fim"), Shot("20-avaliacao"), Quit]);
            v
        }
    };
    bot.auto_card = script == "run";
    bot.queue = q.into();
}

pub fn bot_drive(mut bot: ResMut<Bot>, mut c: ResMut<Controls>, mut p: ResMut<Player>, mut game: ResMut<Game>, targets: Res<Targets>, tac: Res<Tac>, level: Res<Level>,
    mut commands: Commands, mut exit: EventWriter<AppExit>, mut reset: EventWriter<ResetGame>, real: Res<Time<Real>>, mut mapreq: ResMut<MapReq>) {
    let demo = game.demo;
    if !bot.active && !demo { return; }
    let rd = real.delta_secs();
    if rd > 0.0 { bot.fps = if bot.fps == 0.0 { 1.0 / rd } else { bot.fps * 0.95 + (1.0 / rd) * 0.05 }; }
    // na demonstração o jogador só pode trocar de vista, dar zoom ou sair; o resto vem do roteiro
    let kb = c.clone();
    *c = Controls { ads: bot.hold_ads, lean: bot.hold_lean, crouch_hold: bot.hold_crouch, pace: if demo { 0.72 } else { bot.pace }, ..default() };
    if demo && !bot.active { c.confirm = kb.confirm; c.pause = kb.pause; c.view = kb.view; c.zoom = kb.zoom; }
    // no teste o operador vira de uma vez; na demonstração ele gira como uma pessoa
    let turn = 4.0 * rd.min(0.05);
    let face = |p: &mut Player, x: f32, z: f32| -> bool {
        let want = (-(x - p.pos.x)).atan2(-(z - p.pos.y));
        if !demo { p.yaw = want; return true; }
        let d = ang_diff(want, p.yaw);
        p.yaw += d.clamp(-turn, turn);
        if p.yaw > std::f32::consts::PI { p.yaw -= std::f32::consts::TAU } else if p.yaw < -std::f32::consts::PI { p.yaw += std::f32::consts::TAU }
        d.abs() < 0.04
    };
    if demo && bot.active {
        bot.demo_frames += 1;
        if bot.demo_frames == 200 || bot.demo_frames == 520 { commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("{}/4{}-demo-{}.png", bot.out, game.lesson, bot.demo_frames))); }
    }
    // no percurso completo, os cartões de lição são dispensados sozinhos
    if !demo && bot.auto_card && matches!(game.mode, Mode::Card | Mode::Replay) {
        bot.card_n += 1;
        // se o roteiro quer capturar o replay, descarta o que ficou pendente antes dele
        if game.mode == Mode::Replay && bot.queue.iter().take(6).any(|c| matches!(c, Cmd::WaitReplay)) {
            while !matches!(bot.queue.front(), Some(Cmd::WaitReplay)) { bot.queue.pop_front(); }
        }
        let shot = matches!(bot.queue.front(), Some(Cmd::Shot(_)) | Some(Cmd::WaitReplay));
        if !shot { if bot.card_n > if game.mode == Mode::Replay { 150 } else { 25 } { c.confirm = true; bot.card_n = 0; } return; }
    } else { bot.card_n = 0; }
    // Granada por perto: o operador automático de guarda larga tudo e corre pelo caminho de fuga (CQB_FLEE="x,z;x,z;..."),
    // fica lá até ela estourar e depois volta pelo mesmo caminho
    if game.mode == Mode::Play && bot.home.is_some() && !bot.flee.is_empty() {
        let danger = game.nade.map_or(false, |(g, _)| g.distance(p.pos) < 6.5) || (game.nade.is_some() && bot.flee_i > 0);
        if danger {
            if bot.flee_i < bot.flee.len() {
                let wp = bot.flee[bot.flee_i];
                if p.pos.distance(wp) < 0.3 { bot.flee_i += 1; } else { c.world_move = Some((wp - p.pos).normalize()); c.fast = true; c.ads = false; }
                return;
            }
            // já no abrigo: continua atirando no que aparecer enquanto espera o estouro
        }
    }
    // Ameaça à vista tem prioridade sobre o roteiro: como as posições são sorteadas, o operador automático (teste e
    // demonstração) para e neutraliza o que aparecer no setor, desde que não haja um refém na linha de tiro.
    if game.mode == Mode::Play && (demo || bot.auto_card) {
        let eye = p.eye2();
        let clear = |t: &TargetState| -> bool {
            let (tp, d) = (Vec2::new(t.def.x, t.def.z), Vec2::new(t.def.x, t.def.z).distance(eye));
            let dir = (tp - eye) / d.max(0.01);
            !targets.list.iter().any(|c| c.alive && !c.off && c.def.kind == TKind::Civil && { let v = Vec2::new(c.def.x, c.def.z) - eye; let along = v.dot(dir); along > 0.0 && along < d + 1.5 && v.perp_dot(dir).abs() < 0.4 })
        };
        // também reage a uma ameaça que já tem linha de tiro para ele (canto próximo ao entrar), virando-se para ela
        let pick = targets.list.iter().filter(|t| t.alive && !t.off && !t.def.passive && t.def.kind == TKind::Threat && (t.vis || t.react > 0.2) && clear(t))
            .min_by(|a, b| Vec2::new(a.def.x, a.def.z).distance(eye).partial_cmp(&Vec2::new(b.def.x, b.def.z).distance(eye)).unwrap()).map(|t| (t.def.x, t.def.z));
        if let Some((tx, tz)) = pick {
            if p.mag == 0 && !p.reloading { c.reload = true; }
            let aligned = face(&mut p, tx, tz);
            let dd = Vec2::new(tx, tz).distance(p.pos).max(0.5);
            p.pitch = ((1.18 - p.eye.y) / dd).atan();
            bot.engage_n += 1;
            // CQB_BOTLAG: milissegundos que o operador automático leva para dar o primeiro tiro numa ameaça que apareceu
            // (imita o tempo de reação de uma pessoa); sem a variável, vale o ritmo antigo, contado em quadros
            match std::env::var("CQB_BOTLAG").ok().and_then(|v| v.parse::<f32>().ok()) {
                Some(ms) => {
                    if bot.engage_n == 1 { bot.engage_t = 0.0; bot.fire_t = 9.0; }
                    bot.engage_t += rd.min(0.05); bot.fire_t += rd.min(0.05);
                    if aligned && bot.engage_t >= ms / 1000.0 && bot.fire_t >= 0.13 { c.fire = true; bot.fire_t = 0.0; }
                }
                None => if aligned && bot.engage_n % 12 == 3 { c.fire = true; },
            }
            // no vão de uma porta não se para: continua andando para o próximo ponto enquanto atira
            let in_door = crate::map::doorways().iter().any(|w| p.pos.x >= w.x1 && p.pos.x <= w.x2 && p.pos.y >= w.z1 && p.pos.y <= w.z2);
            if in_door {
                let q = if demo { bot.demo_q.front() } else { bot.queue.front() };
                if let Some(Cmd::Go(x, z, _)) = q { let d = Vec2::new(*x, *z) - p.pos; if d.length() > 0.15 { c.world_move = Some(d.normalize()); } }
            }
            return;
        }
        bot.engage_n = 0;
    }
    let Some(cmd) = (if demo { bot.demo_q.front().cloned() } else { bot.queue.front().cloned() }) else { return };
    bot.n += 1;
    let mut done = false;
    match cmd {
        Cmd::Wait(n) => done = bot.n >= n,
        Cmd::Start(free) => { reset.write(ResetGame { free, mission2: false, scn: None }); done = true; }
        Cmd::Confirm => { c.confirm = true; done = true; }
        Cmd::Tp(x, z, yaw, pitch) => { p.pos = Vec2::new(x, z); p.vel = Vec2::ZERO; p.yaw = yaw; p.pitch = pitch; done = true; }
        Cmd::Go(x, z, f) => {
            let d = Vec2::new(x, z) - p.pos;
            if d.length() < 0.15 || bot.n > 900 { if bot.n > 900 { println!("BOT tempo esgotado indo a ({x},{z}); parou em ({:.2},{:.2})", p.pos.x, p.pos.y); } done = true; }
            else { c.world_move = Some(d.normalize()); if let Some((fx, fz)) = f { face(&mut p, fx, fz); } }
        }
        Cmd::Face(x, z) => { done = face(&mut p, x, z) || bot.n > 240; }
        Cmd::Shoot(id) => {
            let t = targets.get(id);
            if p.mag == 0 && !p.reloading { c.reload = true; }
            if t.def.passive || !(demo || bot.auto_card) {
                // alvo de treino (fixo) ou roteiros de captura: mira e atira até cair
                if !t.alive || bot.n > 900 { if t.alive { println!("BOT não derrubou {id} (visto={})", t.seen); } done = true; }
                else {
                    let aligned = face(&mut p, t.def.x, t.def.z);
                    let dd = Vec2::new(t.def.x, t.def.z).distance(p.pos).max(0.5);
                    p.pitch = ((1.18 - p.eye.y) / dd).atan();
                    if aligned && (t.vis || bot.n > 90) && bot.n % 22 == 5 { c.fire = true; }
                }
            } else {
                // posição sorteada: quem atira é a regra acima, quando a ameaça aparecer; aqui só se espera um instante
                done = !t.alive || bot.n > 40;
            }
        }
        Cmd::Spin => { if bot.n % 2 == 0 { p.yaw += std::f32::consts::PI / 36.0; if p.yaw > std::f32::consts::PI { p.yaw -= std::f32::consts::TAU; } } done = bot.n >= 144; }
        Cmd::Interact => { c.interact = true; done = true; }
        Cmd::Reload => { c.reload = true; done = true; }
        Cmd::Fire => { c.fire = true; done = true; }
        Cmd::Look(n) => { c.look = Vec2::new(14.0, 0.0); done = bot.n >= n; }
        Cmd::Ads(n) => { bot.hold_ads = n > 0; c.ads = bot.hold_ads; done = bot.n >= n; }
        Cmd::Lean(v, n) => { bot.hold_lean = v; c.lean = v; done = bot.n >= n; }
        Cmd::Crouch(n) => { bot.hold_crouch = n > 0; c.crouch_hold = bot.hold_crouch; done = bot.n >= n; }
        Cmd::Tac(b) => { if game.tac != b { c.tac = true; } done = true; }
        Cmd::ViewTo(v) => { if game.view != v { c.view = true; } done = true; }
        Cmd::Flash(b) => { if game.flash != b { c.flash = true; } done = true; }
        Cmd::Shot(name) => {
            if bot.n == 1 { commands.spawn(Screenshot::primary_window()).observe(save_to_disk(format!("{}/{}.png", bot.out, name))); }
            done = bot.n >= 8;
        }
        Cmd::Snap(name) => {
            let pct: Vec<String> = crate::map::zones().iter().enumerate().map(|(i, z)| format!("{}={}", z.id, (tac.pct[i] * 100.0).round() as i32)).collect();
            let prim = format!("{:?} entr={:?}", (0..5).map(|i| tac.pie_deg(i) as i32).collect::<Vec<_>>(), game.entry_pie.map(|e| e.map(|v| v as i32)));
println!("BOT MOVE {name}: mudaram={:?} andando={:?} limpos={:#b} de {}", targets.list.iter().filter(|t| t.moved > 0).count(), targets.list.iter().filter(|t| t.moving).count(), game.cleared, game.rooms);
                        println!("BOT ARMA {name}: arma={} {} carregador={} reserva={} recarregando={} vazia={} troca={:?}", p.weapon, p.def().hud, p.mag, p.reserve, p.reloading, p.reload_empty, p.switch_to);
            println!("BOT POS {name}: ({:.2},{:.2}) portas={:?}", p.pos.x, p.pos.y, level.doors.iter().map(|d| (d.open * 100.0).round() as i32).collect::<Vec<_>>());
            println!("BOT PTS {name}: rota={:?} derrubadas={:?} base={:.0} chances={:.0} badges={:#012b} abates={:?}", game.route_score().map(|v| v.round()), game.kill_score().map(|v| v.round()), game.base_score(), game.odds, game.badges, game.kills.iter().map(|k| (k.angle.round() as i32, k.expo_cm.round() as i32, k.pts.round() as i32)).collect::<Vec<_>>());
            println!("BOT ANG {name}: setor={:.0} agora={:?} abertas={:?} foco={:?} lean={:.2} olho=({:.2},{:.2}) ads={:.2} dica={:?} bloq={}", tac.hfov.to_degrees(), tac.pie_now.iter().map(|v| v.map(|(a, b)| (b - a).round() as i32)).collect::<Vec<_>>(), (0..5).map(|i| tac.pie_deg(i) as i32).collect::<Vec<_>>(), tac.pie_active, p.lean, p.eye.x, p.eye.z, p.g_ads, tac.lean_hint.map(|h| (h.0, h.1.round())), p.lean_blocked);
            println!("BOT {name}: qps={:.0} modo={:?} lição={} obj={:?} {} entrada={:?} funil={:?} atingido={} quina={} corpo={:.0}cm expo={:.0}° avisos={:?}",
                bot.fps, game.mode, game.lesson, game.obj, pct.join(" "), game.entry.map(|e| e.map(|v| (v * 100.0).round() as i32)), game.dwell.map(|d| (d * 10.0).round() / 10.0), game.stats.hits_taken, prim, tac.body_cm, tac.expo_deg, game.warns);
            done = true;
        }
        Cmd::Quit => { exit.write(AppExit::Success); done = true; }
        Cmd::Mission2 => { reset.write(ResetGame { free: true, mission2: true, scn: None }); done = true; }
        Cmd::Pause => { c.pause = true; done = true; }
        Cmd::Credits => { game.mode = crate::state::Mode::Credits; game.ui_dirty = true; done = true; }
        Cmd::WaitReplay => { done = (game.mode == Mode::Replay && game.replay_t > 3.5) || bot.n > 900; }
        Cmd::Say(m) => { game.coach = Some((m.to_string(), 3, 11.0)); game.ui_dirty = true; done = true; }
        Cmd::OpenDoor(i) => { if level.doors[i].open < 0.5 && !level.doors[i].opening { c.interact = true; } done = true; }
        Cmd::DemoOf(l) => { game.lesson = l; game.obj = vec![0; crate::lessons::LESSONS[l].objs.len()]; game.mode = Mode::Card; game.demo_req = true; game.ui_dirty = true; done = true; }
        Cmd::Map(i) => { mapreq.0 = Some(i); done = true; }
        // o mesmo que o botão do cenário na tela SCENARIOS
        Cmd::Scn(i) => {
            let sc = &crate::scenario::SCENARIOS[i];
            set_force_figs(sc.figs);
            if crate::map::map_index() != sc.map || figs_built() != want_figs() { mapreq.0 = Some(sc.map); }
            reset.write(ResetGame { free: true, mission2: false, scn: Some(i) });
            done = true;
        }
        Cmd::Screen(m) => { game.mode = m; game.ui_dirty = true; done = true; }
        Cmd::Auto(on) => { bot.auto_card = on; done = true; }
        // espera uma granada entrar em cena
        Cmd::WaitNade => { done = game.nade.is_some() || bot.n > 4000; }
        Cmd::WaitClimb => { done = targets.list.iter().any(|t| t.alive && !t.off && t.rise > 0.0) || bot.n > 4000; }
        // fica parado por n quadros; recarrega só quando não há invasor de pé (entre as ondas)
        Cmd::Hold(n) => {
            if !targets.list.iter().any(|t| t.hunter && t.alive && !t.off) && p.mag < p.def().mag && !p.reloading && !p.switching() { c.reload = true; }
            // depois de fugir de uma granada, volta para a posição de guarda pelo mesmo caminho
            if let (Some((home, look)), true) = (bot.home, bot.flee_i > 0 && game.nade.is_none()) {
                let target = if bot.flee_i >= 2 { bot.flee[bot.flee_i - 2] } else { home };
                if p.pos.distance(target) < 0.3 {
                    bot.flee_i -= 1;
                    if bot.flee_i == 0 { p.yaw = (-(look.x - p.pos.x)).atan2(-(look.y - p.pos.y)); p.pitch = -0.04; }
                } else { c.world_move = Some((target - p.pos).normalize()); c.fast = true; c.ads = false; }
            }
            done = bot.n >= n || game.mode != Mode::Play;
        }
        // posição e ponto para onde olhar vindos de CQB_POS="x,z,olhar_x,olhar_z"
        Cmd::TpEnv => {
            let v: Vec<f32> = std::env::var("CQB_POS").unwrap_or_default().split(',').filter_map(|s| s.trim().parse().ok()).collect();
            if v.len() == 4 {
                p.pos = Vec2::new(v[0], v[1]); p.vel = Vec2::ZERO; p.yaw = (-(v[2] - v[0])).atan2(-(v[3] - v[1])); p.pitch = -0.04;
                bot.home = Some((Vec2::new(v[0], v[1]), Vec2::new(v[2], v[3])));
                bot.flee = std::env::var("CQB_FLEE").unwrap_or_default().split(';').filter_map(|q| { let w: Vec<f32> = q.split(',').filter_map(|s| s.trim().parse().ok()).collect(); if w.len() == 2 { Some(Vec2::new(w[0], w[1])) } else { None } }).collect();
                bot.flee_i = 0;
            }
            done = true;
        }
        // vai para a frente de um alvo escolhido pelo papel dele no cenário: 0 captor, 1 ameaça com escudo, 2 falso rendido, 3 quem vai se render, 4 caçador
        Cmd::Who(w, dist) => {
            let pick = |t: &&TargetState| t.alive && !t.off && t.def.kind == TKind::Threat && match w { 0 => t.captor, 1 => t.shield, 2 => t.mind == MIND_FAKE, 3 => t.mind == MIND_WILL_YIELD, 5 => crate::map::is_mech(&t.def), _ => t.hunter };
            if let Some(t) = targets.list.iter().find(pick) {
                let tp = Vec2::new(t.def.x, t.def.z);
                let (sg, dist) = (dist.signum(), dist.abs());
                let d = level.cast_ray(tp.x, tp.y, t.fwd.x * sg, t.fwd.y * sg, dist + 0.4, 1.0);
                let pos = tp + t.fwd * sg * (d - 0.4).clamp(0.8, dist);
                p.pos = pos; p.vel = Vec2::ZERO; p.yaw = (-(tp.x - pos.x)).atan2(-(tp.y - pos.y)); p.pitch = -0.08;
                println!("BOT ALVO {} em ({:.1},{:.1}) estado={} operador em ({:.1},{:.1})", t.def.id, tp.x, tp.y, t.mind, pos.x, pos.y);
            } else { println!("BOT ALVO nenhum com papel {}", w); }
            done = true;
        }
        // vira para o n-ésimo refém (true) ou a n-ésima ameaça (false), sem sair do lugar
        Cmd::Aim(civil, n) => {
            let want = if civil { TKind::Civil } else { TKind::Threat };
            if let Some(t) = targets.list.iter().filter(|t| t.def.kind == want && !t.def.passive && t.alive && !t.off).nth(n) { p.yaw = (-(t.def.x - p.pos.x)).atan2(-(t.def.z - p.pos.y)); p.pitch = -0.05; }
            done = true;
        }
        // o mesmo que o botão RETURN TO MENU
        Cmd::Menu => { *p = crate::ui::menu_player(p.weapon); game.mode = Mode::Menu; game.view = View::Fp; game.tac = false; game.toast = None; game.coach = None; game.ui_dirty = true; done = true; }
        Cmd::Diff(d) => { let was = difficulty(); set_difficulty(d); if (was == 0) != (d == 0) { mapreq.0 = Some(crate::map::map_index()); } done = true; }
        Cmd::Weapon(i) => { c.weapon = Some(i); done = true; }
        Cmd::Near(civil, n, dist) => {
            let want = if civil { TKind::Civil } else { TKind::Threat };
            if let Some(t) = targets.list.iter().filter(|t| t.def.kind == want && !t.def.passive && t.alive).nth(n) {
                let tp = Vec2::new(t.def.x, t.def.z);
                // recua na direção para onde o alvo olha, até onde houver chão livre
                // distância negativa: vai para trás do alvo, em vez de para a frente
                let (sg, dist) = (dist.signum(), dist.abs());
                let d = level.cast_ray(tp.x, tp.y, t.fwd.x * sg, t.fwd.y * sg, dist + 0.4, 1.0);
                let pos = tp + t.fwd * sg * (d - 0.4).clamp(0.8, dist);
                p.pos = pos; p.vel = Vec2::ZERO; p.yaw = (-(tp.x - pos.x)).atan2(-(tp.y - pos.y)); p.pitch = -0.08;
            }
            done = true;
        }
        Cmd::WaitDemo => { done = bot.n > 3 && !game.demo && !game.demo_req; }
    }
    if done { if demo { bot.demo_q.pop_front(); } else { bot.queue.pop_front(); } bot.n = 0; }
}
