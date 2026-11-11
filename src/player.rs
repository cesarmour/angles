//! Jogador: entrada, movimento, postura (inclinar e abaixar), câmeras e arma em primeira pessoa.
use crate::bot::Bot;
use crate::map::*;
use crate::state::*;
use crate::tactical::Tac;
use crate::world::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ShadowFilteringMethod, VolumetricFog};
use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::camera::{CameraOutputMode, Exposure, RenderTarget, ScalingMode};
use bevy::render::render_resource::{BlendState, Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;
use bevy::window::{CursorGrabMode, MonitorSelection, PrimaryWindow, WindowMode};

pub const LEAN_M: f32 = 0.38;
/// Folga mínima entre a cabeça e a parede ao inclinar.
pub const LEAN_GAP: f32 = 0.08;
pub const MAG_SIZE: u32 = 8;
const PISTOL: &str = "models/pistola_colt.glb";
const PISTOL_SCALE: f32 = 0.019;
// deslocamentos em relação à pose do autor do modelo: arma em pronto (mais baixa) e mira alinhada ao olho
const GUN_HIP: Vec3 = Vec3::new(0.035, -0.035, 0.01);
const GUN_ADS: Vec3 = Vec3::new(-0.0085, 0.0395, 0.0);
const MUZZLE: Vec3 = Vec3::new(0.0125, -0.05, -0.35);
const WRIST: Vec3 = Vec3::new(0.05, -0.1, -0.16);
pub const FLASH_LUMENS: f32 = 700_000.0;

#[derive(Component)] pub struct FpCam;
#[derive(Component)] pub struct BirdCam;
#[derive(Component)] pub struct MiniCam;
#[derive(Component)] pub struct GunRoot;
#[derive(Component)] pub struct Muzzle;
/// Arquivo do modelo da arma, para ler os clipes de animação pelo nome.
#[derive(Resource)] pub struct GunGltf(pub Handle<Gltf>);
#[derive(Component)] pub struct Body;
/// Perna do operador visto de cima (+1 ou -1): balança no passo.
#[derive(Component)] pub struct BodyLeg(pub f32);
#[derive(Component)] pub struct Flashlight;
#[derive(Component)] pub struct MuzzleLight;
/// Textura em que a câmera do minimapa desenha; a interface a exibe.
#[derive(Resource)] pub struct MiniTex(pub Handle<Image>);

fn part(commands: &mut Commands, parent: Entity, mesh: Handle<Mesh>, mat: &Handle<StandardMaterial>, t: Transform) -> Entity {
    let e = commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat.clone()), t)).id();
    commands.entity(parent).add_child(e);
    e
}
#[allow(dead_code)]
fn limb_t(a: Vec3, b: Vec3) -> (f32, Transform) {
    let d = b - a;
    (d.length(), Transform::from_translation((a + b) / 2.0).with_rotation(Quat::from_rotation_arc(Vec3::Y, d.normalize())))
}

/// Todas as luzes valem para a tela inteira. A repartição padrão por blocos (`CQB_CLUSTERS=1`) deixava quadrados nas paredes.
fn cluster_cfg() -> bevy::pbr::ClusterConfig {
    if std::env::var("CQB_CLUSTERS").is_ok() { bevy::pbr::ClusterConfig::default() } else { bevy::pbr::ClusterConfig::Single }
}

pub fn setup_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut smats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, asset: Res<AssetServer>) {
    let mut mini = Image::new_fill(Extent3d { width: 640, height: 640, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Bgra8UnormSrgb, RenderAssetUsages::default());
    mini.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let mini = images.add(mini);
    commands.insert_resource(MiniTex(mini.clone()));
    let on = |k: &str| std::env::var(k).is_err();
    let fp = commands.spawn((
        Camera3d::default(),
        Camera { hdr: true, order: 0, ..default() },
        Projection::Perspective(PerspectiveProjection { fov: 68f32.to_radians(), near: 0.03, far: 90.0, ..default() }),
        Tonemapping::TonyMcMapface,
        Bloom { intensity: 0.09, ..Bloom::NATURAL },
        Msaa::Off,
        RenderLayers::from_layers(&[0, L_FP]),
        SpatialListener::new(0.2),
        cluster_cfg(),
        Exposure { ev100: env_f("CQB_EV", 9.3) },
        Transform::from_xyz(3.0, EYE_STAND, 15.4),
        FpCam,
    )).id();
    if std::env::var("CQB_SSR").is_ok() {
        commands.entity(fp).insert((bevy::core_pipeline::prepass::DeferredPrepass, bevy::pbr::ScreenSpaceReflections { perceptual_roughness_threshold: 0.45, ..default() }));
    }
    if on("CQB_NOSSAO") { commands.entity(fp).insert(ScreenSpaceAmbientOcclusion::default()); }
    // filtro de sombra: o padrão (gaussiano). O temporal foi testado e fazia a sombra "ferver".
    if on("CQB_NOTAA") { commands.entity(fp).insert(TemporalAntiAliasing::default()); }
    if std::env::var("CQB_SHADOWT").is_ok() { commands.entity(fp).insert(ShadowFilteringMethod::Temporal); }
    if std::env::var("CQB_VOL").is_ok() { commands.entity(fp).insert(VolumetricFog { ambient_intensity: 0.0, step_count: env_f("CQB_STEPS", 8.0) as u32, jitter: 1.0, ..default() }); }
    let ortho = |h: f32| Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::FixedVertical { viewport_height: h }, near: 0.1, far: 80.0, ..OrthographicProjection::default_3d() });
    commands.spawn((
        Camera3d::default(), Camera { hdr: true, order: 0, is_active: false, ..default() }, ortho(13.0),
        Tonemapping::TonyMcMapface, Bloom { intensity: 0.08, ..Bloom::NATURAL }, Msaa::Sample4,
        RenderLayers::from_layers(&[0, L_BODY, L_TOP]), Transform::from_xyz(3.0, 40.0, 15.4).looking_at(Vec3::new(3.0, 0.0, 15.4), Vec3::NEG_Z), Exposure { ev100: env_f("CQB_EVB", 9.2) }, cluster_cfg(), BirdCam,
    ));
    commands.spawn((
        Camera3d::default(), Camera { order: -1, is_active: false, target: RenderTarget::Image(mini.into()), clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() }, ortho(11.0),
        Tonemapping::TonyMcMapface, Msaa::Off,
        RenderLayers::from_layers(&[0, L_BODY, L_TOP]), Transform::from_xyz(3.0, 40.0, 15.4).looking_at(Vec3::new(3.0, 0.0, 15.4), Vec3::NEG_Z), Exposure { ev100: env_f("CQB_EVB", 9.2) }, cluster_cfg(), MiniCam,
    ));
    // a interface tem tela própria, transparente, composta por cima da cena
    commands.spawn((Camera2d, Camera { order: 10, clear_color: ClearColorConfig::Custom(Color::NONE), output_mode: CameraOutputMode::Write { blend_state: Some(BlendState::ALPHA_BLENDING), clear_color: ClearColorConfig::None }, ..default() }, IsDefaultUiCamera, Msaa::Off, RenderLayers::layer(7)));

    /* --- pistola com mãos (modelo glTF com esqueleto) --- */
    let g = commands.spawn((Transform::IDENTITY, Visibility::default(), GunRoot)).id();
    commands.entity(fp).add_child(g);
    // no arquivo, o olho do autor fica em (0, 34, 0) olhando para +Z; aqui a frente é -Z
    let model = commands.spawn((
        SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset(PISTOL))),
        Transform { translation: Vec3::new(0.0, -34.0 * PISTOL_SCALE, 0.0), rotation: Quat::from_rotation_y(std::f32::consts::PI), scale: Vec3::splat(PISTOL_SCALE) },
        NoShadowScene,
    )).id();
    commands.entity(g).add_child(model);
    commands.insert_resource(GunGltf(asset.load("models/pistola_colt_anim.glb")));
    let mz = commands.spawn((Transform::from_xyz(MUZZLE.x, MUZZLE.y, MUZZLE.z), Muzzle)).id();
    commands.entity(g).add_child(mz);

    /* --- corpo do operador nas vistas de cima: desenho visto de cima (capacete, colete, braços na pistola) e botas que alternam no passo.
       O corpo não projeta sombra: a sombra de um corpo simplificado deformava no chão e piscava no facho da lanterna.
       O desenho já traz uma sombra de contato suave. --- */
    let body = commands.spawn((Transform::from_xyz(3.0, 0.0, 15.4), Visibility::default(), Body)).id();
    let flat_q = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let sprite = |smats: &mut Assets<StandardMaterial>, img: Handle<Image>| smats.add(StandardMaterial { base_color_texture: Some(img), alpha_mode: AlphaMode::Blend, perceptual_roughness: 1.0, reflectance: 0.05, cull_mode: None, ..default() });
    let op = sprite(&mut smats, images.add(crate::art::svg_image(&crate::art::operator_svg(), crate::art::OP_PX, crate::art::OP_PX, true)));
    let e = part(&mut commands, body, meshes.add(Rectangle::new(crate::art::OP_M, crate::art::OP_M)), &op, Transform::from_xyz(0.0, 1.8, -crate::art::OP_BACK * 1.18).with_rotation(flat_q).with_scale(Vec3::splat(1.18)));
    commands.entity(e).insert((RenderLayers::layer(L_BODY), bevy::pbr::NotShadowCaster, bevy::pbr::NotShadowReceiver));
    let boot_m = sprite(&mut smats, images.add(crate::art::svg_image(&crate::art::boot_svg(), 96, 192, true)));
    for (side, x, z) in [(1.0f32, -0.13, -0.1), (-1.0, 0.13, 0.09)] {
        let leg = commands.spawn((Transform::IDENTITY, Visibility::default(), BodyLeg(side))).id();
        commands.entity(body).add_child(leg);
        let e = part(&mut commands, leg, meshes.add(Rectangle::new(0.16, 0.32)), &boot_m, Transform::from_xyz(x, 0.14, z - 0.03).with_rotation(flat_q));
        commands.entity(e).insert((RenderLayers::layer(L_BODY), bevy::pbr::NotShadowCaster, bevy::pbr::NotShadowReceiver));
    }

    commands.spawn((
        SpotLight { color: Color::srgb(0.95, 0.97, 1.0), intensity: 0.0, range: 40.0, radius: 0.01, shadows_enabled: std::env::var("CQB_SOMBRAS").is_ok(), outer_angle: 0.42, inner_angle: 0.16, shadow_map_near_z: 0.6, shadow_depth_bias: env_f("CQB_FDB", 0.02), shadow_normal_bias: env_f("CQB_FNB", 1.8), ..default() },
        Transform::default(), light_layers(), Flashlight,
    ));
    commands.spawn((PointLight { color: Color::srgb(1.0, 0.76, 0.48), intensity: 0.0, range: 10.0, shadows_enabled: false, ..default() }, Transform::default(), light_layers(), MuzzleLight));
}

pub fn gather_input(
    keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>, motion: Res<AccumulatedMouseMotion>, scroll: Res<AccumulatedMouseScroll>,
    mut c: ResMut<Controls>, bot: Res<Bot>, game: Res<Game>, mut aim_lock: Local<bool>,
) {
    if bot.active { return; }
    let k = |code: KeyCode| if keys.pressed(code) { 1.0 } else { 0.0 };
    let play = game.mode == Mode::Play;
    // mira sem botão direito (trackpad): segurar Espaço, ou Z para travar/destravar a mira
    if !play || game.demo { *aim_lock = false; } else if keys.just_pressed(KeyCode::KeyZ) { *aim_lock = !*aim_lock; }
    *c = Controls {
        mv: Vec2::new(k(KeyCode::KeyD) - k(KeyCode::KeyA), k(KeyCode::KeyW) - k(KeyCode::KeyS)),
        look: if play { motion.delta } else { Vec2::ZERO },
        fire: play && mouse.just_pressed(MouseButton::Left),
        ads: mouse.pressed(MouseButton::Right) || (play && (keys.pressed(KeyCode::Space) || *aim_lock)),
        lean: k(KeyCode::KeyE) - k(KeyCode::KeyQ),
        crouch_toggle: keys.just_pressed(KeyCode::KeyC),
        crouch_hold: keys.pressed(KeyCode::ControlLeft),
        fast: keys.pressed(KeyCode::ShiftLeft),
        reload: keys.just_pressed(KeyCode::KeyR),
        interact: keys.just_pressed(KeyCode::KeyF),
        tac: keys.just_pressed(KeyCode::KeyG),
        view: keys.just_pressed(KeyCode::KeyV),
        flash: play && (keys.just_pressed(KeyCode::KeyL) || mouse.just_pressed(MouseButton::Middle)),
        help: keys.just_pressed(KeyCode::KeyH),
        confirm: keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space),
        pause: keys.just_pressed(KeyCode::Escape),
        zoom: scroll.delta.y,
        world_move: None,
        guide: keys.just_pressed(KeyCode::KeyT),
        hide_box: play && keys.just_pressed(KeyCode::KeyO),
        pace: 0.0,
    };
}

pub fn cursor_grab(game: Res<Game>, bot: Res<Bot>, keys: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window, With<PrimaryWindow>>, mut first: Local<bool>) {
    let Ok(mut w) = windows.single_mut() else { return };
    // tela cheia ao abrir (fora do modo de teste); F11 alterna
    let toggle = keys.just_pressed(KeyCode::F11) || (!*first && !bot.active && std::env::var("CQB_JANELA").is_err());
    *first = true;
    if toggle {
        w.mode = if matches!(w.mode, WindowMode::Windowed) { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
    }
    let want = game.mode == Mode::Play && !bot.active;
    let mode = if want { CursorGrabMode::Locked } else { CursorGrabMode::None };
    if w.cursor_options.grab_mode != mode { w.cursor_options.grab_mode = mode; w.cursor_options.visible = !want; }
}

pub fn update_player(time: Res<Time>, c: Res<Controls>, mut p: ResMut<Player>, mut game: ResMut<Game>, level: Res<Level>, tac: Res<Tac>, targets: Res<Targets>, mut sfx: EventWriter<Sfx>) {
    if game.mode != Mode::Play { p.speed = 0.0; return; }
    let dt = time.delta_secs().min(0.05);
    game.time += dt;
    let bird = game.view == View::Bird;
    let s = 0.0021 * game.sens * if p.ads { 0.62 } else { 1.0 };
    let (dyaw, dpitch) = (-c.look.x * s, -c.look.y * s);
    p.yaw += dyaw;
    if p.yaw > std::f32::consts::PI { p.yaw -= std::f32::consts::TAU } else if p.yaw < -std::f32::consts::PI { p.yaw += std::f32::consts::TAU }
    p.pitch = (p.pitch + dpitch).clamp(-1.25, 1.25);
    if bird { p.pitch = damp(p.pitch, 0.0, 8.0, dt); }
    p.look_acc += dyaw.abs() + dpitch.abs();
    let (f, r) = (p.fwd(), p.right());
    let mut w = if let Some(wm) = c.world_move { wm } else if bird { Vec2::new(c.mv.x, -c.mv.y) } else { f * c.mv.y + r * c.mv.x };
    if w.length() > 1.0 { w = w.normalize(); }
    // postura
    if c.crouch_toggle { p.crouching = !p.crouching; }
    let low = p.crouching || c.crouch_hold;
    // roupa e equipamento farfalham ao mudar de postura
    if (low && p.crouch < 0.08) || (!low && p.crouch > 0.92) || (c.lean != 0.0 && p.lean.abs() < 0.04) { sfx.write(Sfx::Rustle); }
    p.crouch = damp(p.crouch, if low { 1.0 } else { 0.0 }, 9.0, dt);
    if low { p.crouch_acc += dt; }
    p.fast = c.fast && w.length() > 0.1 && (bird || c.mv.y > 0.0 || c.world_move.is_some()) && !c.ads && !low && c.lean == 0.0;
    p.ads = c.ads && !p.reloading && !p.fast;
    if p.ads { p.ads_acc += dt; }
    let mut spd = if p.fast { 3.7 } else if low { 1.2 } else if p.ads { 1.35 } else { 2.05 };
    if c.lean != 0.0 { spd *= 0.55; }
    if c.pace > 0.0 { spd *= c.pace; }
    p.vel.x = damp(p.vel.x, w.x * spd, 11.0, dt);
    p.vel.y = damp(p.vel.y, w.y * spd, 11.0, dt);
    let old = p.pos;
    let mut np = p.pos + p.vel * dt;
    level.resolve_circle(&mut np, PLAYER_R);
    p.pos = np;
    let moved = (p.pos - old).length();
    p.speed = if dt > 0.0 { moved / dt } else { 0.0 };
    p.move_acc += moved; p.bob += moved; p.step_acc += moved;
    if p.step_acc > if p.fast { 1.15 } else { 0.78 } { p.step_acc = 0.0; sfx.write(Sfx::Step(p.fast)); }
    // inclinação do tronco, limitada pela parede ao lado
    let mut lim = 1.0;
    if c.lean != 0.0 {
        let d = level.cast_ray(p.pos.x, p.pos.y, r.x * c.lean, r.y * c.lean, 1.2, p.eye.y - 0.35);
        lim = ((d - LEAN_GAP) / LEAN_M).clamp(0.0, 1.0);
    }
    p.lean_blocked = c.lean != 0.0 && lim < 0.35;
    p.lean = damp(p.lean, c.lean * lim, 9.0, dt);
    if p.lean.abs() > 0.5 { p.lean_acc += dt; }
    let eye_h = EYE_STAND + (EYE_CROUCH - EYE_STAND) * p.crouch - p.lean.abs() * 0.05;
    p.eye = Vec3::new(p.pos.x + r.x * p.lean * LEAN_M, eye_h, p.pos.y + r.y * p.lean * LEAN_M);
    // zona, entradas e tempo no funil fatal
    if let Some(z) = zone_at(p.pos.x, p.pos.y) { p.zone = z; }
    for (k, w) in DOORWAYS.iter().enumerate() {
        let zi = zone_index(w.zone);
        let coord = if w.axis_z { p.pos.y } else { p.pos.x };
        if game.entry[k].is_none() && p.zone == zi && (coord - w.v) * w.s > 0.0 { game.entry[k] = Some(tac.pct[zi]); game.entry_pie[k] = Some(tac.pie_deg(k + 1)); }
        let inside = p.pos.x >= w.x1 && p.pos.x <= w.x2 && p.pos.y >= w.z1 && p.pos.y <= w.z2;
        if inside && (tac.pct[zi] < 0.95 || targets.list.iter().any(|t| t.alive && t.def.kind == TKind::Threat && t.def.zone == w.zone)) { game.dwell[k] += dt; }
    }
    if p.fire_cool > 0.0 { p.fire_cool -= dt; }
    if c.reload && !p.reloading && p.mag < MAG_SIZE && p.reserve > 0 { p.reloading = true; p.reload_t = 0.0; sfx.write(Sfx::Reload); }
    if p.reloading {
        p.reload_t += dt / 2.1;
        if p.reload_t >= 1.0 { let n = (MAG_SIZE - p.mag).min(p.reserve); p.mag += n; p.reserve -= n; p.reloading = false; p.reloads += 1; }
    }
    if c.flash { game.flash = !game.flash; game.flag_flash = true; game.ui_dirty = true; sfx.write(Sfx::Click); }
    if c.guide && !game.free { game.next_guide(); sfx.write(Sfx::Ui); }
    if c.hide_box { game.hide_box = !game.hide_box; game.ui_dirty = true; sfx.write(Sfx::Ui); }
    if c.tac { game.tac = !game.tac; if game.tac { game.flag_tac = true; } sfx.write(Sfx::Ui); game.ui_dirty = true; }
    if c.view {
        game.view = if bird { View::Fp } else { View::Bird };
        if game.view == View::Bird { game.flag_bird = true; } else if game.flag_bird { game.flag_bird_back = true; }
        sfx.write(Sfx::Ui); game.ui_dirty = true;
    }
    if bird && c.zoom != 0.0 { game.bird_zoom = (game.bird_zoom - c.zoom.signum() * 1.5).clamp(8.0, 24.0); }
}

pub fn update_cameras(
    time: Res<Time>, mut p: ResMut<Player>, game: Res<Game>, level: Res<Level>,
    mut cams: ParamSet<(
        Query<(&mut Transform, &mut Projection, &mut Camera), With<FpCam>>,
        Query<(&mut Transform, &mut Projection, &mut Camera), With<BirdCam>>,
        Query<(&mut Transform, &mut Camera), With<MiniCam>>,
        Query<&mut Transform, With<Body>>,
        Query<(&mut Transform, &mut SpotLight), With<Flashlight>>,
        Query<(&mut SpotLight, &CeilLight), Without<Flashlight>>,
    )>,
    tubes: Res<Tubes>, mut smats: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs().min(0.05);
    let t = time.elapsed_secs();
    let replay = game.mode == Mode::Replay;
    let bird = game.view == View::Bird || replay;
    // no replay a câmera de cima enquadra o trajeto inteiro
    let (mut rc, mut rz) = (p.pos, game.bird_zoom);
    let mut rs: Option<(Vec2, f32)> = None;
    if replay && !game.track.is_empty() {
        let (mut lo, mut hi) = (game.track[0].1, game.track[0].1);
        for t in &game.track { lo = lo.min(t.1); hi = hi.max(t.1); }
        rc = (lo + hi) / 2.0; rz = ((hi - lo).max_element() + 5.0).max(9.0);
        let s = game.track[crate::lessons::replay_index(&game)];
        rs = Some((s.1, s.2));
    }
    if game.mode == Mode::Menu {
        p.yaw = (t * 0.17).sin() * 0.5; p.pitch = (t * 0.11).sin() * 0.04 - 0.02;
        p.eye = Vec3::new(p.pos.x, EYE_STAND, p.pos.y);
    }
    let a = p.g_ads * p.g_ads * (3.0 - 2.0 * p.g_ads);
    let bob_amt = (p.speed / 2.0).min(1.0) * (1.0 - a * 0.6);
    let rot = Quat::from_euler(EulerRot::YXZ, p.yaw + p.recoil.y, p.pitch + p.recoil.x, -p.lean * 0.16);
    if let Ok((mut tf, mut proj, mut cam)) = cams.p0().single_mut() {
        tf.translation = p.eye + Vec3::Y * ((p.bob * 3.9).sin() * 0.012 * bob_amt);
        tf.rotation = rot;
        if let Projection::Perspective(pp) = proj.as_mut() { pp.fov = (68.0 + (50.0 - 68.0) * a + p.g_lower * 3.0f32).to_radians(); }
        if cam.is_active == bird { cam.is_active = !bird; }
    }
    if let Ok((mut tf, mut proj, mut cam)) = cams.p1().single_mut() {
        let cur = Vec2::new(tf.translation.x, tf.translation.z);
        let nx = if bird { damp(cur.x, rc.x, 6.0, dt) } else { p.pos.x };
        let nz = if bird { damp(cur.y, rc.y, 6.0, dt) } else { p.pos.y };
        *tf = Transform::from_xyz(nx, 40.0, nz).looking_at(Vec3::new(nx, 0.0, nz), Vec3::NEG_Z);
        if let Projection::Orthographic(o) = proj.as_mut() { o.scaling_mode = ScalingMode::FixedVertical { viewport_height: rz }; }
        if cam.is_active != bird { cam.is_active = bird; }
    }
    if let Ok((mut tf, mut cam)) = cams.p2().single_mut() {
        let f = p.fwd();
        let c = p.pos + f * 2.2;
        *tf = Transform::from_xyz(c.x, 40.0, c.y).looking_at(Vec3::new(c.x, 0.0, c.y), Vec3::new(f.x, 0.0, f.y));
        let want = game.tac && !bird && matches!(game.mode, Mode::Play);
        if cam.is_active != want { cam.is_active = want; }
    }
    if let Ok(mut tf) = cams.p3().single_mut() {
        let (bp, by) = rs.unwrap_or((p.pos, p.yaw));
        tf.translation = Vec3::new(bp.x, 0.0, bp.y);
        tf.rotation = Quat::from_rotation_y(by) * Quat::from_rotation_z(-p.lean * 0.22);
        tf.scale = Vec3::new(1.0, 1.0 - p.crouch * 0.34, 1.0);
    }
    // lanterna presa à arma, sempre à frente da boca do cano
    let dirq = Quat::from_euler(EulerRot::YXZ, p.yaw, if bird { 0.0 } else { p.pitch + p.recoil.x }, 0.0);
    let dir = dirq * Vec3::NEG_Z;
    let right = dirq * Vec3::X;
    let f = p.fwd();
    let fd = level.cast_ray(p.eye.x, p.eye.z, f.x, f.y, 1.3, 0.0);
    let ahead = (fd - 0.12).clamp(0.1, if bird { 0.6 } else { 0.46 });
    if let Ok((mut tf, mut l)) = cams.p4().single_mut() {
        let pos = p.eye + dir * ahead + right * (0.02 + 0.04 * (1.0 - a)) - Vec3::Y * 0.09;
        *tf = Transform::from_translation(pos).looking_at(pos + dir * 10.0, Vec3::Y);
        let lower = if bird { 0.0 } else { p.g_lower.max(p.g_block) };
        l.intensity = if game.flash && game.mode != Mode::Menu { FLASH_LUMENS * (1.0 - lower * 0.85) } else { 0.0 };
    }
}

pub fn update_gun(time: Res<Time>, mut p: ResMut<Player>, c: Res<Controls>, game: Res<Game>, level: Res<Level>, mut gun: Query<&mut Transform, With<GunRoot>>) {
    let dt = time.delta_secs().min(0.05);
    let play = game.mode == Mode::Play;
    let f = p.fwd();
    let block_d = level.cast_ray(p.eye.x, p.eye.z, f.x, f.y, 3.0, p.eye.y - 0.45) / p.pitch.cos().max(0.3);
    p.g_ads = damp(p.g_ads, if p.ads && play { 1.0 } else { 0.0 }, 14.0, dt);
    p.g_kick = damp(p.g_kick, 0.0, 14.0, dt);
    p.recoil.x = damp(p.recoil.x, 0.0, 7.0, dt); p.recoil.y = damp(p.recoil.y, 0.0, 7.0, dt);
    p.g_block = damp(p.g_block, ((0.62 - block_d) / 0.3).clamp(0.0, 1.0), 12.0, dt);
    p.g_lower = damp(p.g_lower, if p.fast && p.speed > 2.4 { 1.0 } else { 0.0 }, 9.0, dt);
    let look = if play { c.look } else { Vec2::ZERO };
    p.g_sway.x = damp(p.g_sway.x, (-look.x * 0.0009).clamp(-0.05, 0.05), 10.0, dt);
    p.g_sway.y = damp(p.g_sway.y, (-look.y * 0.0009).clamp(-0.05, 0.05), 10.0, dt);
    let a = p.g_ads * p.g_ads * (3.0 - 2.0 * p.g_ads);
    let calm = 1.0 - a * 0.85;
    let sp = p.speed.min(1.0);
    let bob = p.bob * 3.6;
    let mut pos = GUN_HIP.lerp(GUN_ADS, a);
    pos.x += bob.sin() * 0.005 * calm * sp + p.g_sway.x * 0.25;
    pos.y += -bob.cos().abs() * 0.0045 * calm * sp + p.g_sway.y * 0.25 - p.g_block * 0.03 - p.g_lower * 0.12;
    pos.z += p.g_kick * 0.008 + p.g_block * 0.06;
    let rx = p.g_kick * 0.07 + p.g_sway.y * 0.6 + p.g_block * 0.8 - p.g_lower * 0.7;
    let ry = p.g_sway.x * 0.8 + p.g_lower * 0.2;
    let mut rz = p.g_sway.x * -0.6 - p.lean * 0.05 * (1.0 - a);
    if p.reloading { let t = p.reload_t.clamp(0.0, 1.0); let k = (t * std::f32::consts::PI).sin(); pos.y -= k * 0.01; rz += k * 0.04; }
    if let Ok(mut tf) = gun.single_mut() {
        let rot = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        tf.rotation = rot;
        tf.translation = pos + WRIST - rot * WRIST;
    }
}

#[derive(Resource)]
pub struct GunAnim { pub player: Entity, pub idle: AnimationNodeIndex, pub shoot: AnimationNodeIndex, pub reload: AnimationNodeIndex, pub reload_len: f32 }

/// Liga os clipes de animação (parada, tiro, recarga) ao esqueleto da arma assim que modelo e clipes carregam.
pub fn gun_anim_setup(mut commands: Commands, gun: Res<GunGltf>, gltfs: Res<Assets<Gltf>>, clips: Res<Assets<AnimationClip>>, mut graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer)>, names: Query<(Entity, &Name), Added<Name>>,
    skinned: Query<Entity, Added<bevy::render::mesh::skinning::SkinnedMesh>>, ready: Option<Res<GunAnim>>) {
    for (e, n) in &names { if n.as_str().contains("Icosphere") { commands.entity(e).insert(Visibility::Hidden); } }
    // malhas com esqueleto têm caixa de recorte calculada na pose de repouso: sem isto elas somem da tela
    for e in &skinned { commands.entity(e).insert(bevy::render::view::NoFrustumCulling); }
    if ready.is_some() { return; }
    let Some(g) = gltfs.get(&gun.0) else { return };
    let Some((e, mut pl)) = players.iter_mut().next() else { return };
    let (Some(idle), Some(shoot), Some(reload)) = (g.named_animations.get("Idle"), g.named_animations.get("Shoot"), g.named_animations.get("Reload")) else { return };
    let reload_len = clips.get(reload).map(|c| c.duration()).unwrap_or(3.7);
    let (graph, idx) = AnimationGraph::from_clips([idle.clone(), shoot.clone(), reload.clone()]);
    commands.entity(e).insert(AnimationGraphHandle(graphs.add(graph)));
    pl.play(idx[0]).repeat();
    commands.insert_resource(GunAnim { player: e, idle: idx[0], shoot: idx[1], reload: idx[2], reload_len });
}

/// Toca tiro e recarga conforme o que o jogador faz.
pub fn gun_anim_drive(anim: Option<Res<GunAnim>>, p: Res<Player>, mut players: Query<&mut AnimationPlayer>, mut last: Local<(u32, bool)>) {
    let Some(a) = anim else { return };
    let Ok(mut pl) = players.get_mut(a.player) else { return };
    if p.reloading && !last.1 {
        pl.stop_all();
        pl.start(a.reload).set_speed(a.reload_len / 2.1);
    } else if p.shot_n != last.0 && !p.reloading {
        pl.stop_all();
        pl.start(a.shoot);
    } else if !p.reloading {
        let busy = pl.animation(a.shoot).map_or(false, |x| !x.is_finished()) || pl.animation(a.reload).map_or(false, |x| !x.is_finished());
        if !busy && !pl.is_playing_animation(a.idle) { pl.stop_all(); pl.play(a.idle).repeat(); }
    }
    *last = (p.shot_n, p.reloading);
}

/// Passada do operador nas vistas de cima: as pernas alternam conforme a distância andada.
pub fn body_anim(p: Res<Player>, mut legs: Query<(&mut Transform, &BodyLeg)>) {
    let sw = (p.bob * 3.9).sin() * 0.17 * (p.speed / 1.6).min(1.0);
    for (mut tf, l) in &mut legs { tf.translation.z = l.0 * sw; }
}
