//! Alvos, disparo, portas e efeitos.
use crate::art;
use crate::map::*;
use crate::player::*;
use crate::state::*;
use crate::world::*;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

pub const BOARD_W: f32 = 0.46;
pub const BOARD_H: f32 = 0.76;
pub const BOARD_Y0: f32 = 0.82;
const REACT: f32 = 1.9;

#[derive(Component)] pub struct TargetPivot(pub usize);
#[derive(Component)] pub struct Puff { vel: Vec3, life: f32, max: f32, grow: f32, op: f32, grav: f32 }
#[derive(Component)] pub struct Casing { vel: Vec3, ang: Vec3, life: f32, rest: bool }
#[derive(Component)] pub struct FlashQuad;
#[derive(Component)] pub struct Decal;

#[derive(Resource)]
pub struct Fx {
    pub quad: Handle<Mesh>, pub decal_mat: Handle<StandardMaterial>, pub soft: Handle<Image>, pub casing_mesh: Handle<Mesh>,
    pub mark_mesh: Handle<Mesh>, pub rim_mesh: Handle<Mesh>, pub mark_mat: Handle<StandardMaterial>, pub rim_mat: Handle<StandardMaterial>,
    pub flash: Entity, pub flash_t: f32, pub decals: VecDeque<Entity>, pub muzzle_i: f32,
}

/// Malhas e materiais dos alvos, guardados para remontar os alvos quando o mapa muda.
#[derive(Resource, Clone)]
pub struct TargetArt { pub frame: Handle<Mesh>, pub board: Handle<Mesh>, pub shoot: Handle<StandardMaterial>, pub no_shoot: Handle<StandardMaterial>, pub wood: Handle<StandardMaterial>,
    /// Bonecos 3D (cena e arquivo, de onde saem os clipes): refém de roupa clara com as mãos para o alto e ameaça armada de cáqui.
    pub hostage: Handle<Scene>, pub hostage_file: Handle<Gltf>, pub threat: Handle<Scene>, pub threat_file: Handle<Gltf> }

/// O alvo é um boneco 3D? Reféns e ameaças são; o alvo fixo de treino da primeira lição continua de papelão.
pub fn is_fig(def: &TargetDef) -> bool { def.kind == TKind::Civil || !def.passive }
/// Zona atingida num boneco em pé, pela distância ao eixo do corpo e pela altura: 3 = cabeça ou centro do peito (equivale à zona A),
/// 2 = tronco (C), 1 = braços e pernas (D), 0 = fora.
pub fn fig_zone(lx: f32, y: f32) -> u8 {
    if !(0.02..=1.78).contains(&y) { return 0; }
    if y >= 1.52 { return if lx <= 0.11 { 3 } else { 0 }; }
    if y >= 0.95 { return if lx <= 0.10 && y >= 1.12 { 3 } else if lx <= 0.21 { 2 } else if lx <= 0.30 { 1 } else { 0 }; }
    if lx <= 0.20 { 1 } else { 0 }
}

/// Cria os alvos do mapa atual: ameaças e reféns são bonecos 3D; o alvo de treino é papelão tipo IPSC grampeado em dois sarrafos.
pub fn spawn_targets(commands: &mut Commands, art: &TargetArt, level: &Level) -> Vec<TargetState> {
    let mut list = Vec::new();
    for (i, def) in target_defs().iter().enumerate() {
        let ry = (def.fx - def.x).atan2(def.fz - def.z);
        let fwd = Vec2::new(ry.sin(), ry.cos());
        let root = commands.spawn((Transform::from_xyz(def.x, 0.0, def.z).with_rotation(Quat::from_rotation_y(ry)), Visibility::default())).id();
        let pivot = commands.spawn((Transform::IDENTITY, Visibility::default(), TargetPivot(i))).id();
        let fr = commands.spawn((Mesh3d(art.frame.clone()), MeshMaterial3d(art.wood.clone()), Transform::IDENTITY)).id();
        let civil = def.kind == TKind::Civil;
        let bd = commands.spawn((Mesh3d(art.board.clone()), MeshMaterial3d(if civil { art.no_shoot.clone() } else { art.shoot.clone() }), Transform::from_xyz(0.0, BOARD_Y0 + BOARD_H / 2.0, 0.0))).id();
        commands.entity(pivot).add_children(&[fr, bd]);
        if is_fig(def) {
            // o alvo é o boneco; papelão e sarrafos ficam ocultos (o papelão continua existindo só para a lógica de marcas)
            commands.entity(fr).insert(Visibility::Hidden);
            commands.entity(bd).insert(Visibility::Hidden);
            let (scene, file) = if civil { (art.hostage.clone(), art.hostage_file.clone()) } else { (art.threat.clone(), art.threat_file.clone()) };
            let h = commands.spawn((SceneRoot(scene), Transform::IDENTITY, Visibility::default(), crate::operator::Fig { target: i, file, threat: !civil, fell: false })).id();
            commands.entity(pivot).add_child(h);
        }
        commands.entity(root).add_child(pivot);
        let mut t = TargetState {
            def: *def, alive: true, fall: 0.0, hits: 0, seen: false, vis: false, react: 0.0, contact: false, contact_t: 0.0, first_facing: None, fwd,
            fall_dir: -1.0, fall_max: 1.5, pivot, board: bd, mask: i, marks: Vec::new(), fall_v: 0.0, sw: 0.0, swv: 0.0, landed: false, root, off: false,
        };
        place_target(level, &mut t);
        list.push(t);
    }
    list
}

pub fn setup_targets(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut smats: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>, mats: Res<Mats>, level: Res<Level>, asset: Res<AssetServer>) {
    let frame = meshes.add(boxes_mesh(&[b3(-0.17, 0.0, -0.028, -0.13, 1.5, -0.006), b3(0.13, 0.0, -0.028, 0.17, 1.5, -0.006), b3(-0.17, 0.0, -0.24, -0.13, 0.04, 0.24), b3(0.13, 0.0, -0.24, 0.17, 0.04, 0.24), b3(-0.17, 0.04, -0.03, 0.17, 0.08, -0.004)], 1.0));
    let board = meshes.add(Rectangle::new(BOARD_W, BOARD_H));
    let mk = |images: &mut Assets<Image>, smats: &mut Assets<StandardMaterial>, ns: bool| {
        let tex = images.add(art::svg_image(&art::ipsc_svg(ns), 920, 1520, true));
        smats.add(StandardMaterial { base_color_texture: Some(tex), alpha_mode: AlphaMode::Mask(0.5), cull_mode: None, double_sided: true, perceptual_roughness: 0.92, reflectance: 0.2, ..default() })
    };
    let (shoot, no_shoot) = (mk(&mut images, &mut smats, false), mk(&mut images, &mut smats, true));
    let art = TargetArt { frame, board, shoot, no_shoot, wood: mats.wood.clone(),
        hostage: asset.load(GltfAssetLabel::Scene(0).from_asset("models/hostage.glb")), hostage_file: asset.load("models/hostage.glb"),
        threat: asset.load(GltfAssetLabel::Scene(0).from_asset("models/threat.glb")), threat_file: asset.load("models/threat.glb") };
    let list = spawn_targets(&mut commands, &art, &level);
    commands.insert_resource(Targets { list, masks: Vec::new() });
    commands.insert_resource(art);

    // efeitos
    let soft = images.add(art::svg_image(&art::radial_svg(&[(0.0, "#fff", 1.0), (0.5, "#fff", 0.35), (1.0, "#fff", 0.0)]), 64, 64, false));
    let hole = images.add(art::svg_image(&art::crater_svg(), 256, 256, true));
    let phole = images.add(art::svg_image(&art::paper_hole_svg(7), 64, 64, true));
    let flash_tex = images.add(art::svg_image(&art::flash_svg(), 128, 128, false));
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let decal_mat = smats.add(StandardMaterial { base_color_texture: Some(hole), alpha_mode: AlphaMode::Blend, perceptual_roughness: 1.0, depth_bias: 12.0, ..default() });
    let fm = smats.add(StandardMaterial { base_color: Color::linear_rgb(9.0, 6.5, 3.5), base_color_texture: Some(flash_tex), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, ..default() });
    let flash = commands.spawn((Transform::default(), Visibility::Hidden)).id();
    for t in [Transform::from_scale(Vec3::splat(0.32)), Transform::from_xyz(0.0, 0.0, -0.12).with_rotation(Quat::from_rotation_y(FRAC_PI_2)).with_scale(Vec3::new(0.42, 0.16, 1.0)),
        Transform::from_xyz(0.0, 0.0, -0.12).with_rotation(Quat::from_euler(EulerRot::XYZ, FRAC_PI_2, 0.0, FRAC_PI_2)).with_scale(Vec3::new(0.42, 0.16, 1.0))] {
        let q = commands.spawn((Mesh3d(quad.clone()), MeshMaterial3d(fm.clone()), t, NotShadowCaster, FlashQuad)).id();
        commands.entity(flash).add_child(q);
    }
    commands.insert_resource(Fx {
        quad, decal_mat, soft, casing_mesh: meshes.add(Cylinder::new(0.0045, 0.03)), mark_mesh: meshes.add(Circle::new(0.014)), rim_mesh: meshes.add(Circle::new(0.021)),
        mark_mat: smats.add(StandardMaterial { base_color_texture: Some(phole), alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.9, depth_bias: 20.0, cull_mode: None, double_sided: true, ..default() }),
        rim_mat: smats.add(StandardMaterial { base_color: Color::srgb(0.8, 0.74, 0.58), perceptual_roughness: 1.0, depth_bias: 10.0, ..default() }),
        flash, flash_t: 0.0, decals: VecDeque::new(), muzzle_i: 0.0,
    });
}

/// Recalcula direção e espaço de queda de um alvo a partir da posição atual dele.
pub fn place_target(level: &Level, t: &mut TargetState) {
    let ry = (t.def.fx - t.def.x).atan2(t.def.fz - t.def.z);
    t.fwd = Vec2::new(ry.sin(), ry.cos());
    let back = level.cast_ray(t.def.x, t.def.z, -t.fwd.x, -t.fwd.y, 5.0, 1.0);
    let front = level.cast_ray(t.def.x, t.def.z, t.fwd.x, t.fwd.y, 5.0, 1.0);
    let top = BOARD_Y0 + BOARD_H + 0.05;
    let use_back = back >= top + 0.1 || back >= front;
    let sp = if use_back { back } else { front };
    t.fall_dir = if use_back { -1.0 } else { 1.0 };
    t.fall_max = if sp >= top + 0.1 { FRAC_PI_2 - 0.03 } else { ((sp - 0.06) / top).clamp(0.1, 1.0).asin() };
}

struct Hit { t: f32, point: Vec3, normal: Vec3, target: Option<(usize, u8)>, through: Vec<(usize, f32, Vec3)>, metal: bool }

fn ray_aabb(o: Vec3, d: Vec3, min: Vec3, max: Vec3) -> Option<(f32, Vec3)> {
    let (mut tmin, mut tmax, mut n) = (0.0f32, f32::INFINITY, Vec3::ZERO);
    for a in 0..3 {
        if d[a].abs() < 1e-9 { if o[a] < min[a] || o[a] > max[a] { return None; } continue; }
        let (mut t1, mut t2) = ((min[a] - o[a]) / d[a], (max[a] - o[a]) / d[a]);
        let mut sign = -1.0;
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); sign = 1.0; }
        if t1 > tmin { tmin = t1; n = Vec3::ZERO; n[a] = sign; }
        tmax = tmax.min(t2);
        if tmin > tmax { return None; }
    }
    if n == Vec3::ZERO { None } else { Some((tmin, n)) }
}

fn raycast(level: &Level, targets: &Targets, o: Vec3, d: Vec3) -> Option<Hit> {
    let mut best: Option<Hit> = None;
    let mut through: Vec<(usize, f32, Vec3)> = Vec::new();
    let mut consider = |h: Hit| { if h.t > 0.0 && best.as_ref().map_or(true, |b| h.t < b.t) { best = Some(h); } };
    if d.y < -1e-5 { let t = -o.y / d.y; consider(Hit { t, point: o + d * t, normal: Vec3::Y, target: None, through: Vec::new(), metal: false }); }
    if d.y > 1e-5 { let t = (WALL_H - o.y) / d.y; consider(Hit { t, point: o + d * t, normal: Vec3::NEG_Y, target: None, through: Vec::new(), metal: false }); }
    let nw = level.walls.len();
    for (si, b) in level.solids.iter().enumerate().filter(|(_, b)| b.on) {
        if let Some((t, n)) = ray_aabb(o, d, Vec3::new(b.x1, 0.0, b.z1), Vec3::new(b.x2, b.h, b.z2)) {
            let metal = b.kind == SolidKind::Door || (si >= nw && si < nw + level.props.len() && matches!(level.props[si - nw].kind, PropKind::Barrel | PropKind::Locker | PropKind::Shelf | PropKind::Table));
            consider(Hit { t, point: o + d * t, normal: n, target: None, through: Vec::new(), metal });
        }
    }
    for b in &level.lintels {
        if let Some((t, n)) = ray_aabb(o, d, Vec3::new(b.x1, DOOR_H, b.z1), Vec3::new(b.x2, WALL_H, b.z2)) { consider(Hit { t, point: o + d * t, normal: n, target: None, through: Vec::new(), metal: false }); }
    }
    for (i, tg) in targets.list.iter().enumerate() {
        if !tg.alive { continue; }
        // boneco em pé: plano vertical de frente para quem atira, com zonas pelo corpo
        if is_fig(&tg.def) {
            let base = Vec3::new(tg.def.x, 0.0, tg.def.z);
            let h = Vec2::new(o.x - base.x, o.z - base.z);
            if h.length() < 0.05 { continue; }
            let hn = h.normalize();
            let n = Vec3::new(hn.x, 0.0, hn.y);
            let den = d.dot(n);
            if den.abs() < 1e-5 { continue; }
            let t = (base - o).dot(n) / den;
            if t <= 0.0 { continue; }
            let q = o + d * t;
            let lx = ((q.x - base.x) * hn.y - (q.z - base.z) * hn.x).abs();
            let zone = fig_zone(lx, q.y);
            if zone != 0 { consider(Hit { t, point: q, normal: n, target: Some((i, zone)), through: Vec::new(), metal: false }); }
            continue;
        }
        let n = Vec3::new(tg.fwd.x, 0.0, tg.fwd.y);
        let den = d.dot(n);
        if den.abs() < 1e-5 { continue; }
        let base = Vec3::new(tg.def.x, 0.0, tg.def.z);
        let t = (base - o).dot(n) / den;
        if t <= 0.0 { continue; }
        let q = o + d * t;
        let lx = (q.x - base.x) * tg.fwd.y - (q.z - base.z) * tg.fwd.x;
        let (u, v) = (lx / BOARD_W + 0.5, 1.0 - (q.y - BOARD_Y0) / BOARD_H);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) { continue; }
        let zone = art::target_zone(u, v);
        if zone == 0 { continue; }
        consider(Hit { t, point: q, normal: if den < 0.0 { n } else { -n }, target: Some((i, zone)), through: Vec::new(), metal: false });
    }
    if let Some(b) = best.as_mut() { b.through = through.into_iter().filter(|x| x.1 < b.t).collect(); }
    best
}

fn puff(commands: &mut Commands, fx: &Fx, smats: &mut Assets<StandardMaterial>, point: Vec3, normal: Vec3, n: usize, color: Color, speed: f32, size: f32) {
    for k in 0..n {
        let r = |a: f32, b: f32, s: f32| a + (b - a) * ((point.x * 12.9898 + point.z * 78.233 + k as f32 * 37.1 + s).sin() * 43758.547).fract().abs();
        let vel = normal * speed * r(0.3, 1.0, 1.0) + Vec3::new(r(-0.4, 0.4, 2.0), r(0.0, 0.5, 3.0), r(-0.4, 0.4, 4.0));
        let life = r(0.35, 0.8, 5.0);
        let op = r(0.2, 0.42, 6.0);
        let mat = smats.add(StandardMaterial { base_color: color.with_alpha(op), base_color_texture: Some(fx.soft.clone()), alpha_mode: AlphaMode::Blend, unlit: true, cull_mode: None, ..default() });
        commands.spawn((Mesh3d(fx.quad.clone()), MeshMaterial3d(mat), Transform::from_translation(point + normal * 0.03).with_scale(Vec3::splat(size * r(0.6, 1.2, 7.0))), NotShadowCaster,
            Puff { vel, life, max: life, grow: r(0.5, 1.2, 8.0), op, grav: 0.0 }));
    }
}

fn paper_mark(commands: &mut Commands, fx: &Fx, t: &mut TargetState, point: Vec3, seed: f32) {
    let base = Vec3::new(t.def.x, 0.0, t.def.z);
    let lx = (point.x - base.x) * t.fwd.y - (point.z - base.z) * t.fwd.x;
    let ly = point.y - BOARD_Y0 - BOARD_H / 2.0;
    let s = 0.042 + rand01(seed * 3.3) * 0.014;
    let mk = commands.spawn((Mesh3d(fx.quad.clone()), MeshMaterial3d(fx.mark_mat.clone()), Transform::from_xyz(lx, ly, 0.003).with_rotation(Quat::from_rotation_z(rand01(seed * 7.1) * TAU)).with_scale(Vec3::splat(s)), NotShadowCaster)).id();
    commands.entity(t.board).add_child(mk);
    t.marks.push(mk);
}
/// Fagulhas de impacto em metal.
fn sparks(commands: &mut Commands, fx: &Fx, smats: &mut Assets<StandardMaterial>, point: Vec3, normal: Vec3, seed: f32) {
    let mat = smats.add(StandardMaterial { base_color: Color::linear_rgb(14.0, 8.0, 2.5), base_color_texture: Some(fx.soft.clone()), unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, ..default() });
    for k in 0..9 {
        let r = |s: f32| rand01(seed * 17.3 + k as f32 * 5.7 + s);
        let vel = normal * (1.5 + r(1.0) * 3.5) + Vec3::new(r(2.0) - 0.5, r(3.0) - 0.2, r(4.0) - 0.5) * 4.0;
        let life = 0.18 + r(5.0) * 0.3;
        commands.spawn((Mesh3d(fx.quad.clone()), MeshMaterial3d(mat.clone()), Transform::from_translation(point + normal * 0.02).with_scale(Vec3::splat(0.018 + r(6.0) * 0.02)), NotShadowCaster,
            Puff { vel, life, max: life, grow: -0.02, op: 1.0, grav: 9.0 }));
    }
}
/// Lascas de alvenaria que caem do impacto.
fn chips(commands: &mut Commands, fx: &Fx, smats: &mut Assets<StandardMaterial>, point: Vec3, normal: Vec3, seed: f32) {
    let mat = smats.add(StandardMaterial { base_color: Color::srgb(0.42, 0.4, 0.36), perceptual_roughness: 1.0, cull_mode: None, ..default() });
    for k in 0..5 {
        let r = |s: f32| rand01(seed * 11.9 + k as f32 * 3.1 + s);
        let vel = normal * (0.8 + r(1.0) * 1.6) + Vec3::new(r(2.0) - 0.5, r(3.0) * 0.8, r(4.0) - 0.5) * 1.8;
        let life = 0.5 + r(5.0) * 0.5;
        commands.spawn((Mesh3d(fx.quad.clone()), MeshMaterial3d(mat.clone()), Transform::from_translation(point + normal * 0.02).with_scale(Vec3::splat(0.012 + r(6.0) * 0.014)), NotShadowCaster,
            Puff { vel, life, max: life, grow: 0.0, op: 1.0, grav: 9.8 }));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn combat(
    time: Res<Time>, c: Res<Controls>, mut p: ResMut<Player>, mut game: ResMut<Game>, mut level: ResMut<Level>, mut targets: ResMut<Targets>,
    mut sfx: EventWriter<Sfx>, mut fx: ResMut<Fx>, mut commands: Commands, mut smats: ResMut<Assets<StandardMaterial>>,
    muzzle: Query<&GlobalTransform, With<Muzzle>>, mut flash: Query<&mut Transform, (Without<TargetPivot>, Without<DoorLeaf>, Without<MuzzleLight>, Without<Puff>, Without<Casing>)>,
    mut ml: Query<(&mut PointLight, &mut Transform), With<MuzzleLight>>,
    mut pivots: Query<(&mut Transform, &TargetPivot), (Without<DoorLeaf>, Without<MuzzleLight>)>,
    mut doors: Query<(&mut Transform, &DoorLeaf), (Without<TargetPivot>, Without<MuzzleLight>)>,
    mut vis: Query<&mut Visibility>,
) {
    let dt = time.delta_secs().min(0.05);
    // clarão de boca
    if fx.flash_t > 0.0 { fx.flash_t -= dt; if fx.flash_t <= 0.0 { if let Ok(mut v) = vis.get_mut(fx.flash) { *v = Visibility::Hidden; } } }
    fx.muzzle_i = (fx.muzzle_i - dt * 14.0).max(0.0);
    if let Ok((mut l, _)) = ml.single_mut() { l.intensity = fx.muzzle_i * 900_000.0; }
    if game.mode != Mode::Play { return; }
    let bird = game.view == View::Bird;

    /* ---------- portas ---------- */
    let f2 = p.fwd();
    game.near_door = None;
    for (i, d) in level.doors.iter().enumerate() {
        if d.opening { continue; }
        let dv = Vec2::new(d.cx, d.cz) - p.pos;
        let dist = dv.length();
        if dist > 1.9 || (dist > 0.6 && dv.dot(f2) / dist < 0.15) { continue; }
        game.near_door = Some(i); break;
    }
    if c.interact {
        if let Some(i) = game.near_door {
            level.doors[i].opening = true;
            game.door_side[i] = Some((p.pos.x - level.doors[i].cx).abs() > 0.5);
            sfx.write(Sfx::Door(Vec3::new(level.doors[i].cx, 1.0, level.doors[i].cz)));
        }
    }
    for i in 0..level.doors.len() {
        if level.doors[i].opening && level.doors[i].t < 1.0 {
            let t = (level.doors[i].t + dt / 0.75).min(1.0);
            let open = t * t * (3.0 - 2.0 * t);
            level.doors[i].t = t; level.doors[i].open = open;
            let s = level.doors[i].solid;
            level.solids[s].on = open < 0.5;
        }
    }
    for (mut tf, dl) in doors.iter_mut() { tf.rotation = Quat::from_rotation_y(level.doors[dl.0].open * 172f32.to_radians()); }

    /* ---------- disparo ---------- */
    if c.fire && !p.reloading && !p.switching() && p.fire_cool <= 0.0 && (bird || (p.g_block < 0.55 && p.g_lower < 0.5)) {
        if p.mag == 0 { sfx.write(Sfx::Dry); p.fire_cool = 0.2; }
        else {
            p.mag -= 1; p.fire_cool = p.def().cool; game.stats.shots += 1; p.shot_n = p.shot_n.wrapping_add(1);
            let q = Quat::from_euler(EulerRot::YXZ, p.yaw + if bird { 0.0 } else { p.recoil.y }, if bird { 0.0 } else { p.pitch + p.recoil.x }, 0.0);
            let (mut dir, right, up) = (q * Vec3::NEG_Z, q * Vec3::X, q * Vec3::Y);
            let spread = ((if p.ads { 0.004 } else { 0.02 }) + p.speed * 0.006 * if p.ads { 0.4 } else { 1.0 } - p.crouch * 0.004f32).max(0.001);
            let (ra, rr) = (rand01(game.time * 91.7 + game.stats.shots as f32) * TAU, rand01(game.time * 33.3 + 7.0).sqrt() * spread);
            dir = (dir + right * ra.cos() * rr + up * ra.sin() * rr).normalize();
            let o = if bird { Vec3::new(p.eye.x, 1.2, p.eye.z) } else { p.eye };
            let mut end = o + dir * 60.0;
            if let Some(h) = raycast(&level, &targets, o, dir) {
                end = h.point;
                for (ti, _, q) in h.through.iter() { let t = &mut targets.list[*ti]; paper_mark(&mut commands, &fx, t, *q, game.time + 3.0); t.swv -= 1.2; sfx.write(Sfx::Paper(*q)); }
                if let Some((ti, zone)) = h.target {
                    game.stats.hits += 1; game.hit_t = 0.14;
                    sfx.write(Sfx::Thwack(h.point));
                    puff(&mut commands, &fx, &mut smats, h.point, -dir, 2, Color::srgb(0.3, 0.27, 0.2), 1.0, 0.1);
                    match zone { 3 => game.stats.zone_a += 1, 2 => game.stats.zone_c += 1, _ => game.stats.zone_d += 1 }
                    let t = &mut targets.list[ti];
                    paper_mark(&mut commands, &fx, t, h.point, game.time);
                    t.swv -= 2.6 * t.fall_dir.signum().abs();
                    if t.def.kind == TKind::Civil {
                        t.alive = false; t.fall_v = 1.0; game.stats.civ_hit += 1; sfx.write(Sfx::Bad);
                        if game.free { game.toast("You hit a hostage. Look for the weapon before firing.", 2, 5.0); }
                    } else {
                        let one_shot = t.hits == 0 && zone == 3;
                        t.hits += if zone == 3 { 2 } else { 1 };
                        if t.hits >= 2 {
                            t.alive = false; t.fall_v = 1.0; game.stats.threats_down += 1;
                            if !t.def.passive && !game.demo {
                                let k = crate::lessons::kill_metrics(&level, &p, t, game.time, one_shot);
                                game.coach = Some((format!("Threat down · slice angle {}° · body exposed {} cm · {:.1} s in its line of fire · {} pts", k.angle.round() as i32, k.expo_cm.round() as i32, k.dt, k.pts.round() as i32), 4, 5.0));
                                game.kills.push(k); game.ui_dirty = true;
                            }
                            // derrubada antes de o contato ser avaliado: quem atirou viu primeiro
                            if t.first_facing.is_none() { t.first_facing = Some(true); }
                        }
                    }
                } else {
                    let rot = Quat::from_rotation_arc(Vec3::Z, h.normal) * Quat::from_rotation_z(rand01(game.time * 5.1) * TAU);
                    let sc = 0.13 + rand01(game.time * 9.3) * 0.06;
                    let e = commands.spawn((Mesh3d(fx.quad.clone()), MeshMaterial3d(fx.decal_mat.clone()), Transform::from_translation(h.point + h.normal * 0.004).with_rotation(rot).with_scale(Vec3::splat(sc)), NotShadowCaster, Decal)).id();
                    fx.decals.push_back(e);
                    if fx.decals.len() > 90 { if let Some(old) = fx.decals.pop_front() { commands.entity(old).despawn(); } }
                    if h.metal { sparks(&mut commands, &fx, &mut smats, h.point, h.normal, game.time); }
                    else {
                        puff(&mut commands, &fx, &mut smats, h.point, h.normal, 4, Color::srgb(0.22, 0.21, 0.19), 1.6, 0.18);
                        chips(&mut commands, &fx, &mut smats, h.point, h.normal, game.time);
                    }
                    sfx.write(if h.metal { Sfx::Metal(h.point) } else { Sfx::Impact(h.point) });
                }
            }
            let mz = if bird { p.eye + dir * 0.5 - Vec3::Y * 0.1 } else { muzzle.single().map(|g| g.translation()).unwrap_or(o + dir * 0.8) };
            if let Ok(mut tf) = flash.get_mut(fx.flash) {
                let s = 0.8 + rand01(game.time * 3.7) * 0.45;
                *tf = Transform::from_translation(mz).with_rotation(Quat::from_rotation_arc(Vec3::NEG_Z, dir) * Quat::from_rotation_z(rand01(game.time * 7.7) * TAU)).with_scale(Vec3::splat(s));
            }
            if let Ok(mut v) = vis.get_mut(fx.flash) { *v = Visibility::Visible; }
            fx.flash_t = 0.045; fx.muzzle_i = 1.0;
            if let Ok((_, mut tf)) = ml.single_mut() { tf.translation = mz + dir * 0.15; }
            game.tracers.push((mz, end, 0.06));
            puff(&mut commands, &fx, &mut smats, mz + dir * 0.12, dir, 2, Color::srgb(0.2, 0.2, 0.19), 1.8, 0.07);
            if !bird {
                let vel = right * (1.6 + rand01(game.time * 2.1) * 0.8) + up * (1.2 + rand01(game.time * 4.3) * 0.8) + dir * (rand01(game.time * 6.1) * 0.8 - 0.5);
                commands.spawn((Mesh3d(fx.casing_mesh.clone()), MeshMaterial3d(smats.add(StandardMaterial { base_color: Color::srgb(0.79, 0.64, 0.30), metallic: 1.0, perceptual_roughness: 0.3, ..default() })),
                    Transform::from_translation(mz - dir * 0.12 + right * 0.02 + up * 0.02), Casing { vel, ang: Vec3::new(17.0, -11.0, 23.0), life: 4.0, rest: false }));
            }
            p.g_kick = 1.0;
            p.recoil.x += (0.028 + rand01(game.time * 8.9) * 0.01) * p.def().kick;
            p.recoil.y += (rand01(game.time * 12.3) - 0.5) * 0.02 * p.def().kick;
            sfx.write(Sfx::Shot);
        }
    }

    /* ---------- alvos: queda e reação ---------- */
    let head = p.head_h();
    let mut exposed = false;
    for i in 0..targets.list.len() {
        let t = &mut targets.list[i];
        t.swv += (-70.0 * t.sw - 6.0 * t.swv) * dt;
        t.sw += t.swv * dt;
        if !t.alive {
            if !t.landed || t.fall_v.abs() > 0.02 {
                t.fall_v += dt * (2.5 + 9.0 * t.fall);
                t.fall += t.fall_v * dt;
                if t.fall >= 1.0 {
                    t.fall = 1.0;
                    if t.fall_v > 0.9 { if !t.landed { sfx.write(Sfx::Fall(Vec3::new(t.def.x, 0.3, t.def.z))); } t.fall_v *= -0.22; } else { t.fall_v = 0.0; }
                    t.landed = true;
                }
            }
            continue;
        }
        if t.def.kind != TKind::Threat || t.def.passive { continue; }
        let dv = p.pos - Vec2::new(t.def.x, t.def.z);
        let d = dv.length();
        let mut rate = 0.0;
        if d < 20.0 && d > 0.01 && dv.dot(t.fwd) / d > 0.15 {
            if level.los(t.def.x, t.def.z, 1.4, p.pos.x, p.pos.y, head - 0.25) { rate = 1.0; }
            else if level.los(t.def.x, t.def.z, 1.4, p.eye.x, p.eye.z, p.eye.y) { rate = 0.45; }
        }
        if std::env::var("CQB_NOREACT").is_ok() { rate = 0.0; }
        if rate > 0.0 {
            exposed = true;
            if !t.contact { t.contact = true; t.contact_t = game.time; }
            t.react += dt * rate;
            if t.react >= REACT {
                t.react = REACT - 1.4;
                game.stats.hits_taken += 1; game.dmg_t = 1.3; game.shake = 1.0;
                let src = Vec3::new(t.def.x + t.fwd.x * 0.25, 1.3, t.def.z + t.fwd.y * 0.25);
                sfx.write(Sfx::EnemyShot(src)); sfx.write(Sfx::Hurt);
                // o disparo da ameaça aparece: clarão na boca do fuzil e traço até o operador
                let to = (Vec3::new(p.eye.x, p.eye.y - 0.3, p.eye.z) - Vec3::new(t.def.x, 1.45, t.def.z)).normalize_or_zero();
                let mz = Vec3::new(t.def.x, 1.45, t.def.z) + Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 0.72;
                game.tracers.push((mz, Vec3::new(p.eye.x, p.eye.y - 0.3, p.eye.z) - to * 0.45, 0.1));
                puff(&mut commands, &fx, &mut smats, mz, to, 3, Color::srgb(1.0, 0.82, 0.45), 2.4, 0.11);
                let seen_now = t.vis;
                game.toast(if seen_now { "Hit (simulated). The threat had a clear line of fire for too long: identify and neutralize faster." }
                    else { "Hit (simulated) by a threat you were not looking at. There was uncleared area with a line of fire to you." }, 2, 6.5);
            }
        } else { t.react = (t.react - dt * 0.6).max(0.0); }
        if t.contact && t.first_facing.is_none() && game.time - t.contact_t > 0.35 { t.first_facing = Some(t.seen); }
    }
    // tempo exposto: enquanto alguma ameaça viva tem linha de tiro para o operador
    if exposed { game.expo_now += dt; game.expo_total += dt; } else { game.expo_now = (game.expo_now - dt * 2.0).max(0.0); }
    for (mut tf, tp) in pivots.iter_mut() {
        let t = &targets.list[tp.0];
        // papelão tomba em torno da base; boneco não tomba como placa: quem mostra a queda é o clipe de animação dele
        tf.rotation = if is_fig(&t.def) { Quat::from_rotation_x(t.sw * 0.03 * (1.0 - t.fall)) } else { Quat::from_rotation_x(t.fall_dir * t.fall_max * t.fall + t.sw * 0.06 * (1.0 - t.fall)) };
    }
}

pub fn rand01(seed: f32) -> f32 { ((seed.sin() * 43758.547).fract()).abs() }

pub fn update_fx(time: Res<Time>, mut commands: Commands, mut puffs: Query<(Entity, &mut Transform, &mut Puff, &MeshMaterial3d<StandardMaterial>), Without<Casing>>,
    mut casings: Query<(Entity, &mut Transform, &mut Casing), Without<Puff>>, mut smats: ResMut<Assets<StandardMaterial>>,
    cam: Query<(&GlobalTransform, &Camera), With<Camera3d>>, mut sfx: EventWriter<Sfx>) {
    let dt = time.delta_secs().min(0.05);
    let face = cam.iter().find(|(_, c)| c.is_active && c.order == 0).map(|(g, _)| g.compute_transform().rotation).unwrap_or(Quat::IDENTITY);
    for (e, mut tf, mut pf, mat) in puffs.iter_mut() {
        pf.life -= dt;
        if pf.life <= 0.0 { commands.entity(e).despawn(); continue; }
        tf.translation += pf.vel * dt;
        if pf.grav > 0.0 { pf.vel.y -= pf.grav * dt; if tf.translation.y < 0.02 { pf.vel = Vec3::ZERO; } } else { pf.vel *= (-4.0 * dt).exp(); }
        let s = (tf.scale.x + pf.grow * dt).max(0.002);
        tf.scale = Vec3::splat(s);
        tf.rotation = face;
        if pf.grav == 0.0 { if let Some(m) = smats.get_mut(&mat.0) { m.base_color.set_alpha(pf.op * (pf.life / pf.max)); } }
    }
    for (e, mut tf, mut c) in casings.iter_mut() {
        c.life -= dt;
        if c.life <= 0.0 { commands.entity(e).despawn(); continue; }
        if c.rest { continue; }
        c.vel.y -= 9.8 * dt;
        tf.translation += c.vel * dt;
        tf.rotate(Quat::from_euler(EulerRot::XYZ, c.ang.x * dt, c.ang.y * dt, c.ang.z * dt));
        if tf.translation.y < 0.006 {
            tf.translation.y = 0.006;
            if c.vel.y.abs() > 0.8 { c.vel.y *= -0.35; c.vel.x *= 0.5; c.vel.z *= 0.5; c.ang *= 0.5; sfx.write(Sfx::Casing(tf.translation)); }
            else { c.rest = true; tf.rotation = Quat::from_rotation_y(c.ang.x) * Quat::from_rotation_z(FRAC_PI_2); }
        }
    }
}
