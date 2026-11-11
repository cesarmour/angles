//! Ponto de Entrada — simulador didático de CQB (Bevy).
mod art;
mod audio;
mod bot;
mod combat;
mod gi;
mod lessons;
mod map;
mod player;
mod state;
mod tactical;
mod ui;
mod world;

use bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin;
use bevy::pbr::PointLightShadowMap;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};

fn gizmo_cfg(mut store: ResMut<GizmoConfigStore>) {
    let (cfg, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    cfg.line.width = 2.5;
}

/// Diagnóstico (CQB_LIGHTLOG): avisa quando muda o conjunto de luzes que o motor considera visíveis.
fn light_log(g: Res<bevy::pbr::GlobalVisibleClusterableObjects>, spots: Query<(Entity, &SpotLight)>, mut last: Local<(usize, Vec<Entity>)>, mut frame: Local<u32>) {
    if std::env::var("CQB_LIGHTLOG").is_err() { return; }
    *frame += 1;
    let mut v: Vec<Entity> = spots.iter().filter(|(e, _)| g.contains(*e)).map(|(e, _)| e).collect();
    v.sort();
    let n = g.iter().count();
    if n != last.0 || v != last.1 { println!("BOT LUZES quadro {} total {} focos {:?}", *frame, n, v); *last = (n, v); }
}

fn main() {
    if let Ok(dir) = std::env::var("CQB_DUMP") { art::dump(&dir); return; }
    let mut app = App::new();
    if std::env::var("CQB_SSR").is_ok() { app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred()); }
    app
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PointLightShadowMap { size: world::env_f("CQB_SHADOW", 2048.0) as usize })
        .insert_resource(map::Level::new())
        .init_resource::<state::Game>()
        .init_resource::<state::Player>()
        .init_resource::<state::Controls>()
        .init_resource::<bot::Bot>()
        .init_resource::<lessons::DemoSnap>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Point of Entry".into(), resolution: { let r = WindowResolution::new(1440.0, 810.0); match std::env::var("CQB_SCALE").ok().and_then(|s| s.parse::<f32>().ok()) { Some(sc) => r.with_scale_factor_override(sc), None => r } }, present_mode: if std::env::var("CQB_NOVSYNC").is_ok() { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync }, ..default() }),
            ..default()
        }).set(bevy::pbr::PbrPlugin { use_gpu_instance_buffer_builder: std::env::var("CQB_CPUB").is_err(), ..default() }))
        .add_plugins(TemporalAntiAliasPlugin)
        .add_event::<state::Sfx>()
        .add_event::<ui::ResetGame>()
        .add_systems(Startup, (gizmo_cfg, world::setup_world, player::setup_player, combat::setup_targets, tactical::setup_tactical, gi::setup_gi, ui::setup_ui, audio::setup_audio, bot::setup_bot).chain())
        .add_systems(Update, (
            player::gather_input, bot::bot_drive, ui::ui_actions, lessons::demo_system, ui::do_reset, player::cursor_grab,
            player::update_player, combat::combat, combat::update_fx, player::update_gun, player::update_cameras,
            tactical::tac_update, lessons::update_lessons, lessons::coach_system, lessons::replay_system, tactical::tac_gizmos, lessons::guide_system, ui::update_ui, audio::play_sfx,
        ).chain())
        .add_systems(Update, (light_log, world::mip_loaded, world::tame_materials, world::no_shadow_children, player::gun_anim_setup, player::gun_anim_drive, player::body_anim))
        .run();
}
