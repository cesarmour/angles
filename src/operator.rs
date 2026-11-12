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

/// Corpo 3D do operador do jogador nas vistas de cima (arquivo de onde saem os clipes).
#[derive(Component)] pub struct BodyScene(pub Handle<Gltf>);
/// Clipes do corpo do jogador: espera com a pistola baixa, mira e coice do disparo.
#[derive(Component)] pub struct BodyAnim { pub idle: AnimationNodeIndex, pub aim: Option<AnimationNodeIndex>, pub shoot: Option<AnimationNodeIndex>, pub walk: Option<AnimationNodeIndex>, pub walk_aim: Option<AnimationNodeIndex>,
    /// Estado em exibição: 0 espera, 1 mira, 2 andando, 3 andando com a arma apontada.
    pub mode: u8, pub recoil: f32, pub hot: f32, pub shot_n: u32 }
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
    /// Refém: braços abaixados, depois que o cômodo dela fica limpo.
    pub relax: Option<AnimationNodeIndex>,
    /// Quedas disponíveis (o jogo sorteia uma a cada vez) e tempo que resta do coice do disparo.
    pub falls: Vec<AnimationNodeIndex>, pub recoil: f32,
    /// Estado em exibição: 0 = espera, 1 = mira (ameaça) ou braços abaixados (refém), 2 = andando.
    pub mode: u8, pub walk: Option<AnimationNodeIndex>, pub frozen: bool }

pub fn setup_operator(mut commands: Commands, mut images: ResMut<Assets<Image>>, asset: Res<AssetServer>) {
    let mut img = Image::new_fill(Extent3d { width: 800, height: 1200, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Bgra8UnormSrgb, RenderAssetUsages::default());
    img.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let img = images.add(img);
    commands.insert_resource(PortraitTex(img.clone()));
    commands.spawn((SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset(MODEL))), Transform::from_translation(STUDIO), Visibility::default(), PortraitScene, IdleScene(asset.load(MODEL))));
    let layer = RenderLayers::layer(L_PORTRAIT);
    // enquadramento fechado, da coxa para cima
    let aim = STUDIO + Vec3::new(0.0, 1.30, 0.0);
    commands.spawn((
        Camera3d::default(),
        Camera { order: -2, is_active: false, target: RenderTarget::Image(img.into()), clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        Projection::Perspective(PerspectiveProjection { fov: 22f32.to_radians(), near: 0.2, far: 30.0, ..default() }),
        Tonemapping::TonyMcMapface, Msaa::Sample4, layer.clone(), Exposure { ev100: env_f("CQB_EVP", 9.3) },
        Transform::from_translation(STUDIO + Vec3::new(0.0, 1.42, 3.3)).looking_at(aim, Vec3::Y), PortraitCam,
    ));
    // luz principal de cima e de lado, preenchimento frio do outro lado e contraluz para recortar a silhueta escura do fundo
    let k = env_f("CQB_LP", 1.0);
    for (pos, lumens, color, outer) in [
        // principal quente pela frente e de lado (acende o rosto e a jaqueta), preenchimento fraco, dois recortes verde-azulados por trás
        (Vec3::new(2.6, 2.3, 2.6), 520_000.0, Color::srgb(1.0, 0.93, 0.82), 0.5),
        (Vec3::new(-2.6, 1.4, 2.4), 70_000.0, Color::srgb(0.7, 0.85, 1.0), 0.6),
        (Vec3::new(-2.2, 2.4, -2.6), 300_000.0, Color::srgb(0.35, 0.95, 0.8), 0.6),
        (Vec3::new(2.4, 2.6, -2.4), 170_000.0, Color::srgb(0.4, 0.9, 0.85), 0.6),
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
    if let Ok(mut tf) = scene.single_mut() {
        let t = time.elapsed_secs();
        // corpo de lado, quase de costas; o rosto (virado por cima do ombro no próprio modelo) fica voltado para a câmera
        tf.rotation = Quat::from_rotation_y(1.82 + (t * 0.2).sin() * 0.10);
    }
}

/// Onde a cabeça do boneco vai parar em cada queda ("Fall1" a "Fall4"), em metros, no espaço do modelo (+Z = para a frente dele).
/// Medido na conversão (tools3d/conv_operator.py imprime "FIMQUEDA"); serve para escolher um giro em que o corpo caiba no chão livre.
const FALL_REACH: [Vec3; 4] = [Vec3::new(-0.07, 0.0, 1.10), Vec3::new(1.02, 0.0, 0.0), Vec3::new(0.15, 0.0, 1.22), Vec3::new(0.03, 0.0, -1.19)];

/// Bonecos-alvo: ligam os clipes ao esqueleto, apontam o fuzil (clipe "Aim") e viram para o operador quando a ameaça o
/// tem na linha de tiro, e desabam (clipe "Fall") quando derrubados.
pub fn fig_system(time: Res<Time>, mut commands: Commands, gltfs: Res<Assets<Gltf>>, mut graphs: ResMut<Assets<AnimationGraph>>, targets: Res<Targets>, p: Res<Player>, tac: Res<crate::tactical::Tac>,
    mut fresh: Query<(Entity, &mut AnimationPlayer), (Without<IdleReady>, Without<FigAnim>)>, parents: Query<&ChildOf>,
    mut anims: Query<(&mut FigAnim, &mut AnimationPlayer, &mut AnimationTransitions)>, mut roots: Query<(Entity, &mut Fig, &mut Transform), Without<EnemyFlash>>, globals: Query<&GlobalTransform>,
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
            if let Ok((_, f, _)) = roots.get(cur) { fig = Some((f.target, f.file.clone())); break; }
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
        let relax = g.named_animations.get("Relax").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let walk = g.named_animations.get("Walk").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let mut names: Vec<&Box<str>> = g.named_animations.keys().filter(|k| k.starts_with("Fall")).collect();
        names.sort();
        let falls: Vec<AnimationNodeIndex> = names.into_iter().map(|k| graph.add_clip(g.named_animations[k].clone(), 1.0, root)).collect();
        // o clipe de espera entra pelo controle de transições (senão ficaria misturado meio a meio com o de mira);
        // cada boneco começa num ponto diferente do clipe, para não se mexerem todos juntos
        let mut tr = AnimationTransitions::new();
        tr.play(&mut pl, idle, Duration::ZERO).repeat().seek_to((e.index() % 7) as f32 * 1.07);
        commands.entity(e).insert((AnimationGraphHandle(graphs.add(graph)), tr, IdleReady, FigAnim { target, idle, aim, shoot, relax, falls, recoil: 0.0, mode: 0, walk, frozen: false }));
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
                    tr.play(&mut pl, n, Duration::from_millis(160)).set_speed(1.1 + rnd(now * 313.0 + a.target as f32 * 7.3) * 0.4);
                }
            }
            continue;
        }
        if a.frozen { pl.resume_all(); a.frozen = false; a.mode = 0; a.recoil = 0.0; tr.play(&mut pl, a.idle, Duration::ZERO).repeat(); }
        // disparo: coice curto e volta para a mira
        if shooters.contains(&a.target) { if let Some(n) = a.shoot { tr.play(&mut pl, n, Duration::from_millis(30)); a.recoil = 0.36; a.mode = 1; } }
        if a.recoil > 0.0 {
            a.recoil -= fdt;
            if a.recoil <= 0.0 { if let Some(n) = a.aim { tr.play(&mut pl, n, Duration::from_millis(90)).repeat(); } }
            continue;
        }
        // ameaça: mira quando tem o operador na linha de tiro e anda quando troca de posição. Refém: baixa os braços quando
        // o cômodo dela está limpo (área já vista e nenhuma ameaça viva lá dentro).
        let want: u8 = if a.aim.is_some() { if t.react > 0.04 { 1 } else if t.moving && a.walk.is_some() { 2 } else { 0 } } else if a.relax.is_some() {
            let zi = crate::map::zone_index(t.def.zone);
            (tac.pct.get(zi).map_or(false, |p| *p >= 0.9) && !targets.list.iter().any(|o| o.alive && !o.off && o.def.kind == crate::map::TKind::Threat && o.def.zone == t.def.zone)) as u8
        } else { 0 };
        if want != a.mode {
            a.mode = want;
            let n = match want { 1 => a.aim.or(a.relax).unwrap_or(a.idle), 2 => a.walk.unwrap_or(a.idle), _ => a.idle };
            let ms = if a.aim.is_some() { if want == 1 { 200 } else { 350 } } else { 900 };
            tr.play(&mut pl, n, Duration::from_millis(ms)).repeat();
        }
    }
    let dt = time.delta_secs().min(0.05);
    for (e, mut f, mut tf) in &mut roots {
        let Some(t) = targets.list.get(f.target) else { continue };
        if !t.alive {
            // ao cair, o corpo gira para um lado sorteado; se desse lado ele acabaria deitado dentro de uma parede ou de um
            // móvel, tenta outros giros até achar chão livre (a queda sorteada é a mesma do clipe: mesma semente)
            if !f.fell {
                f.fell = true;
                let reach = FALL_REACH[((rnd(now * 977.0 + f.target as f32 * 31.7) * FALL_REACH.len() as f32) as usize).min(FALL_REACH.len() - 1)];
                let base = (rnd(now * 191.0 + f.target as f32 * 3.1) - 0.5) * 1.3;
                let (cur, _, _) = tf.rotation.to_euler(EulerRot::YXZ);
                let mut best = (base, -1i32);
                if let Some(pg) = parents.get(e).ok().and_then(|c| globals.get(c.parent()).ok()) {
                    for off in [0.0, 0.8, -0.8, 1.6, -1.6, 2.4, -2.4, 3.14] {
                        let q = Quat::from_rotation_y(cur + base + off);
                        let free = [0.45, 0.75, 1.0].iter().filter(|k| {
                            let w = pg.transform_point(q * (reach * **k));
                            crate::tactical::Tac::cell(w.x, w.z).map_or(false, |c| tac.zone_idx[c] >= 0)
                        }).count() as i32;
                        if free > best.1 { best = (base + off, free); }
                        if free == 3 { break; }
                    }
                }
                tf.rotation = Quat::from_rotation_y(cur + best.0);
            }
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

/// Corpo do jogador nas vistas de cima: pistola baixa em espera; levanta a arma ao mirar ou logo depois de atirar; coice no disparo.
pub fn body_system(time: Res<Time>, mut commands: Commands, gltfs: Res<Assets<Gltf>>, mut graphs: ResMut<Assets<AnimationGraph>>, p: Res<Player>,
    mut fresh: Query<(Entity, &mut AnimationPlayer), (Without<IdleReady>, Without<BodyAnim>)>, parents: Query<&ChildOf>, scenes: Query<&BodyScene>,
    mut anims: Query<(&mut BodyAnim, &mut AnimationPlayer, &mut AnimationTransitions)>) {
    use std::time::Duration;
    for (e, mut pl) in &mut fresh {
        let (mut cur, mut file) = (e, None);
        for _ in 0..24 {
            if let Ok(s) = scenes.get(cur) { file = Some(s.0.clone()); break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
        let Some(g) = file.and_then(|h| gltfs.get(&h)) else { continue };
        let Some(idle_clip) = g.named_animations.get("Idle").cloned() else { continue };
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        let idle = graph.add_clip(idle_clip, 1.0, root);
        let aim = g.named_animations.get("Aim").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let shoot = g.named_animations.get("Shoot").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let walk = g.named_animations.get("Walk").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let walk_aim = g.named_animations.get("WalkAim").map(|h| graph.add_clip(h.clone(), 1.0, root));
        let mut tr = AnimationTransitions::new();
        tr.play(&mut pl, idle, Duration::ZERO).repeat();
        commands.entity(e).insert((AnimationGraphHandle(graphs.add(graph)), tr, IdleReady, BodyAnim { idle, aim, shoot, walk, walk_aim, mode: 0, recoil: 0.0, hot: 0.0, shot_n: p.shot_n }));
    }
    let dt = time.delta_secs();
    for (mut a, mut pl, mut tr) in &mut anims {
        if p.shot_n != a.shot_n {
            a.shot_n = p.shot_n; a.hot = 1.4;
            if let Some(n) = a.shoot { tr.play(&mut pl, n, Duration::from_millis(30)); a.recoil = 0.36; a.mode = 9; }
        }
        a.hot = (a.hot - dt).max(0.0);
        if a.recoil > 0.0 { a.recoil -= dt; if a.recoil > 0.0 { continue; } }
        let aiming = a.aim.is_some() && (p.ads || a.hot > 0.0);
        let moving = p.speed > 0.35 && a.walk.is_some();
        let want: u8 = match (moving, aiming) { (false, false) => 0, (false, true) => 1, (true, false) => 2, (true, true) => 3 };
        if want != a.mode {
            a.mode = want;
            let n = match want { 1 => a.aim.unwrap_or(a.idle), 2 => a.walk.unwrap_or(a.idle), 3 => a.walk_aim.or(a.walk).unwrap_or(a.idle), _ => a.idle };
            tr.play(&mut pl, n, Duration::from_millis(if want == 0 { 320 } else { 150 })).repeat();
        }
        // o passo acompanha a velocidade (o clipe foi feito para cerca de 1,35 m/s)
        if moving { let n = if want == 3 { a.walk_aim.or(a.walk) } else { a.walk }; if let Some(x) = n.and_then(|n| pl.animation_mut(n)) { x.set_speed((p.speed / 1.35).clamp(0.5, 2.6)); } }
    }
}

/// Cada refém ganha uma combinação própria de cor de blusa, calça e cabelo (a textura dessas peças é cinza claro), para
/// não serem todas iguais. A combinação muda de uma refém para outra e de uma partida do jogo para outra.
pub fn hostage_look(mut commands: Commands, mut mats: ResMut<Assets<StandardMaterial>>, parents: Query<&ChildOf>, figs: Query<&Fig>,
    fresh: Query<(Entity, &bevy::gltf::GltfMaterialName, &MeshMaterial3d<StandardMaterial>), Added<bevy::gltf::GltfMaterialName>>, mut seed: Local<Option<usize>>) {
    // (blusa, calça, cabelo)
    const LOOKS: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.30, 0.31, 0.34], [0.42, 0.55, 0.78], [0.62, 0.20, 0.10]),   // grafite, jeans, ruiva
        ([0.55, 0.12, 0.16], [0.12, 0.12, 0.14], [0.10, 0.09, 0.08]),   // vinho, preta, cabelo preto
        ([0.85, 0.62, 0.18], [0.40, 0.41, 0.44], [0.38, 0.24, 0.14]),   // mostarda, cinza, castanho
        ([0.12, 0.45, 0.48], [0.70, 0.62, 0.48], [0.88, 0.70, 0.38]),   // verde-azulado, bege, loira
        ([0.30, 0.36, 0.54], [0.16, 0.17, 0.20], [0.22, 0.14, 0.09]),   // azul-ardósia, grafite, castanho escuro (nada de branco: estoura na luz)
        ([0.32, 0.42, 0.22], [0.55, 0.66, 0.82], [0.48, 0.18, 0.09]),   // oliva, jeans claro, acaju
    ];
    let s = *seed.get_or_insert_with(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos() as usize).unwrap_or(0));
    for (e, name, mat) in &fresh {
        let part = match name.0.as_str() { "Top_Mat" => 0, "Pants_Mat" => 1, "hair_Transparency" | "scalp_Transparency" | "Front_Transparency" => 2, _ => continue };
        let (mut cur, mut who) = (e, None);
        for _ in 0..24 {
            if let Ok(f) = figs.get(cur) { if !f.threat { who = Some(f.target); } break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
        let Some(target) = who else { continue };
        let look = LOOKS[(target * 5 + s) % LOOKS.len()];
        let c = [look.0, look.1, look.2][part];
        let Some(mut m) = mats.get(&mat.0).cloned() else { continue };
        m.base_color = Color::srgb(c[0], c[1], c[2]);
        commands.entity(e).insert(MeshMaterial3d(mats.add(m)));
    }
}

/// Cômodo limpo: toda a área já vista e nenhuma ameaça viva lá dentro. Contam só os cômodos que têm ameaça ou refém.
/// A primeira vez que um cômodo fica limpo aparece o carimbo "ROOM CLEAR" (sem som).
pub fn room_clear_system(time: Res<Time>, mut game: ResMut<Game>, tac: Res<crate::tactical::Tac>, targets: Res<Targets>, mut sfx: EventWriter<Sfx>, mut met: Local<(u32, f32)>) {
    use crate::map::{zone_index, zones, TKind};
    if game.stamp_t > 0.0 { game.stamp_t -= time.delta_secs(); }
    if game.mode != Mode::Play { return; }
    // primeira vez que um refém aparece: o operador grita "Freeze!" (no máximo uma vez a cada 4 s)
    met.1 = (met.1 - time.delta_secs()).max(0.0);
    for (i, t) in targets.list.iter().enumerate().filter(|(_, t)| t.def.kind == TKind::Civil && !t.off) {
        let bit = 1u32 << (i as u32 % 32);
        if !t.seen { met.0 &= !bit; } else if met.0 & bit == 0 { met.0 |= bit; if met.1 <= 0.0 && t.alive { sfx.write(Sfx::Voice(0)); met.1 = 4.0; } }
    }
    let mut used = 0u32;
    for t in targets.list.iter().filter(|t| !t.off && !t.def.passive) { used |= 1 << zone_index(t.def.zone); }
    let rooms = used.count_ones();
    if rooms != game.rooms { game.rooms = rooms; game.ui_dirty = true; }
    for (zi, z) in zones().iter().enumerate() {
        if used & (1 << zi) == 0 || game.cleared & (1 << zi) != 0 { continue; }
        let hot = targets.list.iter().any(|t| t.alive && !t.off && !t.def.passive && t.def.kind == TKind::Threat && zone_index(t.def.zone) == zi);
        if !hot && tac.pct[zi] >= 0.9 {
            game.cleared |= 1 << zi;
            game.stamp = format!("ROOM CLEAR  \u{b7}  {}", z.name.to_uppercase());
            // sem fala: o dono do projeto pediu para tirar o som de "area cleared"; fica só o carimbo
            game.stamp_t = 2.6; game.ui_dirty = true;
        }
    }
}

/// Ameaças que trocam de posição dentro do cômodo quando ouvem tiro (só no percurso livre e na missão 2; no tutorial elas
/// ficam onde estão, para a lição continuar igual). Cada uma se muda no máximo duas vezes, para uma vaga livre do mesmo
/// cômodo, e para se já estiver de olho no operador.
pub fn threat_move_system(time: Res<Time>, game: Res<Game>, level: Res<crate::map::Level>, mut targets: ResMut<Targets>, mut heard: EventReader<Sfx>, mut roots: Query<&mut Transform, Without<Fig>>) {
    use crate::map::{slots, TKind};
    let dt = time.delta_secs().min(0.05);
    let shot = heard.read().filter(|e| matches!(e, Sfx::Shot | Sfx::Rifle | Sfx::EnemyShot(_))).count() > 0;
    if game.mode != Mode::Play { return; }
    let now = time.elapsed_secs();
    let rnd = |s: f32| (s.sin() * 43758.547).fract().abs();
    let n = targets.list.len();
    if shot && game.free && std::env::var("CQB_NOMOVE").is_err() {
        let mut taken: Vec<Vec2> = targets.list.iter().filter(|t| t.alive && !t.off).map(|t| Vec2::new(t.def.x, t.def.z)).collect();
        for t in targets.list.iter() { if let Some(g) = t.goal { taken.push(Vec2::new(slots()[g].1, slots()[g].2)); } }
        for i in 0..n {
            let pick = {
                let t = &targets.list[i];
                if !t.alive || t.off || t.def.passive || t.def.kind != TKind::Threat || t.goal.is_some() || t.moved >= 2 || t.move_cd > 0.0 || t.vis || t.react > 0.04 { None }
                else if rnd(now * 71.3 + i as f32 * 13.7) > 0.4 { None }
                else {
                    let here = Vec2::new(t.def.x, t.def.z);
                    let cands: Vec<usize> = slots().iter().enumerate().filter(|(_, s)| s.0 == t.def.zone && { let q = Vec2::new(s.1, s.2); q.distance(here) > 1.0 && taken.iter().all(|o| o.distance(q) > 0.7) }).map(|(k, _)| k).collect();
                    if cands.is_empty() { None } else { Some(cands[((rnd(now * 37.1 + i as f32 * 5.3) * cands.len() as f32) as usize).min(cands.len() - 1)]) }
                }
            };
            if let Some(g) = pick {
                taken.push(Vec2::new(slots()[g].1, slots()[g].2));
                let t = &mut targets.list[i];
                t.goal = Some(g); t.wait = 0.25 + rnd(now * 11.9 + i as f32) * 0.9; t.stuck = 0.0;
            }
        }
    }
    for i in 0..n {
        let t = &mut targets.list[i];
        if t.move_cd > 0.0 { t.move_cd -= dt; }
        let Some(g) = t.goal else { t.moving = false; continue };
        // derrubada, ou já de olho no operador: fica onde está
        if !t.alive || t.react > 0.25 { t.goal = None; t.moving = false; t.move_cd = 4.0; continue; }
        if t.wait > 0.0 { t.wait -= dt; continue; }
        let s = slots()[g];
        let (pos, goal) = (Vec2::new(t.def.x, t.def.z), Vec2::new(s.1, s.2));
        let d = goal - pos;
        let mut arrived = d.length() < 0.08;
        if !arrived {
            let dir = d.normalize();
            let mut np = pos + dir * (1.35 * dt).min(d.length());
            level.resolve_circle(&mut np, 0.28);
            if (np - pos).length() < 1.35 * dt * 0.3 { t.stuck += dt; } else { t.stuck = 0.0; }
            t.def.x = np.x; t.def.z = np.y; t.def.fx = np.x + dir.x; t.def.fz = np.y + dir.y; t.fwd = dir; t.moving = true;
            // travou num móvel: fica por ali mesmo, virada para onde ia
            if t.stuck > 0.7 { arrived = true; }
        }
        if arrived {
            if t.stuck <= 0.7 { t.def.x = s.1; t.def.z = s.2; t.def.fx = s.3; t.def.fz = s.4; }
            t.goal = None; t.moving = false; t.moved += 1; t.move_cd = 5.0; t.stuck = 0.0;
            crate::combat::place_target(&level, t);
        }
        let ry = (t.def.fx - t.def.x).atan2(t.def.fz - t.def.z);
        if let Ok(mut tf) = roots.get_mut(t.root) { tf.translation = Vec3::new(t.def.x, 0.0, t.def.z); tf.rotation = Quat::from_rotation_y(ry); }
    }
}
