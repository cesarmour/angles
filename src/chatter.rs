//! Falas dos personagens em cena: cada invasor tem a sua voz (uma por modelo), a refém tem a dela. As falas saem do lugar
//! onde o personagem está (som espacial) e ficam mais baixas quando há parede no meio. Os arquivos estão em
//! assets/voice/ch_<quem>_<fala><n>.mp3 (gerados no ElevenLabs; o roteiro está em tools/voices_characters.tsv).
use bevy::prelude::*;
use bevy::audio::{PlaybackMode, SpatialScale, Volume};
use crate::map::{Level, TKind};
use crate::state::*;

#[derive(Default, Clone, Copy)]
struct Prev { engaged: bool, flinch: bool, retreat: bool, toss: bool, yielded: bool, seen: bool, slice: bool, safe: bool }
#[derive(Default)]
pub struct Chat { prev: Vec<Prev>, cd: [f32; 3], gap: f32, n: usize,
    /// Falas marcadas para daqui a pouco (a refém responde depois da ordem do operador, não por cima dela).
    later: Vec<(f32, String, Vec3, f32)> }

pub fn chatter_system(time: Res<Time>, mut game: ResMut<Game>, targets: Res<Targets>, p: Res<Player>, level: Res<Level>, asset: Res<AssetServer>, mut commands: Commands, mut st: Local<Chat>) {
    let dt = time.delta_secs().min(0.1);
    for c in &mut st.cd { *c = (*c - dt).max(0.0); }
    st.gap = (st.gap - dt).max(0.0);
    if st.prev.len() != targets.list.len() { st.prev = vec![Prev::default(); targets.list.len()]; }
    if game.mode != Mode::Play || game.demo { for q in &mut st.prev { *q = Prev::default(); } st.later.clear(); return; }
    let mut due: Vec<(String, Vec3, f32)> = Vec::new();
    st.later.retain_mut(|l| { l.0 -= dt; if l.0 <= 0.0 { due.push((l.1.clone(), l.2, l.3)); false } else { true } });
    let mut play = |commands: &mut Commands, file: String, pos: Vec3, vol: f32| {
        // parede no meio: a fala chega mais baixa
        let clear = level.los(p.eye.x, p.eye.z, p.eye.y, pos.x, pos.z, 1.5);
        let settings = PlaybackSettings { mode: PlaybackMode::Despawn, volume: Volume::Linear(if clear { vol } else { vol * 0.4 }), spatial: true, spatial_scale: Some(SpatialScale::new(0.25)), ..default() };
        commands.spawn((AudioPlayer::new(asset.load::<AudioSource>(file)), settings, Transform::from_translation(pos)));
    };
    for (f, q, v) in due { play(&mut commands, f, q, v); }
    for (i, t) in targets.list.iter().enumerate() {
        let was = st.prev[i];
        let civil = t.def.kind == TKind::Civil;
        let live = t.alive && !t.off && crate::combat::is_fig(&t.def) && !crate::map::is_mech(&t.def);
        let zone_safe = civil && t.seen && !targets.list.iter().any(|o| o.alive && !o.off && o.def.kind == TKind::Threat && o.def.zone == t.def.zone);
        let now = Prev { engaged: t.engaged, flinch: t.ai.flinch > 0.0, retreat: t.ai.tactic == TAC_RETREAT, toss: t.ai.toss > 0.0,
            yielded: t.mind == MIND_YIELDED, seen: t.seen, slice: t.ai.tactic == TAC_SLICE, safe: zone_safe };
        st.prev[i] = now;
        if !live { continue; }
        // (fala, quantas variações, volume, ignora a espera de quem fala)
        let (who, pick): (usize, Option<(&str, usize, f32, bool)>) = if civil {
            (2, if now.safe && !was.safe { Some(("safe", 2, 1.2, true)) } else if now.seen && !was.seen { Some(("see", 3, 1.5, true)) } else { None })
        } else {
            (i % 2, if now.yielded && !was.yielded { Some(("yield", 1, 1.6, true)) }
                else if now.toss && !was.toss { Some(("nade", 2, 1.6, true)) }
                else if now.retreat && !was.retreat { Some(("back", 2, 1.5, false)) }
                else if now.flinch && !was.flinch { Some(("scare", 2, 1.5, false)) }
                else if now.engaged && !was.engaged { Some(("spot", 3, 1.6, false)) }
                else if now.slice && !was.slice { Some(("door", 2, 0.7, false)) }
                else { None })
        };
        let Some((line, count, vol, force)) = pick else { continue };
        // uma fala por vez: com o instrutor (ou outro personagem) falando, esta fica para a próxima
        if game.talk > 0.0 || st.gap > 0.0 || (!force && st.cd[who] > 0.0) { continue; }
        game.talk = 1.7;
        st.n = st.n.wrapping_add(1);
        st.cd[who] = 3.0; st.gap = 0.7;
        let name = ["inv0", "inv1", "host"][who];
        let file = format!("voice/ch_{name}_{line}{}.mp3", (st.n + i) % count + 1);
        let pos = Vec3::new(t.def.x, 1.5, t.def.z);
        if civil && line == "see" { st.later.push((1.5, file, pos, vol)); } else { play(&mut commands, file, pos, vol); }
    }
}
