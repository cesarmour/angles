//! Ponto de Entrada — simulador didático de CQB (Bevy).
mod art;
mod audio;
mod bot;
mod combat;
mod gi;
mod lessons;
mod map;
mod operator;
mod player;
mod scenario;
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

/// Diagnóstico dos testes automáticos: avisa quando a janela recebe pedido de fechar ou o jogo pede para sair.
fn exit_log(mut close: EventReader<bevy::window::WindowCloseRequested>, mut exit: EventReader<AppExit>, game: Res<state::Game>) {
    if std::env::var("CQB_SCRIPT").is_err() { return; }
    for _ in close.read() { println!("BOT JANELA fechada de fora (modo {:?})", game.mode); }
    for e in exit.read() { println!("BOT SAIDA pedida pelo jogo {:?} (modo {:?})", e, game.mode); }
}

fn main() {
    if let Ok(dir) = std::env::var("CQB_DUMP") { art::dump(&dir); return; }
    // dificuldade inicial nos testes automáticos: CQB_DIFF=0 (easy), 1 (normal) ou 2 (hard)
    if let Some(d) = std::env::var("CQB_DIFF").ok().and_then(|v| v.parse::<u8>().ok()) { state::set_difficulty(d); }
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
        .init_resource::<lessons::Checkpoint>()
        .init_resource::<scenario::Nav>()
        .insert_resource(scenario::Ranking::load())
        .init_resource::<state::MapReq>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Point of Entry".into(), resolution: { let r = WindowResolution::new(world::env_f("CQB_W", 1440.0), world::env_f("CQB_H", 810.0)); match std::env::var("CQB_SCALE").ok().and_then(|s| s.parse::<f32>().ok()) { Some(sc) => r.with_scale_factor_override(sc), None => r } }, present_mode: if std::env::var("CQB_NOVSYNC").is_ok() { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync }, ..default() }),
            ..default()
        }).set(bevy::pbr::PbrPlugin { use_gpu_instance_buffer_builder: std::env::var("CQB_CPUB").is_err(), ..default() }))
        .add_plugins(TemporalAntiAliasPlugin)
        .add_event::<state::Sfx>()
        .add_event::<ui::ResetGame>()
        .add_systems(Startup, (gizmo_cfg, world::setup_world, player::setup_player, operator::setup_operator, combat::setup_targets, tactical::setup_tactical, gi::setup_gi, ui::setup_ui, audio::setup_audio, bot::setup_bot).chain())
        .add_systems(Update, (
            player::gather_input, bot::bot_drive, ui::ui_actions, world::map_switch, lessons::demo_system, ui::do_reset, player::cursor_grab,
            player::update_player, combat::combat, combat::update_fx, player::update_gun, player::update_cameras,
            tactical::tac_update, lessons::update_lessons, lessons::coach_system, lessons::replay_system, tactical::tac_gizmos, lessons::guide_system, ui::update_ui, audio::play_sfx,
        ).chain())
        .add_systems(Update, (light_log, exit_log, world::mip_loaded, world::tame_materials, world::no_shadow_children, player::gun_anim_setup, player::gun_anim_drive, operator::update_operator, operator::fig_system, operator::body_system, operator::hostage_look))
        .add_systems(Update, (operator::room_clear_system, operator::threat_move_system).after(tactical::tac_update).before(ui::update_ui))
        .add_systems(Update, lessons::checkpoint_system.after(lessons::replay_system).before(ui::update_ui))
        .add_systems(Update, scenario::scenario_system.after(combat::combat).before(lessons::update_lessons))
        .add_systems(Update, scenario::rank_system.after(lessons::update_lessons).before(ui::update_ui))
        .add_systems(Update, operator::hands_setup)
        .add_systems(Update, scenario::grenade_system.after(scenario::scenario_system).before(lessons::update_lessons))
        .add_systems(PostUpdate, operator::hands_pose.after(bevy::app::Animation).before(bevy::transform::TransformSystem::TransformPropagate))
        .run();
}
