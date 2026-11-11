//! Som: gravações reais (CC0) tratadas na abertura do jogo. Cada som ganha a reverberação do tipo de ambiente
//! (sala ou corredor) e uma versão abafada, usada quando há parede entre a fonte e o operador.
use crate::map::*;
use crate::state::{Player, Sfx};
use bevy::audio::{PlaybackMode, SpatialScale, Volume};
use bevy::prelude::*;
use std::f32::consts::TAU;

const SR: f32 = 44_100.0;

/// Lê um WAV mono de 16 bits e devolve as amostras.
fn wav_samples(bytes: &[u8]) -> Vec<f32> {
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let len = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
        if &bytes[i..i + 4] == b"data" {
            return bytes[i + 8..(i + 8 + len).min(bytes.len())].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect();
        }
        i += 8 + len + (len & 1);
    }
    Vec::new()
}
macro_rules! clip { ($n:literal) => { wav_samples(include_bytes!(concat!("../assets/sfx/", $n, ".wav"))) }; }

fn pad(x: &[f32], secs: f32) -> Vec<f32> { let mut v = x.to_vec(); v.resize(x.len() + (secs * SR) as usize, 0.0); v }
fn mix(a: &[f32], b: &[f32], gb: f32) -> Vec<f32> { let n = a.len().max(b.len()); (0..n).map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0) * gb).collect() }
fn gain(x: &[f32], g: f32) -> Vec<f32> { x.iter().map(|v| v * g).collect() }
/// Passa-baixas de dois polos: o que sobra de um som depois de atravessar uma parede.
fn lowpass(x: &[f32], fc: f32) -> Vec<f32> {
    let a = 1.0 - (-TAU * fc / SR).exp();
    let (mut l1, mut l2) = (0.0f32, 0.0f32);
    x.iter().map(|v| { l1 += (v - l1) * a; l2 += (l1 - l2) * a; l2 }).collect()
}
fn tone(out: &mut [f32], at: f32, dur: f32, f0: f32, f1: f32, g: f32, square: bool) {
    let (s, n) = ((at * SR) as usize, (dur * SR) as usize);
    let mut ph = 0.0f32;
    for i in 0..n {
        if s + i >= out.len() { break; }
        let t = i as f32 / n as f32;
        ph += TAU * f0 * (f1 / f0).powf(t) / SR;
        let v = if square { ph.sin().signum() * 0.6 } else { ph.sin() };
        out[s + i] += v * (1.0 - t).powf(1.8) * (i as f32 / 80.0).min(1.0) * g;
    }
}
/// Reverberação de Schroeder: quatro filtros pente amortecidos e dois passa-tudo.
fn reverb(dry: &[f32], wet: f32, rt: f32) -> Vec<f32> {
    let n = dry.len();
    let mut acc = vec![0.0f32; n];
    for d_ms in [29.7f32, 37.1, 41.1, 43.7] {
        let d = (d_ms / 1000.0 * SR) as usize;
        let g = 10f32.powf(-3.0 * (d_ms / 1000.0) / rt);
        let mut line = vec![0.0f32; d];
        let mut lp = 0.0;
        for i in 0..n {
            let y = line[i % d];
            lp += (y - lp) * 0.45;
            line[i % d] = dry[i] + lp * g;
            acc[i] += y * 0.25;
        }
    }
    for (d_ms, g) in [(5.0f32, 0.7f32), (1.7, 0.7)] {
        let d = (d_ms / 1000.0 * SR) as usize;
        let mut line = vec![0.0f32; d];
        for i in 0..n {
            let delayed = line[i % d];
            let x = acc[i];
            let y = -g * x + delayed;
            line[i % d] = x + g * y;
            acc[i] = y;
        }
    }
    (0..n).map(|i| dry[i] + acc[i] * wet).collect()
}
fn wav(samples: &[f32], peak: f32) -> AudioSource {
    let m = samples.iter().fold(0.0f32, |a, b| a.max(b.abs())).max(1e-6);
    let k = peak / m;
    let mut b: Vec<u8> = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    b.extend_from_slice(b"RIFF"); b.extend_from_slice(&(36 + data).to_le_bytes()); b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); b.extend_from_slice(&1u16.to_le_bytes()); b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&(SR as u32).to_le_bytes()); b.extend_from_slice(&(SR as u32 * 2).to_le_bytes()); b.extend_from_slice(&2u16.to_le_bytes()); b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data"); b.extend_from_slice(&data.to_le_bytes());
    for s in samples { b.extend_from_slice(&(((s * k).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()); }
    AudioSource { bytes: b.into() }
}

type H = Handle<AudioSource>;
/// Par (som direto, som abafado por parede).
type Pair = [H; 2];
#[derive(Resource)]
pub struct Sounds {
    shot_room: H, shot_corr: H, enemy: Pair, dry: H, click: H, reload: H, steps: Vec<H>, casing: Vec<H>, impact: Vec<Pair>, metal: Pair,
    paper: Pair, thwack: Pair, fall: Pair, door: Pair, rustle: H, good: H, bad: H, hurt: H, ui: H, n: u32,
}

fn add(a: &mut Assets<AudioSource>, s: &[f32], p: f32) -> H { a.add(wav(s, p)) }
fn pair(a: &mut Assets<AudioSource>, s: &[f32], p: f32) -> Pair { [a.add(wav(s, p)), a.add(wav(&lowpass(s, 520.0), p * 0.6))] }

pub fn setup_audio(mut commands: Commands, mut a: ResMut<Assets<AudioSource>>) {
    // disparo: gravação de .45 de perto, reforçada no grave e com a cauda do ambiente
    let mut base = pad(&clip!("tiro_1"), 1.5);
    tone(&mut base, 0.0, 0.17, 150.0, 44.0, 0.55, false);
    let shot_room = reverb(&base, 0.5, 0.85);
    let shot_corr = reverb(&mix(&base, &clip!("tiro_3"), 0.4), 0.8, 1.5);
    let enemy = reverb(&pad(&clip!("tiro_2"), 1.2), 0.7, 1.1);
    let steps: Vec<Vec<f32>> = [clip!("passo_0"), clip!("passo_1"), clip!("passo_2"), clip!("passo_3"), clip!("passo_4"), clip!("passo_5")].iter().map(|s| reverb(&pad(s, 0.2), 0.18, 0.5)).collect();
    let casing: Vec<Vec<f32>> = [clip!("capsula_0"), clip!("capsula_1"), clip!("capsula_2"), clip!("capsula_3")].iter().map(|s| reverb(&pad(s, 0.3), 0.25, 0.6)).collect();
    let imp1 = reverb(&pad(&clip!("impacto_1"), 0.5), 0.35, 0.7);
    let imp3 = reverb(&pad(&clip!("impacto_3"), 0.5), 0.35, 0.7);
    let metal = reverb(&pad(&mix(&clip!("capsula_0"), &clip!("impacto_3"), 0.5), 0.5), 0.4, 0.8);
    let paper = reverb(&pad(&clip!("papel_1"), 0.3), 0.25, 0.6);
    let thwack = reverb(&pad(&mix(&clip!("papel_1"), &lowpass(&clip!("impacto_1"), 900.0), 0.5), 0.4), 0.3, 0.7);
    let fall = reverb(&pad(&mix(&clip!("queda_1"), &clip!("queda_2"), 0.5), 0.6), 0.4, 0.9);
    let door = reverb(&pad(&clip!("porta_1"), 0.6), 0.35, 0.9);
    let reload = reverb(&pad(&clip!("recarga_1"), 0.3), 0.12, 0.5);
    let buf = |secs: f32| vec![0.0f32; (secs * SR) as usize];
    let good = { let mut b = buf(0.4); tone(&mut b, 0.0, 0.12, 880.0, 880.0, 0.5, false); tone(&mut b, 0.09, 0.2, 1320.0, 1320.0, 0.5, false); b };
    let bad = { let mut b = buf(0.5); tone(&mut b, 0.0, 0.2, 392.0, 392.0, 0.4, false); tone(&mut b, 0.16, 0.3, 294.0, 294.0, 0.4, false); b };
    // ser atingido: baque surdo de impacto no colete e um zumbido curto no ouvido, sem buzina
    let hurt = {
        let thud = mix(&lowpass(&clip!("queda_2"), 260.0), &lowpass(&clip!("impacto_1"), 420.0), 0.8);
        let mut b = pad(&gain(&thud, 1.0), 1.0);
        tone(&mut b, 0.0, 0.14, 70.0, 42.0, 0.9, false);
        let m = b.iter().fold(0.0f32, |x, v| x.max(v.abs())).max(1e-6);
        for v in b.iter_mut() { *v /= m; }
        let n = b.len();
        for (i, v) in b.iter_mut().enumerate() {
            let t = i as f32 / SR;
            let env = (t / 0.06).min(1.0) * (1.0 - i as f32 / n as f32).powf(2.5);
            *v += (t * TAU * 3150.0).sin() * 0.07 * env + (t * TAU * 3170.0).sin() * 0.05 * env;
        }
        b
    };
    let ui = { let mut b = buf(0.08); tone(&mut b, 0.0, 0.04, 1400.0, 1400.0, 0.5, false); b };
    let sounds = Sounds {
        shot_room: add(&mut a, &shot_room, 0.97), shot_corr: add(&mut a, &shot_corr, 0.97), enemy: pair(&mut a, &enemy, 0.95), dry: add(&mut a, &clip!("seco_1"), 0.6), click: add(&mut a, &clip!("interruptor_1"), 0.5),
        reload: add(&mut a, &reload, 0.7), steps: steps.iter().map(|s| add(&mut a, s, 0.7)).collect(), casing: casing.iter().map(|s| add(&mut a, s, 0.45)).collect(),
        impact: vec![pair(&mut a, &imp1, 0.8), pair(&mut a, &imp3, 0.8)], metal: pair(&mut a, &metal, 0.8), paper: pair(&mut a, &paper, 0.7), thwack: pair(&mut a, &thwack, 0.85), fall: pair(&mut a, &fall, 0.85), door: pair(&mut a, &door, 0.85),
        rustle: add(&mut a, &gain(&clip!("roupa_1"), 1.0)[..(0.6 * SR) as usize], 0.35), good: add(&mut a, &good, 0.35), bad: add(&mut a, &bad, 0.3), hurt: add(&mut a, &hurt, 0.6), ui: add(&mut a, &ui, 0.25), n: 0,
    };
    commands.spawn((AudioPlayer::new(a.add(wav(&clip!("ambiente"), 0.5))), PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(0.3), ..default() }));
    commands.insert_resource(sounds);
}

pub fn play_sfx(mut ev: EventReader<Sfx>, mut s: ResMut<Sounds>, mut commands: Commands, p: Res<Player>, level: Res<Level>) {
    let corridor = zones()[p.zone.min(zones().len() - 1)].is_corridor();
    for e in ev.read() {
        s.n = s.n.wrapping_add(1);
        let k = s.n as usize;
        // som vindo de trás de parede ou porta fechada toca a versão abafada
        let hid = |q: Vec3| if level.los(p.eye.x, p.eye.z, p.eye.y, q.x, q.z, q.y.max(0.6)) { 0 } else { 1 };
        let (h, vol, pos): (H, f32, Option<Vec3>) = match *e {
            Sfx::Shot => (if corridor { s.shot_corr.clone() } else { s.shot_room.clone() }, 1.0, None),
            Sfx::EnemyShot(q) => (s.enemy[hid(q)].clone(), 1.7, Some(q)),
            Sfx::Dry => (s.dry.clone(), 0.8, None),
            Sfx::Click => (s.click.clone(), 0.7, None),
            Sfx::Reload => (s.reload.clone(), 0.9, None),
            Sfx::Step(fast) => (s.steps[k % s.steps.len()].clone(), if fast { 0.75 } else { 0.4 }, None),
            Sfx::Rustle => (s.rustle.clone(), 0.5, None),
            Sfx::Impact(q) => (s.impact[k % 2][hid(q)].clone(), 1.5, Some(q)),
            Sfx::Metal(q) => (s.metal[hid(q)].clone(), 1.5, Some(q)),
            Sfx::Paper(q) => (s.paper[hid(q)].clone(), 1.2, Some(q)),
            Sfx::Thwack(q) => (s.thwack[hid(q)].clone(), 1.6, Some(q)),
            Sfx::Fall(q) => (s.fall[hid(q)].clone(), 1.6, Some(q)),
            Sfx::Door(q) => (s.door[0].clone(), 1.5, Some(q)),
            Sfx::Casing(q) => (s.casing[k % s.casing.len()].clone(), 0.9, Some(q)),
            Sfx::Good => (s.good.clone(), 0.6, None),
            Sfx::Bad => (s.bad.clone(), 0.6, None),
            Sfx::Hurt => (s.hurt.clone(), 0.9, None),
            Sfx::Ui => (s.ui.clone(), 0.5, None),
        };
        let mut settings = PlaybackSettings { mode: PlaybackMode::Despawn, volume: Volume::Linear(vol), ..default() };
        match pos {
            Some(q) => { settings.spatial = true; settings.spatial_scale = Some(SpatialScale::new(0.25)); commands.spawn((AudioPlayer::new(h), settings, Transform::from_translation(q))); }
            None => { commands.spawn((AudioPlayer::new(h), settings)); }
        }
    }
}
