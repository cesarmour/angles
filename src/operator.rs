//! Operador em 3D (modelo com esqueleto e animação de espera) mostrado no menu e na avaliação.
//! Fica num "estúdio" longe do mapa, com luzes e câmera próprias; a câmera desenha numa textura de fundo
//! transparente que a interface exibe por cima da cena.
use crate::state::*;
use crate::world::{env_f, L_PORTRAIT};
use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::camera::{Exposure, RenderTarget};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;

const STUDIO: Vec3 = Vec3::new(-300.0, 0.0, -300.0);
const MODEL: &str = "models/operator.glb";

/// Marca a cena do operador: as malhas dela vão para a camada do estúdio (ver `world::no_shadow_children`).
#[derive(Component)] pub struct PortraitScene;
#[derive(Component)] pub struct PortraitCam;
/// Textura em que a câmera do estúdio desenha.
#[derive(Resource)] pub struct PortraitTex(pub Handle<Image>);
/// Cena de um boneco com esqueleto que deve ficar tocando o primeiro clipe do arquivo em laço (operador do menu, reféns).
#[derive(Component)] pub struct IdleScene(pub Handle<Gltf>);
/// Esqueleto que já recebeu o clipe de espera.
#[derive(Component)] pub struct IdleReady;
/// Boneco 3D que faz o papel de um alvo do percurso (índice em `Targets`): ameaça armada de cáqui ou refém de branco.
#[derive(Component)] pub struct Fig { pub target: usize, pub file: Handle<Gltf>, pub threat: bool,
    /// Giro extra sorteado quando o boneco cai, para as quedas não ficarem todas viradas para o mesmo lado.
    pub fell: bool }
/// Clarão do disparo de uma ameaça (quadro luminoso + luz): some quando o tempo zera.
#[derive(Component)] pub struct EnemyFlash(pub f32);
/// Clipes de um boneco-alvo já ligados ao esqueleto dele.
#[derive(Component)] pub struct FigAnim { pub target: usize, pub idle: AnimationNodeIndex, pub aim: Option<AnimationNodeIndex>, pub shoot: Option<AnimationNodeIndex>,
    /// Quedas disponíveis (o jogo sorteia uma a cada vez) e tempo que resta do coice do disparo.
    pub falls: Vec<AnimationNodeIndex>, pub recoil: f32, pub aiming: bool, pub frozen: bool }

pub fn setup_operator(mut commands: Commands, mut images: ResMut<Assets<Image>>, asset: Res<AssetServer>) {
    let mut img = Image::new_fill(Extent3d { width: 800, height: 1200, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Bgra8UnormSrgb, RenderAssetUsages::default());
    img.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let img = images.add(img);
    commands.insert_resource(PortraitTex(img.clone()));
    commands.spawn((SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset(MODEL))), Transform::from_translation(STUDIO), Visibility::default(), PortraitScene, IdleScene(asset.load(MODEL))));
    let layer = RenderLayers::layer(L_PORTRAIT);
    let aim = STUDIO + Vec3::new(0.0, 0.98, 0.0);
    commands.spawn((
        Camera3d::default(),
        Camera { order: -2, is_active: false, target: RenderTarget::Image(img.into()), clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        Projection::Perspective(PerspectiveProjection { fov: 22f32.to_radians(), near: 0.2, far: 30.0, ..default() }),
        Tonemapping::TonyMcMapface, Msaa::Sample4, layer.clone(), Exposure { ev100: env_f("CQB_EVP", 9.3) },
        Transform::from_translation(STUDIO + Vec3::new(0.0, 1.12, 5.6)).looking_at(aim, Vec3::Y), PortraitCam,
    ));
    // luz principal de cima e de lado, preenchimento frio do outro lado e contraluz para recortar a silhueta escura do fundo
    let k = env_f("CQB_LP", 1.0);
    for (pos, lumens, color, outer) in [
        (Vec3::new(2.4, 3.0, 3.2), 620_000.0, Color::srgb(1.0, 0.96, 0.9), 0.5),
        (Vec3::new(-2.8, 1.6, 2.6), 190_000.0, Color::srgb(0.75, 0.86, 1.0), 0.6),
        (Vec3::new(-1.6, 2.6, -3.0), 260_000.0, Color::srgb(0.55, 0.85, 1.0), 0.6),
        (Vec3::new(2.0, 2.2, -2.8), 150_000.0, Color::srgb(1.0, 0.8, 0.55), 0.6),
    ] {
        commands.spawn((
            SpotLight { color, intensity: lumens * k, range: 12.0, radius: 0.05, shadows_enabled: false, outer_angle: outer, inner_angle: outer * 0.6, ..default() },
            Transform::from_translation(STUDIO + pos).looking_at(aim + Vec3::Y * 0.15, Vec3::Y), layer.clone(),
        ));
    }
}

/// Liga a animação de espera, gira o operador devagar e só desenha o estúdio enquanto menu ou avaliação estão na tela.
pub fn update_operator(time: Res<Time<Real>>, game: Res<Game>, mut commands: Commands, gltfs: Res<Assets<Gltf>>, mut graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<IdleReady>>, parents: Query<&ChildOf>, mut scene: Query<&mut Transform, With<PortraitScene>>, idle: Query<&IdleScene>,
    mut cam: Query<&mut Camera, With<PortraitCam>>) {
    let show = matches!(game.mode, Mode::Menu | Mode::Debrief);
    if let Ok(mut c) = cam.single_mut() { if c.is_active != show { c.is_active = show; } }
    // cada esqueleto novo (operador do menu, reféns) sobe na hierarquia até a cena dele e passa a tocar o clipe de espera
    for (e, mut pl) in &mut players {
        let (mut cur, mut file) = (e, None);
        for _ in 0..24 {
            if let Ok(s) = idle.get(cur) { file = Some(&s.0); break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
        let Some(g) = file.and_then(|h| gltfs.get(h)) else { continue };
        let Some(clip) = g.animations.first() else { continue };
        let (graph, node) = AnimationGraph::from_clip(clip.clone());
        commands.entity(e).insert((AnimationGraphHandle(graphs.add(graph)), IdleReady));
        // cada boneco começa num ponto diferente do clipe, para não se mexerem todos juntos
        pl.play(node).repeat().seek_to((e.index() % 7) as f32 * 1.07);
    }
    if !show { return; }
    // meia-volta lenta de um lado para o outro: mostra a frente, o perfil e o equipamento das costas
    if let Ok(mut tf) = scene.single_mut() {
        let t = time.elapsed_secs();
        tf.rotation = Quat::from_rotation_y(-0.45 + (t * 0.22).sin() * 0.95);
    }
}

/// Bonecos-alvo: ligam os clipes ao esqueleto, apontam o fuzil (clipe "Aim") e viram para o operador quando a ameaça o
/// tem na linha de tiro, e desabam (clipe "Fall") quando derrubados.
pub fn fig_system(time: Res<Time>, mut commands: Commands, gltfs: Res<Assets<Gltf>>, mut graphs: ResMut<Assets<AnimationGraph>>, targets: Res<Targets>, p: Res<Player>,
    mut fresh: Query<(Entity, &mut AnimationPlayer), (Without<IdleReady>, Without<FigAnim>)>, parents: Query<&ChildOf>,
    mut anims: Query<(&mut FigAnim, &mut AnimationPlayer, &mut AnimationTransitions)>, mut roots: Query<(&mut Fig, &mut Transform), Without<EnemyFlash>>,
    (mut shots, mut flashes, mut meshes, mut smats, mut images, mut art): (EventReader<Sfx>, Query<(Entity, &mut EnemyFlash, Option<&mut PointLight>)>, ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>, ResMut<Assets<Image>>, Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>)) {
    use std::time::Duration;
    // disparo de uma ameaça: clarão na boca do fuzil virado para o operador e uma luz curta que acende a sala
    let (quad, mat) = art.get_or_insert_with(|| {
        let tex = images.add(crate::art::svg_image(&crate::art::flash_svg(), 128, 128, false));
        (meshes.add(Rectangle::new(1.0, 1.0)), smats.add(StandardMaterial { base_color_texture: Some(tex), base_color: Color::srgb(6.0, 4.6, 2.4), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, ..default() }))
    }).clone();
    let now = time.elapsed_secs();
    let rnd = |s: f32| (s.sin() * 43758.547).fract().abs();
    let mut shooters: Vec<usize> = Vec::new();
    for ev in shots.read() {
        let Sfx::EnemyShot(src) = *ev else { continue };
        // quem atirou: a ameaça mais próxima da origem do disparo
        if let Some((i, _)) = targets.list.iter().enumerate().filter(|(_, t)| t.alive && t.def.kind == crate::map::TKind::Threat)
            .min_by(|a, b| Vec2::new(a.1.def.x - src.x, a.1.def.z - src.z).length().partial_cmp(&Vec2::new(b.1.def.x - src.x, b.1.def.z - src.z).length()).unwrap()) { shooters.push(i); }
        let to = Vec3::new(p.eye.x - src.x, 0.0, p.eye.z - src.z).normalize_or_zero();
        let mz = Vec3::new(src.x, 1.44, src.z) + to * 0.42;
        commands.spawn((Mesh3d(quad.clone()), MeshMaterial3d(mat.clone()), Transform::from_translation(mz).looking_at(p.eye, Vec3::Y).with_scale(Vec3::splat(0.55)), bevy::pbr::NotShadowCaster, EnemyFlash(0.075)));
        commands.spawn((PointLight { color: Color::srgb(1.0, 0.78, 0.5), intensity: 900_000.0, range: 9.0, shadows_enabled: false, ..default() }, Transform::from_translation(mz + to * 0.2), crate::world::light_layers(), EnemyFlash(0.11)));
    }
    let fdt = time.delta_secs();
    for (e, mut f, light) in &mut flashes {
        f.0 -= fdt;
        if let Some(mut l) = light { l.intensity = 900_000.0 * (f.0 / 0.11).clamp(0.0, 1.0); }
        if f.0 <= 0.0 { commands.entity(e).try_despawn(); }
    }
    for (e, mut pl) in &mut fresh {
        let (mut cur, mut fig) = (e, None);
        for _ in 0..24 {
            if let Ok((f, _)) = roots.get(cur) { fig = Some((f.target, f.file.clone())); break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
        let Some((target, file)) = fig else { continue };
        let Some(g) = gltfs.get(&file) else { continue };
        let Some(idle_clip) = g.named_animations.get("Idle").cloned().or_else(|| g.animations.first().cloned()) else { continue };
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        let idle = graph.add_clip(idle_clip, 1.0, root);
        let aim = g.named_animations.get("Aim").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let shoot = g.named_animations.get("Shoot").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let mut names: Vec<&Box<str>> = g.named_animations.keys().filter(|k| k.starts_with("Fall")).collect();
        names.sort();
        let falls: Vec<AnimationNodeIndex> = names.into_iter().map(|k| graph.add_clip(g.named_animations[k].clone(), 1.0, root)).collect();
        // o clipe de espera entra pelo controle de transições (senão ficaria misturado meio a meio com o de mira);
        // cada boneco começa num ponto diferente do clipe, para não se mexerem todos juntos
        let mut tr = AnimationTransitions::new();
        tr.play(&mut pl, idle, Duration::ZERO).repeat().seek_to((e.index() % 7) as f32 * 1.07);
        commands.entity(e).insert((AnimationGraphHandle(graphs.add(graph)), tr, IdleReady, FigAnim { target, idle, aim, shoot, falls, recoil: 0.0, aiming: false, frozen: false }));
    }
    for (mut a, mut pl, mut tr) in &mut anims {
        let Some(t) = targets.list.get(a.target) else { continue };
        // derrubado: toca uma das quedas, sorteada, em velocidade também sorteada, e fica na última pose
        if !t.alive {
            if !a.frozen {
                a.frozen = true;
                if a.falls.is_empty() { pl.pause_all(); } else {
                    let r = rnd(now * 977.0 + a.target as f32 * 31.7);
                    let n = a.falls[((r * a.falls.len() as f32) as usize).min(a.falls.len() - 1)];
                    tr.play(&mut pl, n, Duration::from_millis(70)).set_speed(0.8 + rnd(now * 313.0 + a.target as f32 * 7.3) * 0.45);
                }
            }
            continue;
        }
        if a.frozen { pl.resume_all(); a.frozen = false; a.aiming = false; a.recoil = 0.0; tr.play(&mut pl, a.idle, Duration::ZERO).repeat(); }
        // disparo: coice curto e volta para a mira
        if shooters.contains(&a.target) { if let Some(n) = a.shoot { tr.play(&mut pl, n, Duration::from_millis(30)); a.recoil = 0.36; a.aiming = true; } }
        if a.recoil > 0.0 {
            a.recoil -= fdt;
            if a.recoil <= 0.0 { if let Some(n) = a.aim { tr.play(&mut pl, n, Duration::from_millis(90)).repeat(); } }
            continue;
        }
        let want = a.aim.is_some() && t.react > 0.04;
        if want != a.aiming {
            a.aiming = want;
            let n = if want { a.aim.unwrap_or(a.idle) } else { a.idle };
            tr.play(&mut pl, n, Duration::from_millis(if want { 200 } else { 450 })).repeat();
        }
    }
    let dt = time.delta_secs().min(0.05);
    for (mut f, mut tf) in &mut roots {
        let Some(t) = targets.list.get(f.target) else { continue };
        if !t.alive {
            // ao cair, o corpo gira um pouco para um lado sorteado
            if !f.fell { f.fell = true; tf.rotation = tf.rotation * Quat::from_rotation_y((rnd(now * 191.0 + f.target as f32 * 3.1) - 0.5) * 1.3); }
            continue;
        }
        f.fell = false;
        let mut want = 0.0;
        if f.threat && t.react > 0.04 {
            let dv = p.pos - Vec2::new(t.def.x, t.def.z);
            want = (dv.x * t.fwd.y - dv.y * t.fwd.x).atan2(dv.dot(t.fwd));
        }
        let (cur, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
        tf.rotation = Quat::from_rotation_y(crate::map::damp(cur, want, 7.0, dt));
    }
}
