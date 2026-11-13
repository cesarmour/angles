//! Cenário: piso, paredes, móveis, letreiros e iluminação.
use crate::art;
use crate::map::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageLoaderSettings;
use bevy::pbr::{FogVolume, NotShadowCaster, NotShadowReceiver, VolumetricLight};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::view::RenderLayers;

/// Camadas: 0 mundo · 1 só primeira pessoa (forro, vergas) · 2 corpo do operador · 3 só vistas de cima · 4 só para sombras
pub const L_FP: usize = 1;
pub const L_BODY: usize = 2;
pub const L_TOP: usize = 3;
pub const L_SHADOW: usize = 4;
/// Camada do "estúdio" do operador 3D do menu: só a câmera e as luzes de lá a enxergam.
pub const L_PORTRAIT: usize = 5;
pub fn light_layers() -> RenderLayers { RenderLayers::from_layers(&[0, L_FP, L_BODY, L_SHADOW]) }

#[derive(Clone, Copy)]
pub struct B3 { pub x1: f32, pub y1: f32, pub z1: f32, pub x2: f32, pub y2: f32, pub z2: f32 }
pub fn b3(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> B3 { B3 { x1, y1, z1, x2, y2, z2 } }

/// Malha única com várias caixas e UV em metros do mundo (densidade de textura uniforme).
pub fn boxes_mesh(list: &[B3], s: f32) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let mut face = |v: [[f32; 3]; 4], n: [f32; 3]| {
        let base = pos.len() as u32;
        for p in v {
            pos.push(p); nor.push(n);
            uv.push(if n[0] != 0.0 { [p[2] * s, -p[1] * s] } else if n[2] != 0.0 { [p[0] * s, -p[1] * s] } else { [p[0] * s, p[2] * s] });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for b in list {
        let (x1, x2, y1, y2, z1, z2) = (b.x1, b.x2, b.y1, b.y2, b.z1, b.z2);
        face([[x2, y1, z2], [x2, y1, z1], [x2, y2, z1], [x2, y2, z2]], [1.0, 0.0, 0.0]);
        face([[x1, y1, z1], [x1, y1, z2], [x1, y2, z2], [x1, y2, z1]], [-1.0, 0.0, 0.0]);
        face([[x1, y2, z2], [x2, y2, z2], [x2, y2, z1], [x1, y2, z1]], [0.0, 1.0, 0.0]);
        face([[x1, y1, z1], [x2, y1, z1], [x2, y1, z2], [x1, y1, z2]], [0.0, -1.0, 0.0]);
        face([[x1, y1, z2], [x2, y1, z2], [x2, y2, z2], [x1, y2, z2]], [0.0, 0.0, 1.0]);
        face([[x2, y1, z1], [x1, y1, z1], [x1, y2, z1], [x2, y2, z1]], [0.0, 0.0, -1.0]);
    }
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_indices(Indices::U32(idx));
    let _ = m.generate_tangents();
    m
}

#[derive(Resource, Clone)]
pub struct Mats {
    pub floor: Handle<StandardMaterial>, pub wall_hi: Handle<StandardMaterial>, pub wall_lo: Handle<StandardMaterial>, pub ceil: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>, pub crate_: Handle<StandardMaterial>, pub frame: Handle<StandardMaterial>, pub door: Handle<StandardMaterial>,
    pub locker: Handle<StandardMaterial>, pub barrel: Handle<StandardMaterial>, pub fabric: Handle<StandardMaterial>, pub steel: Handle<StandardMaterial>,
    pub gun: Handle<StandardMaterial>, pub poly: Handle<StandardMaterial>, pub glove: Handle<StandardMaterial>, pub sleeve: Handle<StandardMaterial>,
    pub brass: Handle<StandardMaterial>, pub cloth: Handle<StandardMaterial>, pub gear: Handle<StandardMaterial>, pub invisible: Handle<StandardMaterial>,
    /// Kill house de madeira: chapa de OSB (textura do Poly Haven, duas chapas de 1,22 m por repetição) e sarrafo da estrutura.
    pub ply: Handle<StandardMaterial>, pub lumber: Handle<StandardMaterial>,
}

fn tex(asset: &AssetServer, name: &str, kind: &str, srgb: bool) -> Handle<Image> {
    asset.load_with_settings(format!("tex/{name}_{kind}.jpg"), move |s: &mut ImageLoaderSettings| { s.is_srgb = srgb; s.sampler = art::sampler(true); })
}
fn pbr(asset: &AssetServer, name: &str, tint: Color, rough: f32, metal: f32) -> StandardMaterial {
    let arm = tex(asset, name, "arm", false);
    StandardMaterial {
        base_color: tint,
        base_color_texture: Some(tex(asset, name, "diff", true)),
        normal_map_texture: Some(tex(asset, name, "nor", false)),
        metallic_roughness_texture: Some(arm.clone()),
        occlusion_texture: Some(arm),
        perceptual_roughness: rough, metallic: metal,
        ..default()
    }
}
fn flat(color: Color, rough: f32, metal: f32) -> StandardMaterial { StandardMaterial { base_color: color, perceptual_roughness: rough, metallic: metal, ..default() } }

pub fn build_mats(asset: &AssetServer, mats: &mut Assets<StandardMaterial>) -> Mats {
    Mats {
        floor: mats.add(StandardMaterial { reflectance: 0.5, ..pbr(asset, "concrete_floor_worn_001", Color::srgb(0.78, 0.77, 0.74), env_f("CQB_FLOOR_R", 0.7), 1.0) }),
        wall_hi: mats.add(pbr(asset, "concrete_block_wall", Color::linear_rgb(1.9, 1.84, 1.66), 1.0, 1.0)),
        wall_lo: mats.add(pbr(asset, "concrete_block_wall", Color::linear_rgb(0.62, 0.8, 0.68), 0.95, 1.0)),
        ceil: mats.add(pbr(asset, "concrete_wall_008", Color::srgb(0.30, 0.30, 0.32), 1.0, 1.0)),
        wood: mats.add(pbr(asset, "plywood", Color::srgb(0.95, 0.9, 0.82), 1.0, 1.0)),
        crate_: mats.add(pbr(asset, "plywood", Color::srgb(0.42, 0.47, 0.30), 1.0, 1.0)),
        frame: mats.add(pbr(asset, "metal_plate", Color::srgb(0.16, 0.17, 0.18), 0.9, 1.0)),
        door: mats.add(pbr(asset, "blue_metal_plate", Color::srgb(0.72, 0.78, 0.85), 1.0, 1.0)),
        locker: mats.add(pbr(asset, "green_metal_rust", Color::srgb(0.85, 0.9, 0.8), 1.0, 1.0)),
        barrel: mats.add(pbr(asset, "rusty_painted_metal", Color::srgb(0.45, 0.62, 0.95), 1.0, 1.0)),
        fabric: mats.add(flat(Color::srgb(0.20, 0.17, 0.15), 0.97, 0.0)),
        steel: mats.add(flat(Color::srgb(0.62, 0.64, 0.66), 0.35, 1.0)),
        gun: mats.add(StandardMaterial { reflectance: 0.4, ..flat(Color::srgb(0.035, 0.037, 0.04), 0.48, 0.55) }),
        poly: mats.add(flat(Color::srgb(0.045, 0.046, 0.05), 0.7, 0.0)),
        glove: mats.add(flat(Color::srgb(0.035, 0.035, 0.032), 0.9, 0.0)),
        sleeve: mats.add(flat(Color::srgb(0.13, 0.15, 0.10), 0.98, 0.0)),
        brass: mats.add(flat(Color::srgb(0.79, 0.64, 0.30), 0.3, 1.0)),
        cloth: mats.add(flat(Color::srgb(0.20, 0.23, 0.15), 0.95, 0.0)),
        gear: mats.add(flat(Color::srgb(0.07, 0.08, 0.07), 0.85, 0.0)),
        invisible: mats.add(flat(Color::BLACK, 1.0, 0.0)),
        ply: mats.add(pbr(asset, "killosb", Color::linear_rgb(env_f("CQB_PLY", 1.0), env_f("CQB_PLY", 1.0), env_f("CQB_PLY", 1.0)), 1.0, 1.0)),
        lumber: mats.add(flat(Color::srgb(0.88, 0.77, 0.57), 0.85, 0.0)),
    }
}

/// Tudo o que pertence ao mapa atual (paredes, móveis, portas, luzes, placas): some quando o mapa é trocado.
#[derive(Component)]
pub struct MapEntity;

fn sb(commands: &mut Commands, meshes: &mut Assets<Mesh>, list: &[B3], s: f32, mat: &Handle<StandardMaterial>) -> Entity {
    commands.spawn((Mesh3d(meshes.add(boxes_mesh(list, s))), MeshMaterial3d(mat.clone()), Transform::IDENTITY, MapEntity)).id()
}
#[allow(clippy::too_many_arguments)]
fn sign(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, images: &mut Assets<Image>, lines: &[(&str, f32)], w: f32, h: f32, pos: Vec3, ry: f32, color: &str, fp: bool) {
    let px_w = 512.0;
    let px_h = (512.0 * h / w).round();
    let img = images.add(art::svg_image(&art::sign_svg(lines, px_w, px_h, color), px_w as u32, px_h as u32, true));
    let mat = mats.add(StandardMaterial { base_color_texture: Some(img), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.92, depth_bias: 8.0, ..default() });
    let mut e = commands.spawn((Mesh3d(meshes.add(Rectangle::new(w, h))), MeshMaterial3d(mat), Transform::from_translation(pos).with_rotation(Quat::from_rotation_y(ry)), NotShadowCaster, MapEntity));
    if fp { e.insert(RenderLayers::layer(L_FP)); }
}
/// Gera mipmaps para as texturas carregadas de arquivo.
pub fn mip_loaded(mut ev: EventReader<AssetEvent<Image>>, mut images: ResMut<Assets<Image>>) {
    let ids: Vec<AssetId<Image>> = ev.read().filter_map(|e| if let AssetEvent::LoadedWithDependencies { id } = e { Some(*id) } else { None }).collect();
    for id in ids {
        let need = images.get(id).map_or(false, |i| i.texture_descriptor.mip_level_count == 1 && i.data.is_some() && i.texture_descriptor.size.width >= 256);
        if need { if let Some(img) = images.get_mut(id) { art::add_mips(img); } }
    }
}

#[derive(Component)]
pub struct CeilLight { pub idx: usize, pub base: f32 }
#[derive(Resource, Default)]
pub struct Tubes(pub Vec<(Handle<StandardMaterial>, LinearRgba)>);
#[derive(Component)]
pub struct DoorLeaf(pub usize);
/// Marca uma cena cujas malhas não devem projetar sombra.
#[derive(Component)]
pub struct NoShadowScene;

pub fn no_shadow_children(mut commands: Commands, added: Query<Entity, Added<Mesh3d>>, parents: Query<&ChildOf>, marked: Query<(), With<NoShadowScene>>, portrait: Query<(), With<crate::operator::PortraitScene>>, body: Query<(), With<crate::operator::BodyScene>>) {
    for e in &added {
        let mut cur = e;
        for _ in 0..24 {
            if portrait.get(cur).is_ok() { commands.entity(e).insert((NotShadowCaster, RenderLayers::layer(L_PORTRAIT))); break; }
            if body.get(cur).is_ok() { commands.entity(e).insert((NotShadowCaster, RenderLayers::layer(L_BODY))); break; }
            if marked.get(cur).is_ok() { commands.entity(e).insert((NotShadowCaster, RenderLayers::layer(L_FP))); break; }
            match parents.get(cur) { Ok(p) => cur = p.parent(), Err(_) => break }
        }
    }
}

/// Ajusta materiais de modelos importados que estouram sob as luminárias (o tampo claro da mesa ficava branco visto de cima).
pub fn tame_materials(mut ev: EventReader<AssetEvent<StandardMaterial>>, mut mats: ResMut<Assets<StandardMaterial>>, asset: Res<AssetServer>) {
    let ids: Vec<AssetId<StandardMaterial>> = ev.read().filter_map(|e| if let AssetEvent::Added { id } = e { Some(*id) } else { None }).collect();
    for id in ids {
        let Some(path) = asset.get_path(id) else { continue };
        // operador do menu: tecido preto sem o brilho de plástico que a luz de estúdio dava
        let name = path.path().to_string_lossy();
        if name.contains("operator.glb") || name.contains("threat.glb") || name.contains("player.glb") { if let Some(m) = mats.get_mut(id) { m.reflectance = 0.16; } }
        if name.contains("hostage.glb") { if let Some(m) = mats.get_mut(id) { m.reflectance = 0.25; } }
        if name.contains("walker.glb") { if let Some(m) = mats.get_mut(id) { m.reflectance = 0.2; } }
        if path.path().to_string_lossy().contains("metal_office_desk") {
            if let Some(m) = mats.get_mut(id) { m.base_color = Color::srgb(0.3, 0.31, 0.32); m.perceptual_roughness = 1.0; m.metallic = 0.0; m.metallic_roughness_texture = None; m.reflectance = 0.15; }
        }
    }
}

pub const CEIL_LUMENS: f32 = 1_300_000.0;
/// Ajuste por variável de ambiente, para calibrar luz sem recompilar.
pub fn env_f(name: &str, default: f32) -> f32 { std::env::var(name).ok().and_then(|s| s.parse().ok()).unwrap_or(default) }
pub const TUBE_NITS: f32 = 3_500.0;

pub fn setup_world(
    mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>,
    asset: Res<AssetServer>, level: Res<Level>,
) {
    let m = build_mats(&asset, &mut mats);
    spawn_map(&mut commands, &mut meshes, &mut mats, &mut images, &asset, &level, &m);
    commands.insert_resource(m);
}

/// Troca de mapa: desmonta o cenário e os alvos do mapa anterior e monta os do novo.
#[allow(clippy::too_many_arguments)]
pub fn map_switch(
    mut req: ResMut<crate::state::MapReq>, mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>,
    asset: Res<AssetServer>, mut level: ResMut<Level>, m: Res<Mats>, art: Res<crate::combat::TargetArt>, mut targets: ResMut<crate::state::Targets>, mut tac: ResMut<crate::tactical::Tac>,
    mut p: ResMut<crate::state::Player>, old: Query<Entity, With<MapEntity>>,
) {
    let Some(i) = req.0.take() else { return };
    if i >= MAPS.len() { return; }
    // mesmo mapa: só os alvos são refeitos (troca de dificuldade: bonecos ou papelão)
    if i == map_index() {
        for t in targets.list.drain(..) { for mk in t.marks { commands.entity(mk).try_despawn(); } commands.entity(t.root).try_despawn(); }
        targets.list = crate::combat::spawn_targets(&mut commands, &art, &level);
        return;
    }
    set_map(i);
    *level = Level::new();
    for e in &old { commands.entity(e).try_despawn(); }
    for t in targets.list.drain(..) { for mk in t.marks { commands.entity(mk).try_despawn(); } commands.entity(t.root).try_despawn(); }
    spawn_map(&mut commands, &mut meshes, &mut mats, &mut images, &asset, &level, &m);
    targets.list = crate::combat::spawn_targets(&mut commands, &art, &level);
    tac.rebuild(&level);
    // o operador vai para o ponto de partida do mapa novo (o menu mostra o mapa escolhido ao fundo)
    let st = map().start;
    *p = crate::state::Player::default();
    p.pos = Vec2::new(st.0, st.1); p.yaw = st.2; p.eye = Vec3::new(st.0, EYE_STAND, st.1); p.zone = zone_at(st.0, st.1).unwrap_or(0);
    let errs = level.validate();
    if errs.is_empty() { info!("mapa {} ({}): dados conferidos", i, map().name); } else { for e in &errs { warn!("mapa {}: {}", i, e); } }
    if std::env::var("CQB_SCRIPT").is_ok() { println!("BOT MAPA {} {}: {} problema(s) {:?}", i, map().name, errs.len(), errs); }
}

/// Largura da chapa de compensado da kill house.
const SHEET: f32 = 1.22;
/// Escala da textura de OSB (Poly Haven, 2,5 m de chapa fotografada): duas chapas de 1,22 m por repetição, com a junta
/// de cada chapa escurecida na própria textura (`tools/prep_osb.py`).
const OSB_UV: f32 = 1.0 / (2.0 * SHEET);

/// Estrutura de sarrafo aparente das paredes da kill house, como numa parede de OSB de verdade: só de um lado (o outro
/// lado é a chapa lisa, com as juntas à vista). Guias em cima e embaixo, montantes em pé a cada meia chapa e uma travessa
/// a meia altura. Os montantes ficam 2 mm à frente das travessas, para nenhuma face coincidir com outra.
fn framing(walls: &[crate::map::Solid]) -> Vec<B3> {
    let mut v = Vec::new();
    let inside = |x: f32, z: f32, skip: usize| walls.iter().enumerate().any(|(j, w)| j != skip && x > w.x1 && x < w.x2 && z > w.z1 && z < w.z2);
    for (i, w) in walls.iter().enumerate() {
        let along_x = w.x2 - w.x1 >= w.z2 - w.z1;
        let (a, b, _n0, n1) = if along_x { (w.x1, w.x2, w.z1, w.z2) } else { (w.z1, w.z2, w.x1, w.x2) };
        if b - a < 0.25 { continue; }
        let (face, sgn) = (n1, 1.0f32);
        // ponta enterrada em outra parede (canto de cômodo): a face visível começa depois da meia espessura dela
        let buried = |u: f32| { let f = face + sgn * 0.01; if along_x { inside(u, f, i) } else { inside(f, u, i) } };
        let ua = if buried(a + 0.02) { a + WALL_T / 2.0 } else { a };
        let ub = if buried(b - 0.02) { b - WALL_T / 2.0 } else { b };
        if ub - ua < 0.2 { continue; }
        let mut put = |u0: f32, u1: f32, y0: f32, y1: f32, depth: f32| {
            let (f0, f1) = (face, face + depth);
            v.push(if along_x { b3(u0, y0, f0, u1, y1, f1) } else { b3(f0, y0, u0, f1, y1, u1) });
        };
        put(ua, ub, 0.001, 0.045, 0.038);
        put(ua, ub, SHEET - 0.0225, SHEET + 0.0225, 0.038);
        put(ua, ub, WALL_H - 0.047, WALL_H - 0.002, 0.038);
        put(ua, ua + 0.045, 0.0, WALL_H, 0.04);
        put(ub - 0.045, ub, 0.0, WALL_H, 0.04);
        // montantes do meio: a cada meia chapa (61 cm), em posições fixas no mundo
        let step = SHEET / 2.0;
        let mut k = ((ua + 0.2) / step).ceil();
        loop {
            let u = k * step;
            if u > ub - 0.2 { break; }
            put(u - 0.0225, u + 0.0225, 0.0, WALL_H, 0.04);
            k += 1.0;
        }
    }
    v
}

/// Monta o cenário do mapa atual: piso, paredes, batentes, móveis, portas, placas e luminárias.
#[allow(clippy::too_many_arguments)]
pub fn spawn_map(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, images: &mut Assets<Image>, asset: &AssetServer, level: &Level, m: &Mats) {
    // piso e forro
    // (com o vão da escada do andar de cima aberto, quando o mapa tem um)
    let floor: Vec<B3> = match map().pit {
        // o piso para 3 cm antes do vão: esse espaço é do forro do vão (duas faces no mesmo plano piscam)
        Some((x1, z1, x2, z2, _)) => vec![b3(MAP_X1, -0.1, MAP_Z1, MAP_X2, 0.0, z1 - 0.03), b3(MAP_X1, -0.1, z2 + 0.03, MAP_X2, 0.0, MAP_Z2), b3(MAP_X1, -0.1, z1 - 0.03, x1 - 0.03, 0.0, z2 + 0.03), b3(x2, -0.1, z1 - 0.03, MAP_X2, 0.0, z2 + 0.03)],
        None => vec![b3(MAP_X1, -0.1, MAP_Z1, MAP_X2, 0.0, MAP_Z2)],
    };
    sb(&mut *commands, &mut *meshes, &floor, 0.4, &m.floor);
    let ce = sb(&mut *commands, &mut *meshes, &[b3(MAP_X1, WALL_H, MAP_Z1, MAP_X2, WALL_H + 0.1, MAP_Z2)], 0.4, &m.ceil);
    commands.entity(ce).insert((RenderLayers::layer(L_FP), NotShadowCaster));
    // paredes em duas faixas de pintura
    let band = 1.05;
    let lo: Vec<B3> = level.walls.iter().map(|w| b3(w.x1, 0.0, w.z1, w.x2, band, w.z2)).collect();
    let hi: Vec<B3> = level.walls.iter().map(|w| b3(w.x1, band, w.z1, w.x2, WALL_H, w.z2)).collect();
    // a casa é uma kill house de madeira: compensado de cima a baixo, sem faixa de pintura
    // todos os mapas são a mesma kill house: paredes de OSB com sarrafo aparente
    let ply = true;
    let (w_lo, w_hi) = if ply { (&m.ply, &m.ply) } else { (&m.wall_lo, &m.wall_hi) };
    // cada repetição da textura é uma chapa de 1,22 m: as juntas das chapas caem sempre no mesmo lugar do mundo
    let ws = if ply { OSB_UV } else { 1.0 };
    sb(&mut *commands, &mut *meshes, &lo, ws, w_lo);
    sb(&mut *commands, &mut *meshes, &hi, ws, w_hi);
    if ply { sb(&mut *commands, &mut *meshes, &framing(&level.walls), 1.0, &m.lumber); }
    let lint: Vec<B3> = level.lintels.iter().map(|w| b3(w.x1, DOOR_H, w.z1, w.x2, WALL_H, w.z2)).collect();
    let e = sb(&mut *commands, &mut *meshes, &lint, ws, w_hi);
    commands.entity(e).insert(RenderLayers::layer(L_FP));
    // a luz vem de cima do forro (mais uniforme); este bloqueador só existe para as sombras
    let blk: Vec<B3> = level.walls.iter().chain(level.lintels.iter()).map(|w| b3(w.x1, WALL_H, w.z1, w.x2, 4.9, w.z2)).collect();
    let e = sb(&mut *commands, &mut *meshes, &blk, 1.0, &m.invisible);
    commands.entity(e).insert((RenderLayers::layer(L_SHADOW), NotShadowReceiver));
    // batentes
    let (mut jambs, mut heads) = (Vec::new(), Vec::new());
    for g in &level.gaps {
        let (t, w) = (WALL_T / 2.0 + 0.025, 0.05);
        if g.horizontal {
            jambs.push(b3(g.a, 0.0, g.c - t, g.a + w, DOOR_H, g.c + t)); jambs.push(b3(g.b - w, 0.0, g.c - t, g.b, DOOR_H, g.c + t));
            heads.push(b3(g.a, DOOR_H - w, g.c - t, g.b, DOOR_H + 0.01, g.c + t));
        } else {
            jambs.push(b3(g.c - t, 0.0, g.a, g.c + t, DOOR_H, g.a + w)); jambs.push(b3(g.c - t, 0.0, g.b - w, g.c + t, DOOR_H, g.b));
            heads.push(b3(g.c - t, DOOR_H - w, g.a, g.c + t, DOOR_H + 0.01, g.b));
        }
    }
    let jm = if ply { &m.lumber } else { &m.frame };
    sb(&mut *commands, &mut *meshes, &jambs, 1.0, jm);
    let e = sb(&mut *commands, &mut *meshes, &heads, 1.0, jm);
    commands.entity(e).insert(RenderLayers::layer(L_FP));

    // mobília: modelos 3D do Poly Haven (CC0); a barricada de compensado continua sendo construída aqui
    let model = |commands: &mut Commands, name: &str, pos: Vec3, ry: f32, scale: Vec3, off: Vec3| {
        let rot = Quat::from_rotation_y(ry);
        commands.spawn((SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset(format!("models/ph/{name}/{name}.gltf")))),
            Transform { translation: pos - rot * (off * scale), rotation: rot, scale }, MapEntity));
    };
    let hp2 = std::f32::consts::FRAC_PI_2;
    for p in &level.props {
        let (w, d, mx, mz) = (p.x2 - p.x1, p.z2 - p.z1, (p.x1 + p.x2) / 2.0, (p.z1 + p.z2) / 2.0);
        let c = Vec3::new(mx, 0.0, mz);
        match p.kind {
            PropKind::Crate => model(&mut *commands, "wooden_military_crate", c, if d > w { hp2 } else { 0.0 }, Vec3::ONE, Vec3::ZERO),
            PropKind::Barrel => model(&mut *commands, "Barrel_01", c, mx * 2.3, Vec3::splat(0.93), Vec3::ZERO),
            // kill house: nada de mobília de casa. O que era bancada, mesa, sofá ou cama vira uma barricada baixa de OSB
            // com tampa de sarrafo; o que era estante ou armário vira um painel alto de OSB em quadro de sarrafo. O tamanho,
            // a altura e a colisão continuam os mesmos (o que cada peça ensina, cobertura baixa ou espaço morto, não muda)
            PropKind::Bench | PropKind::Table | PropKind::Sofa | PropKind::Bed => {
                sb(&mut *commands, &mut *meshes, &[b3(p.x1 + 0.02, 0.0, p.z1 + 0.02, p.x2 - 0.02, p.h - 0.04, p.z2 - 0.02)], OSB_UV, &m.ply);
                sb(&mut *commands, &mut *meshes, &[b3(p.x1, p.h - 0.04, p.z1, p.x2, p.h, p.z2),
                    b3(p.x1, 0.0, p.z1, p.x1 + 0.045, p.h - 0.041, p.z1 + 0.045), b3(p.x2 - 0.045, 0.0, p.z1, p.x2, p.h - 0.041, p.z1 + 0.045),
                    b3(p.x1, 0.0, p.z2 - 0.045, p.x1 + 0.045, p.h - 0.041, p.z2), b3(p.x2 - 0.045, 0.0, p.z2 - 0.045, p.x2, p.h - 0.041, p.z2)], 1.0, &m.lumber);
            }
            PropKind::Shelf | PropKind::Locker => {
                sb(&mut *commands, &mut *meshes, &[b3(p.x1 + 0.03, 0.0, p.z1 + 0.03, p.x2 - 0.03, p.h - 0.05, p.z2 - 0.03)], OSB_UV, &m.ply);
                let mut frame = vec![b3(p.x1, p.h - 0.05, p.z1, p.x2, p.h, p.z2), b3(p.x1, 0.0, p.z1, p.x2, 0.045, p.z1 + 0.029), b3(p.x1, 0.0, p.z2 - 0.029, p.x2, 0.045, p.z2)];
                for (x, z) in [(p.x1, p.z1), (p.x2 - 0.045, p.z1), (p.x1, p.z2 - 0.045), (p.x2 - 0.045, p.z2 - 0.045)] { frame.push(b3(x, 0.046, z, x + 0.045, p.h - 0.051, z + 0.045)); }
                sb(&mut *commands, &mut *meshes, &frame, 1.0, &m.lumber);
            }
            // piscina: borda baixa e água (bloqueia a passagem, não a visão)
            PropKind::Pool => {
                sb(&mut *commands, &mut *meshes, &[b3(p.x1, 0.0, p.z1, p.x2, p.h, p.z1 + 0.18), b3(p.x1, 0.0, p.z2 - 0.18, p.x2, p.h, p.z2), b3(p.x1, 0.0, p.z1 + 0.19, p.x1 + 0.18, p.h, p.z2 - 0.19), b3(p.x2 - 0.18, 0.0, p.z1 + 0.19, p.x2, p.h, p.z2 - 0.19)], 0.6, &m.wall_hi);
                let water = mats.add(StandardMaterial { base_color: Color::srgb(0.08, 0.32, 0.46), perceptual_roughness: 0.12, reflectance: 0.6, ..default() });
                sb(&mut *commands, &mut *meshes, &[b3(p.x1 + 0.19, 0.0, p.z1 + 0.19, p.x2 - 0.19, p.h - 0.05, p.z2 - 0.19)], 1.0, &water);
            }
            // guarda-corpo da escada: vidro com montantes pretos, como o da casa
            PropKind::Rail => {
                let glass = mats.add(StandardMaterial { base_color: Color::srgba(0.75, 0.86, 0.9, 0.16), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.05, reflectance: 0.6, ..default() });
                let top = |x: f32| ramp_h(x, p.z1 - 0.2) + 0.95;
                let mut posts = Vec::new();
                let n = ((w / 0.62).round() as usize).max(1);
                for i in 0..=n { let x = p.x1 + 0.03 + (w - 0.06) * i as f32 / n as f32; posts.push(b3(x - 0.025, 0.0, mz - 0.025, x + 0.025, top(x) + 0.06, mz + 0.025)); }
                sb(&mut *commands, &mut *meshes, &posts, 1.0, if map_index() == 5 { &m.lumber } else { &m.frame });
                let mut panes = Vec::new();
                for i in 0..n { let (xa, xb) = (p.x1 + 0.03 + (w - 0.06) * i as f32 / n as f32, p.x1 + 0.03 + (w - 0.06) * (i + 1) as f32 / n as f32); panes.push(b3(xa + 0.03, 0.02, mz - 0.006, xb - 0.03, top((xa + xb) / 2.0), mz + 0.006)); }
                if map_index() == 5 {
                    // duas réguas de sarrafo acompanhando a subida, no lugar do vidro
                    for i in 0..n {
                        let (xa, xb) = (p.x1 + 0.03 + (w - 0.06) * i as f32 / n as f32, p.x1 + 0.03 + (w - 0.06) * (i + 1) as f32 / n as f32);
                        for (k, drop) in [0.0f32, 0.45].into_iter().enumerate() {
                            let (pa, pb) = (Vec3::new(xa, top(xa) - drop, mz), Vec3::new(xb, top(xb) - drop, mz));
                            let d = pb - pa;
                            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.036 - k as f32 * 0.004, 0.085, d.length() + 0.02))), MeshMaterial3d(m.lumber.clone()),
                                Transform::from_translation((pa + pb) / 2.0).looking_to(d.normalize(), Vec3::Y), NotShadowCaster, MapEntity));
                        }
                    }
                } else {
                    let e = sb(&mut *commands, &mut *meshes, &panes, 1.0, &glass);
                    commands.entity(e).insert(NotShadowCaster);
                }
            }
            // peitoril de janela: meia parede de OSB no vão, com uma régua de sarrafo em cima
            PropKind::Sill => {
                sb(&mut *commands, &mut *meshes, &[b3(p.x1, 0.0, p.z1, p.x2, p.h - 0.04, p.z2)], OSB_UV, &m.ply);
                sb(&mut *commands, &mut *meshes, &[b3(p.x1 - 0.015, p.h - 0.04, p.z1 - 0.03, p.x2 + 0.015, p.h, p.z2 + 0.03)], 1.0, &m.lumber);
            }
            PropKind::Cover => {
                // nenhuma face de uma peça coincide com a de outra (faces no mesmo plano piscam): as chapas param
                // embaixo da tampa, a tampa sobra 12 mm para fora e os pés ficam 10 mm para fora das chapas
                let (za, zb, cap) = (p.z1 + d * 0.3, p.z2 - d * 0.3, p.h - 0.06);
                sb(&mut *commands, &mut *meshes, &[b3(p.x1, 0.0, za, p.x2, cap, za + 0.04), b3(p.x1, 0.0, zb - 0.04, p.x2, cap, zb)], 0.8, &m.wood);
                sb(&mut *commands, &mut *meshes, &[b3(p.x1 - 0.01, 0.0, p.z1, p.x1 + 0.06, 0.09, p.z2), b3(p.x2 - 0.06, 0.0, p.z1, p.x2 + 0.01, 0.09, p.z2),
                    b3(p.x1 - 0.012, cap, za - 0.012, p.x2 + 0.012, p.h, zb + 0.012)], 1.0, &m.frame);
            }
        }
    }
    // escadas (casa de dois andares): degraus de granito preto como os da casa, o vão do andar de cima revestido, os vãos
    // escuros por onde a escada continua para o outro andar e as escadas de mão do quintal. Os degraus são só desenho: a
    // altura de quem anda neles vem de `ramp_h`.
    if !map().ramps.is_empty() || !map().portals.is_empty() {
        let granite = mats.add(StandardMaterial { base_color: Color::srgb(0.13, 0.13, 0.14), perceptual_roughness: 0.32, reflectance: 0.5, ..default() });
        let pale = mats.add(StandardMaterial { base_color: Color::srgb(0.27, 0.27, 0.285), perceptual_roughness: 0.3, ..default() });
        let white = mats.add(StandardMaterial { base_color: Color::srgb(0.86, 0.86, 0.84), perceptual_roughness: 0.5, ..default() });
        let dark = mats.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
        let (mut steps, mut ladders, mut holes, mut lining) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut nosing, mut rails): (Vec<B3>, Vec<(Vec3, Vec3)>) = (Vec::new(), Vec::new());
        let bottom = map().pit.map_or(0.0, |p| -p.4 - 0.25);
        for r in map().ramps {
            // base de cada degrau: o piso (escada que sobe) ou o fundo do vão (escada que desce)
            let y0 = if r.ha < -0.01 || r.hb < -0.01 { bottom } else { 0.0 };
            let (lo, hi) = if r.along_x { (r.x1, r.x2) } else { (r.z1, r.z2) };
            let slab = |c0: f32, c1: f32, h: f32| { let (c0, c1) = (c0.min(c1), c0.max(c1)); if r.along_x { b3(c0, y0, r.z1, c1, h, r.z2) } else { b3(r.x1, y0, c0, r.x2, h, c1) } };
            let n = ((r.hb - r.ha).abs() / 0.19).round() as usize;
            for i in 0..n {
                let (c0, c1) = (r.a + (r.b - r.a) * i as f32 / n as f32, r.a + (r.b - r.a) * (i + 1) as f32 / n as f32);
                // subindo, o degrau já tem a altura do seguinte; descendo, cada piso fica um espelho abaixo do anterior
                let h = r.ha + (r.hb - r.ha) * (i + 1) as f32 / n as f32;
                steps.push(slab(c0, c1 + (c1 - c0).signum() * 0.004, h));
                // friso claro na quina de cada degrau (o granito preto some no escuro) e corrimão branco na parede
                let edge = if r.hb > r.ha { c0 } else { c1 };
                let (e0, e1) = (edge.min(edge + (c1 - c0).signum() * if r.hb > r.ha { 0.022 } else { -0.022 }), edge.max(edge + (c1 - c0).signum() * if r.hb > r.ha { 0.022 } else { -0.022 }));
                nosing.push(if r.along_x { b3(e0, h, r.z1 + 0.002, e1, h + 0.006, r.z2 - 0.002) } else { b3(r.x1 + 0.002, h, e0, r.x2 - 0.002, h + 0.006, e1) });
            }
            // corrimão branco na parede, inclinado junto com o lance
            if n > 0 {
                let (pa, pb) = if r.along_x { (Vec3::new(r.a, r.ha + 0.92, r.z1 + 0.045), Vec3::new(r.b, r.hb + 0.92, r.z1 + 0.045)) } else { (Vec3::new(r.x1 + 0.045, r.ha + 0.92, r.a), Vec3::new(r.x1 + 0.045, r.hb + 0.92, r.b)) };
                rails.push((pa, pb));
            }
            // patamares: o que sobra do retângulo antes de `a` e depois de `b`
            let (before, after) = if r.a < r.b { ((lo, r.a), (r.b, hi)) } else { ((r.a, hi), (lo, r.b)) };
            if (before.1 - before.0).abs() > 0.02 && (r.ha - y0).abs() > 0.02 { steps.push(slab(before.0, before.1, r.ha)); }
            if (after.1 - after.0).abs() > 0.02 && (r.hb - y0).abs() > 0.02 { steps.push(slab(after.0, after.1, r.hb)); }
        }
        if let Some((x1, z1, x2, z2, _)) = map().pit {
            // paredes do vão, do fundo até o piso (por fora do retângulo, para não coincidirem com os degraus)
            lining.push(b3(x1 - 0.03, bottom, z1 - 0.03, x2, 0.0, z1)); lining.push(b3(x1 - 0.03, bottom, z2, x2, 0.0, z2 + 0.03));
            lining.push(b3(x1 - 0.03, bottom, z1, x1, 0.0, z2));
        }
        for d in map().darks { holes.push(b3(d.0, d.1, d.2, d.3, d.4, d.5)); }
        for q in map().portals.iter().filter(|q| q.alt) {
            // escada de mão presa à parede da casa, no quintal
            let (cx, z) = (q.ha.0, q.z1 - 0.3 + WALL_T / 2.0 + 0.04);
            ladders.push(b3(cx - 0.23, 0.0, z - 0.02, cx - 0.19, WALL_H, z + 0.02)); ladders.push(b3(cx + 0.19, 0.0, z - 0.02, cx + 0.23, WALL_H, z + 0.02));
            let mut y = 0.28;
            while y < WALL_H - 0.05 { ladders.push(b3(cx - 0.19, y, z - 0.012, cx + 0.19, y + 0.028, z + 0.012)); y += 0.3; }
        }
        if ply {
            sb(&mut *commands, &mut *meshes, &steps, OSB_UV, &m.ply);
            sb(&mut *commands, &mut *meshes, &nosing, 1.0, &m.lumber);
        } else {
            sb(&mut *commands, &mut *meshes, &steps, 1.0, &granite);
            sb(&mut *commands, &mut *meshes, &nosing, 1.0, &pale);
        }
        for (pa, pb) in rails {
            let d = pb - pa;
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.045, 0.09, d.length()))), MeshMaterial3d(if ply { m.lumber.clone() } else { white.clone() }),
                Transform::from_translation((pa + pb) / 2.0).looking_to(d.normalize(), Vec3::Y), NotShadowCaster, MapEntity));
        }
        sb(&mut *commands, &mut *meshes, &lining, if ply { OSB_UV } else { 0.6 }, if ply { &m.ply } else { &m.wall_lo });
        sb(&mut *commands, &mut *meshes, &ladders, 1.0, &m.frame);
        sb(&mut *commands, &mut *meshes, &holes, 1.0, &dark);
    }
    if map_index() == 0 {
        model(&mut *commands, "korean_fire_extinguisher_01", Vec3::new(4.32, 0.0, 11.3), hp2, Vec3::ONE, Vec3::ZERO);
        model(&mut *commands, "korean_fire_extinguisher_01", Vec3::new(14.32, 0.0, 7.6), hp2, Vec3::ONE, Vec3::ZERO);
    }

    // portas
    for (i, d) in level.doors.iter().enumerate() {
        let leaf = sb(&mut *commands, &mut *meshes, &[b3(0.006, 0.01, -0.0225, d.len - 0.006, DOOR_H - 0.03, 0.0225)], 1.0, &m.door);
        let kick = sb(&mut *commands, &mut *meshes, &[b3(0.1, 0.04, -0.026, d.len - 0.1, 0.29, 0.026), b3(d.len - 0.2, 1.0, -0.05, d.len - 0.07, 1.03, 0.05), b3(d.len - 0.11, 0.95, -0.03, d.len - 0.07, 1.09, 0.03)], 1.0, &m.steel);
        let win = sb(&mut *commands, &mut *meshes, &[b3(d.len / 2.0 - 0.11, 1.3, -0.026, d.len / 2.0 + 0.11, 1.8, 0.026)], 1.0, &m.frame);
        commands.spawn((Transform::from_xyz(d.hx, 0.0, d.hz), Visibility::default(), DoorLeaf(i), MapEntity)).add_children(&[leaf, kick, win]);
    }

    // letreiros estampados: fixos na casa do tutorial; nos outros mapas, o nome de cada cômodo vai acima da porta, do lado de fora
    let ink = "#141414";
    let hp = std::f32::consts::FRAC_PI_2;
    if map_index() == 0 {
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("STAGING AREA", 62.0)], 2.6, 0.36, Vec3::new(2.0, 2.3, 12.106), 0.0, ink, false);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("SAFETY RULES", 44.0), ("1 · EVERY GUN IS LOADED", 30.0), ("2 · MUZZLE ALWAYS IN A SAFE DIRECTION", 30.0), ("3 · FINGER OFF THE TRIGGER UNTIL READY", 30.0), ("4 · KNOW YOUR TARGET AND WHAT IS BEYOND", 30.0)], 2.3, 1.15, Vec3::new(0.106, 1.75, 14.4), hp, ink, false);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("CORRIDOR 1", 70.0)], 1.6, 0.3, Vec3::new(5.894, 2.3, 14.2), -hp, ink, false);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("ROOM A", 120.0)], 1.0, 0.42, Vec3::new(10.0, 2.44, 5.894), PI_F, ink, true);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("ROOM B", 120.0)], 1.0, 0.42, Vec3::new(13.894, 2.44, 10.95), -hp, ink, true);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("CORRIDOR 3", 96.0)], 1.05, 0.36, Vec3::new(17.55, 2.44, 6.106), 0.0, ink, true);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("ROOM C", 120.0)], 1.0, 0.42, Vec3::new(19.55, 2.44, 4.106), 0.0, ink, true);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("END OF COURSE", 64.0)], 2.2, 0.34, Vec3::new(18.6, 2.3, 0.106), 0.0, ink, false);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[("WHITE = NO SHOOT", 76.0)], 1.7, 0.34, Vec3::new(17.5, 2.3, 11.894), PI_F, "#78140e", false);
        sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[(".45 ACP", 80.0), ("BLANK · LOT 26-11", 52.0)], 0.5, 0.2, Vec3::new(5.37, 0.35, 16.894), PI_F, "#d8cfa4", false);
    } else {
        for d in doorways() {
            let name = zones()[zone_index(d.zone)].name.to_uppercase();
            let wdt = (0.2 + name.len() as f32 * 0.115).min(2.2);
            let size = (112.0f32).min(512.0 * 1.55 / name.len() as f32);
            let (pos, ry) = if d.axis_z { (Vec3::new(d.cx, 2.44, d.cz - d.s * 0.106), if d.s > 0.0 { PI_F } else { 0.0 }) } else { (Vec3::new(d.cx - d.s * 0.106, 2.44, d.cz), if d.s > 0.0 { -hp } else { hp }) };
            sign(&mut *commands, &mut *meshes, &mut *mats, &mut *images, &[(name.as_str(), size)], wdt, 0.36, pos, ry, ink, true);
        }
        let _ = hp;
    }

    // luminárias
    commands.insert_resource(AmbientLight { color: Color::srgb(0.75, 0.8, 0.9), brightness: env_f("CQB_AMB", 75.0), ..default() });
    let mut tubes = Tubes::default();
    for (idx, def) in lights().iter().enumerate() {
        let color = Color::linear_rgb(def.color[0], def.color[1], def.color[2]);
        let base = CEIL_LUMENS * def.i * env_f("CQB_LUM", 1.0);
        commands.spawn((
            SpotLight { color, intensity: base, range: env_f("CQB_RANGE", 6.5), radius: 0.15, shadows_enabled: std::env::var("CQB_SOMBRAS").is_ok(), outer_angle: env_f("CQB_SPOT", 1.2), inner_angle: env_f("CQB_SPOTIN", 0.75), shadow_depth_bias: env_f("CQB_CDB", 0.04), shadow_normal_bias: env_f("CQB_CNB", 1.2), shadow_map_near_z: 0.7, ..default() },
            Transform::from_xyz(def.x, 3.9, def.z).looking_at(Vec3::new(def.x, 0.0, def.z), Vec3::NEG_Z),
            light_layers(), VolumetricLight, CeilLight { idx, base }, MapEntity,
        ));
        // luz de preenchimento, sem sombra: faz o papel do rebatimento em paredes e forro
        if env_f("CQB_FILL", 0.0) > 0.0 {
            commands.spawn((PointLight { color, intensity: base * env_f("CQB_FILL", 0.0), range: 7.0, radius: 0.3, shadows_enabled: false, ..default() }, Transform::from_xyz(def.x, 2.25, def.z), light_layers(), MapEntity));
        }
        let (lx, lz) = if def.rot { (0.2, 1.25) } else { (1.25, 0.2) };
        commands.spawn((SceneRoot(asset.load(GltfAssetLabel::Scene(0).from_asset("models/ph/mounted_fluorescent_lights/mounted_fluorescent_lights.gltf"))),
            Transform { translation: Vec3::new(def.x, WALL_H, def.z), rotation: Quat::from_rotation_y(if def.rot { hp2 } else { 0.0 }), scale: Vec3::splat(1.35) }, NoShadowScene, MapEntity));
        let em = LinearRgba::rgb(def.color[0], def.color[1], def.color[2]);
        let tm = mats.add(StandardMaterial { base_color: Color::BLACK, emissive: em * TUBE_NITS, ..default() });
        tubes.0.push((tm.clone(), em));
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(lx * 0.9, 0.012, lz * 0.5))), MeshMaterial3d(tm), Transform::from_xyz(def.x, 2.742, def.z), RenderLayers::layer(L_FP), NotShadowCaster, MapEntity));
    }
    commands.insert_resource(tubes);
    // névoa fina para os fachos de luz
    // névoa volumétrica: desligada por padrão (criava véu e ruído perto das luminárias); CQB_VOL=1 liga
    if std::env::var("CQB_VOL").is_ok() {
        commands.spawn((FogVolume { density_factor: env_f("CQB_FOGD", 0.00001), scattering: 0.35, ..default() }, Transform::from_xyz(10.5, 1.4, 9.0).with_scale(Vec3::new(22.0, 2.8, 19.0)), MapEntity));
    }
}
const PI_F: f32 = std::f32::consts::PI;
