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
/// Duração da troca de arma: metade guardando a atual, metade sacando a outra.
pub const SWITCH_S: f32 = 0.9;
pub const FLASH_LUMENS: f32 = 700_000.0;

#[derive(Component)] pub struct FpCam;
#[derive(Component)] pub struct BirdCam;
#[derive(Component)] pub struct MiniCam;
#[derive(Component)] pub struct GunRoot;
#[derive(Component)] pub struct Muzzle;
/// Modelo de uma das armas (índice em `WEAPONS`); só o da arma em uso fica visível.
#[derive(Component)] pub struct GunModel(pub usize);
/// Arquivos de onde vêm os clipes de cada arma, para lê-los pelo nome.
#[derive(Resource)] pub struct GunGltfs(pub Vec<Handle<Gltf>>);
#[derive(Component)] pub struct Body;
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

    /* --- armas com mãos (modelos glTF com esqueleto): pistola e SCAR; só a arma em uso aparece --- */
    let g = commands.spawn((Transform::IDENTITY, Visibility::default(), GunRoot)).id();
    commands.entity(fp).add_child(g);
    let mut files = Vec::new();
    for (i, w) in WEAPONS.iter().enumerate() {
        // no arquivo, o olho do autor fica em `eye` olhando para +Z; aqui a frente é -Z
        let model = commands.spawn((
            SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset(w.file))),
            Transform { translation: Vec3::new(w.eye.x, -w.eye.y, w.eye.z) * w.scale, rotation: Quat::from_rotation_y(std::f32::consts::PI), scale: Vec3::splat(w.scale) },
            if i == 0 { Visibility::Inherited } else { Visibility::Hidden },
            NoShadowScene, GunModel(i),
        )).id();
        commands.entity(g).add_child(model);
        files.push(asset.load(w.anim));
    }
    commands.insert_resource(GunGltfs(files));
    commands.insert_resource(GunAnims((0..WEAPONS.len()).map(|_| None).collect()));
    let mz = commands.spawn((Transform::from_translation(WEAPONS[0].muzzle), Muzzle)).id();
    commands.entity(g).add_child(mz);

    /* --- corpo do operador nas vistas de cima: o mesmo boneco 3D dos demais, de preto, com a pistola; só as câmeras de cima
       o enxergam (camada L_BODY). No arquivo ele olha para +Z; aqui a frente é -Z. --- */
    let body = commands.spawn((Transform::from_xyz(3.0, 0.0, 15.4), Visibility::default(), Body)).id();
    let fig = commands.spawn((
        SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset("models/player.glb"))),
        Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI)), Visibility::default(),
        crate::operator::BodyScene(asset.load("models/player.glb")),
    )).id();
    commands.entity(body).add_child(fig);

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
        weapon: if !play { None } else if keys.just_pressed(KeyCode::Digit1) { Some(0) } else if keys.just_pressed(KeyCode::Digit2) { Some(1) } else { None },
        weapon_next: play && keys.just_pressed(KeyCode::KeyX),
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
    // operador abatido: não responde mais aos comandos
    let dead = Controls::default();
    let c: &Controls = if game.down_t > 0.0 { &dead } else { &c };
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
    p.ads = c.ads && !p.reloading && !p.fast && !p.switching();
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
    for (k, w) in doorways().iter().enumerate() {
        let zi = zone_index(w.zone);
        let coord = if w.axis_z { p.pos.y } else { p.pos.x };
        if game.entry[k].is_none() && p.zone == zi && (coord - w.v) * w.s > 0.0 { game.entry[k] = Some(tac.pct[zi]); game.entry_pie[k] = Some(tac.pie_deg(w.opening)); }
        let inside = p.pos.x >= w.x1 && p.pos.x <= w.x2 && p.pos.y >= w.z1 && p.pos.y <= w.z2;
        if inside && (tac.pct[zi] < 0.95 || targets.list.iter().any(|t| t.alive && t.def.kind == TKind::Threat && t.def.zone == w.zone)) { game.dwell[k] += dt; }
    }
    if p.fire_cool > 0.0 { p.fire_cool -= dt; }
    // troca de arma: a atual desce (clipe de guardar), a outra sobe (clipe de sacar); cada arma guarda a própria munição
    let want = if c.weapon_next { Some((p.weapon + 1) % WEAPONS.len()) } else { c.weapon };
    if let Some(w) = want { if w < WEAPONS.len() && w != p.weapon && !p.switching() && !p.reloading { p.switch_to = Some(w); p.switch_t = 0.0; sfx.write(Sfx::Rustle); } }
    if let Some(w) = p.switch_to {
        let before = p.switch_t;
        p.switch_t += dt / SWITCH_S;
        if before < 0.5 && p.switch_t >= 0.5 {
            let cur = p.weapon;
            p.ammo[cur] = (p.mag, p.reserve);
            p.weapon = w; p.mag = p.ammo[w].0; p.reserve = p.ammo[w].1;
            sfx.write(Sfx::Click); game.ui_dirty = true;
        }
        if p.switch_t >= 1.0 { p.switch_to = None; p.switch_t = 0.0; }
    }
    // recarga tática (ainda há munição: mais rápida e fica um cartucho na câmara) ou de emergência (carregador vazio: mais lenta)
    let def = p.def();
    if c.reload && !p.reloading && !p.switching() && p.mag < def.mag && p.reserve > 0 {
        p.reloading = true; p.reload_t = 0.0; p.reload_empty = p.mag == 0;
        p.reload_dur = if p.reload_empty { def.reload_empty_s } else { def.reload_s };
        sfx.write(Sfx::Reload);
        if p.reload_empty && !p.warned_empty && !game.demo {
            p.warned_empty = true;
            game.toast("Emergency reload: you ran dry. Reload behind cover before the next door \u{2014} a tactical reload is faster and keeps one round in the chamber.", 2, 6.0);
        }
    }
    if p.reloading {
        p.reload_t += dt / p.reload_dur;
        if p.reload_t >= 1.0 {
            let cap = def.mag + if p.reload_empty { 0 } else { 1 };
            let n = cap.saturating_sub(p.mag).min(p.reserve);
            p.mag += n; p.reserve -= n; p.reloading = false; p.reloads += 1;
        }
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
        // operador abatido: a câmera desaba para o chão e tomba de lado
        let k = if game.mode == Mode::Down { if game.down_title.is_empty() { 1.0 } else { 0.0 } } else if game.down_t > 0.0 { (1.0 - game.down_t / DOWN_SECS).clamp(0.0, 1.0) } else { 0.0 };
        if k > 0.0 {
            let e = k * k * (3.0 - 2.0 * k);
            tf.translation.y += (0.34 - tf.translation.y) * e;
            tf.rotation = rot * Quat::from_euler(EulerRot::YXZ, 0.0, 0.25 * e, 1.05 * e);
        }
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

pub fn update_gun(time: Res<Time>, mut p: ResMut<Player>, c: Res<Controls>, game: Res<Game>, level: Res<Level>, mut gun: Query<&mut Transform, With<GunRoot>>,
    mut muzzle: Query<&mut Transform, (With<Muzzle>, Without<GunRoot>)>, mut models: Query<(&GunModel, &mut Visibility)>) {
    let def = p.def();
    // só a arma em uso aparece; no menu a tela é do operador, sem arma na frente
    for (m, mut v) in &mut models {
        let want = if m.0 == p.weapon && game.mode != Mode::Menu { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
    if let Ok(mut m) = muzzle.single_mut() { if m.translation != def.muzzle { m.translation = def.muzzle; } }
    let dt = time.delta_secs().min(0.05);
    let play = game.mode == Mode::Play;
    let f = p.fwd();
    let block_d = level.cast_ray(p.eye.x, p.eye.z, f.x, f.y, 3.0, p.eye.y - 0.45) / p.pitch.cos().max(0.3);
    p.g_ads = damp(p.g_ads, if p.ads && play { 1.0 } else { 0.0 }, 14.0, dt);
    p.g_kick = damp(p.g_kick, 0.0, 14.0, dt);
    p.recoil.x = damp(p.recoil.x, 0.0, 7.0, dt); p.recoil.y = damp(p.recoil.y, 0.0, 7.0, dt);
    p.g_block = damp(p.g_block, ((def.block - block_d) / 0.3).clamp(0.0, 1.0), 12.0, dt);
    p.g_lower = damp(p.g_lower, if p.fast && p.speed > 2.4 { 1.0 } else { 0.0 }, 9.0, dt);
    let look = if play { c.look } else { Vec2::ZERO };
    p.g_sway.x = damp(p.g_sway.x, (-look.x * 0.0009).clamp(-0.05, 0.05), 10.0, dt);
    p.g_sway.y = damp(p.g_sway.y, (-look.y * 0.0009).clamp(-0.05, 0.05), 10.0, dt);
    let a = p.g_ads * p.g_ads * (3.0 - 2.0 * p.g_ads);
    let calm = 1.0 - a * 0.85;
    let sp = p.speed.min(1.0);
    let bob = p.bob * 3.6;
    // arma com clipe próprio de andar/correr balança menos por código, para não somar os dois movimentos
    let bobk = if def.walk.is_empty() { 1.0 } else { 0.3 };
    let lower = p.g_lower * if def.run.is_empty() { 1.0 } else { 0.25 };
    // na troca a arma desce até sair da tela, o modelo é trocado lá embaixo e a outra sobe
    let sw = if p.switching() { (p.switch_t * std::f32::consts::PI).sin() } else { 0.0 };
    let mut pos = def.hip.lerp(def.ads, a);
    pos.x += bob.sin() * 0.005 * calm * sp * bobk + p.g_sway.x * 0.25;
    pos.y += -bob.cos().abs() * 0.0045 * calm * sp * bobk + p.g_sway.y * 0.25 - p.g_block * 0.03 - lower * 0.12 - sw * 0.2;
    pos.z += p.g_kick * 0.008 * def.kick + p.g_block * 0.06;
    let rx = p.g_kick * 0.07 * def.kick + p.g_sway.y * 0.6 + p.g_block * 0.8 - lower * 0.7 - sw * 0.5 + a * def.ads_pitch;
    let ry = p.g_sway.x * 0.8 + lower * 0.2 + a * def.ads_yaw;
    let mut rz = p.g_sway.x * -0.6 - p.lean * 0.05 * (1.0 - a);
    if p.reloading { let t = p.reload_t.clamp(0.0, 1.0); let k = (t * std::f32::consts::PI).sin(); pos.y -= k * 0.01; rz += k * 0.04; }
    if let Ok(mut tf) = gun.single_mut() {
        let rot = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        tf.rotation = rot;
        tf.translation = pos + def.wrist - rot * def.wrist;
    }
}

/// Clipes de uma arma já ligados ao esqueleto dela.
pub struct GunAnim {
    pub player: Entity, pub idle: AnimationNodeIndex, pub shoot: AnimationNodeIndex, pub reload: AnimationNodeIndex, pub reload_empty: AnimationNodeIndex,
    pub take: Option<AnimationNodeIndex>, pub hide: Option<AnimationNodeIndex>, pub walk: Option<AnimationNodeIndex>, pub run: Option<AnimationNodeIndex>,
    pub reload_len: f32, pub reload_empty_len: f32, pub take_len: f32, pub hide_len: f32,
    /// Sem clipe de parada: a pose de repouso é o primeiro quadro do clipe de tiro, congelado.
    pub idle_frozen: bool,
}
#[derive(Resource)]
pub struct GunAnims(pub Vec<Option<GunAnim>>);

/// Liga os clipes de animação de cada arma ao esqueleto dela assim que modelo e clipes carregam.
pub fn gun_anim_setup(mut commands: Commands, files: Res<GunGltfs>, gltfs: Res<Assets<Gltf>>, clips: Res<Assets<AnimationClip>>, mut graphs: ResMut<Assets<AnimationGraph>>,
    mut anims: ResMut<GunAnims>, mut players: Query<(Entity, &mut AnimationPlayer)>, parents: Query<&ChildOf>, models: Query<&GunModel>, names: Query<(Entity, &Name), Added<Name>>,
    skinned: Query<Entity, Added<bevy::render::mesh::skinning::SkinnedMesh>>) {
    for (e, n) in &names { if n.as_str().contains("Icosphere") { commands.entity(e).insert(Visibility::Hidden); } }
    // malhas com esqueleto têm caixa de recorte calculada na pose de repouso: sem isto elas somem da tela
    for e in &skinned { commands.entity(e).insert(bevy::render::view::NoFrustumCulling); }
    if anims.0.iter().all(|a| a.is_some()) { return; }
    for (e, mut pl) in &mut players {
        // sobe na hierarquia até achar de qual arma é este esqueleto (o operador do menu também tem um)
        let (mut cur, mut which) = (e, None);
        for _ in 0..24 {
            if let Ok(m) = models.get(cur) { which = Some(m.0); break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
        let Some(i) = which else { continue };
        if anims.0[i].is_some() { continue; }
        let w = &WEAPONS[i];
        let Some(g) = gltfs.get(&files.0[i]) else { continue };
        let get = |n: &str| if n.is_empty() { None } else { g.named_animations.get(n).cloned() };
        let (Some(shoot), Some(reload), Some(reload_empty)) = (get(w.shoot), get(w.reload), get(w.reload_empty)) else { continue };
        let len = |h: &Handle<AnimationClip>| clips.get(h).map(|c| c.duration()).unwrap_or(1.0);
        let (reload_len, reload_empty_len) = (len(&reload), len(&reload_empty));
        let (take, hide, walk, run, idle) = (get(w.take), get(w.hide), get(w.walk), get(w.run), get(w.idle));
        let (take_len, hide_len) = (take.as_ref().map(&len).unwrap_or(0.4), hide.as_ref().map(&len).unwrap_or(0.4));
        let idle_frozen = idle.is_none();
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        let idle_n = graph.add_clip(idle.unwrap_or_else(|| shoot.clone()), 1.0, root);
        let shoot_n = graph.add_clip(shoot, 1.0, root);
        let reload_n = graph.add_clip(reload, 1.0, root);
        let reload_empty_n = graph.add_clip(reload_empty, 1.0, root);
        let take_n = take.map(|h| graph.add_clip(h, 1.0, root));
        let hide_n = hide.map(|h| graph.add_clip(h, 1.0, root));
        let walk_n = walk.map(|h| graph.add_clip(h, 1.0, root));
        let run_n = run.map(|h| graph.add_clip(h, 1.0, root));
        // o clipe de parada entra pelo controle de transições: se entrasse direto no tocador ele nunca sairia e
        // ficaria misturado meio a meio com tiro, recarga e os demais
        let mut tr = AnimationTransitions::new();
        let a = tr.play(&mut pl, idle_n, std::time::Duration::ZERO);
        if idle_frozen { a.set_speed(0.0); } else { a.repeat(); }
        commands.entity(e).insert((AnimationGraphHandle(graphs.add(graph)), tr));
        anims.0[i] = Some(GunAnim { player: e, idle: idle_n, shoot: shoot_n, reload: reload_n, reload_empty: reload_empty_n, take: take_n, hide: hide_n, walk: walk_n, run: run_n,
            reload_len, reload_empty_len, take_len, hide_len, idle_frozen });
    }
}

#[derive(Default)]
pub struct GunDrive { shot_n: u32, reloading: bool, weapon: usize, phase: u8, fp: bool, cur: Option<AnimationNodeIndex>, once: bool }

/// Escolhe o clipe da arma em uso: guardar e sacar na troca (e ao voltar para a primeira pessoa), recarga tática ou de
/// emergência, tiro, e parada / passo / corrida quando a arma tem esses clipes.
pub fn gun_anim_drive(anims: Res<GunAnims>, p: Res<Player>, game: Res<Game>, mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>, mut st: Local<GunDrive>) {
    use std::time::Duration;
    let fp = game.mode == Mode::Play && game.view == View::Fp;
    let phase: u8 = match p.switch_to { Some(_) if p.switch_t < 0.5 => 1, Some(_) => 2, None => 0 };
    let (was_fp, was_phase, was_reloading, was_shot) = (st.fp, st.phase, st.reloading, st.shot_n);
    st.fp = fp; st.phase = phase; st.reloading = p.reloading; st.shot_n = p.shot_n;
    if st.weapon != p.weapon { st.weapon = p.weapon; st.cur = None; st.once = false; }
    let Some(Some(a)) = anims.0.get(p.weapon) else { return };
    let Ok((mut pl, mut tr)) = players.get_mut(a.player) else { return };
    let half = SWITCH_S * 0.5;
    let mut one: Option<(AnimationNodeIndex, f32)> = None;
    if phase != was_phase && phase == 1 { one = a.hide.map(|n| (n, a.hide_len / half)); }
    else if phase != was_phase && phase == 2 { one = a.take.map(|n| (n, a.take_len / half)); }
    else if fp && !was_fp && phase == 0 && !p.reloading { one = a.take.map(|n| (n, a.take_len / 0.45)); }
    else if p.reloading && !was_reloading { one = Some(if p.reload_empty { (a.reload_empty, a.reload_empty_len / p.reload_dur) } else { (a.reload, a.reload_len / p.reload_dur) }); }
    else if p.shot_n != was_shot && !p.reloading && phase == 0 { one = Some((a.shoot, 1.0)); }
    if let Some((n, speed)) = one {
        tr.play(&mut pl, n, Duration::from_millis(50)).set_speed(speed.max(0.05));
        st.cur = Some(n); st.once = true;
        return;
    }
    let busy = st.once && st.cur.and_then(|n| pl.animation(n)).map_or(false, |x| !x.is_finished());
    if busy || phase != 0 || p.reloading { return; }
    let want = match (a.run, a.walk) {
        (Some(r), _) if p.fast && p.speed > 2.4 => r,
        (_, Some(w)) if p.speed > 0.5 && !p.ads && fp => w,
        _ => a.idle,
    };
    if st.cur != Some(want) || st.once {
        let x = tr.play(&mut pl, want, Duration::from_millis(170));
        if want == a.idle && a.idle_frozen { x.set_speed(0.0); } else { x.repeat(); }
        st.cur = Some(want); st.once = false;
    }
}
