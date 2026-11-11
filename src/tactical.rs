//! Visão tática: área coberta agora, área já vista, área não limpa, ângulo oculto por quina e exposição do corpo.
use crate::art;
use crate::map::*;
use crate::player::FpCam;
use crate::state::*;
use crate::world::*;
use bevy::asset::RenderAssetUsages;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::view::{NoFrustumCulling, RenderLayers};
use bevy::window::PrimaryWindow;
use std::f32::consts::PI;

pub const SECTOR_READY: f32 = 70.0;
pub const SECTOR_AIM: f32 = 32.0;
pub const PPM: f32 = 16.0;
pub const GW: usize = 352;
pub const GH: usize = 304;

#[derive(Component)] pub struct OverlayPlane;
#[derive(Component)] pub struct FogPlane;

#[derive(Resource)]
pub struct Tac {
    pub vis: Vis, pub wedge: Wedge, pub hfov: f32, pub c: f32,
    pub seen: Vec<u8>, pub cur: Vec<u8>, pub expo: Vec<u8>, pub zone_idx: Vec<i8>, pub tot: [u32; MAX_ZONES], pub pct: [f32; MAX_ZONES],
    pub expo_deg: f32, pub stat_t: f32, pub overlay: Handle<Image>, pub fog: Handle<Image>, pub wedge_mesh: Handle<Mesh>,
    pub overlay_mat: Handle<StandardMaterial>, pub fog_mat: Handle<StandardMaterial>,
    pub primary: Option<Edge>, pub body_cm: f32, pub corner_zone: Option<&'static str>,
    /// Fatias já abertas de cada abertura: 180 casas de 1°. `pie_now` é o que se vê neste quadro; `pie_active`, a abertura em foco.
    pub pie: Vec<[u8; 180]>, pub pie_now: Vec<Option<(f32, f32)>>, pub pie_active: Option<usize>,
    /// Inclinar para este lado (-1 = Q, +1 = E) põe mais graus da abertura em foco dentro do setor: (lado, ganho em graus).
    pub lean_hint: Option<(f32, f32)>,
}
impl Tac {
    pub fn pie_deg(&self, i: usize) -> f32 { self.pie[i].iter().filter(|b| **b > 0).count() as f32 }
    /// Refaz o que depende do mapa: a que cômodo pertence cada célula da grade e quantas aberturas existem.
    pub fn rebuild(&mut self, level: &Level) {
        self.tot = [0; MAX_ZONES];
        for k in 0..GW * GH {
            let c = cell_center(k);
            let mut zi = -1i8;
            for (n, q) in zones().iter().enumerate() {
                if c.x > q.x1 + 0.2 && c.x < q.x2 - 0.2 && c.y > q.z1 + 0.2 && c.y < q.z2 - 0.2 { zi = n as i8; break; }
            }
            if zi >= 0 && level.props.iter().any(|p| p.h > 1.5 && c.x > p.x1 - 0.12 && c.x < p.x2 + 0.12 && c.y > p.z1 - 0.12 && c.y < p.z2 + 0.12) { zi = -1; }
            self.zone_idx[k] = zi;
            if zi >= 0 { self.tot[zi as usize] += 1; }
        }
        self.pie = vec![[0u8; 180]; openings().len()];
        self.pie_now = vec![None; openings().len()];
        self.corner_zone = None;
        self.reset();
    }
    pub fn reset(&mut self) {
        for p in self.pie.iter_mut() { *p = [0u8; 180]; }
        for p in self.pie_now.iter_mut() { *p = None; }
        self.pie_active = None; self.lean_hint = None;
        self.seen.fill(0); self.cur.fill(0); self.expo.fill(0); self.pct = [0.0; MAX_ZONES]; self.expo_deg = 0.0; self.stat_t = 0.0; self.primary = None;
    }
    #[inline] pub fn cell(x: f32, z: f32) -> Option<usize> {
        let (i, j) = (((x - MAP_X1) * PPM).floor() as i32, ((z - MAP_Z1) * PPM).floor() as i32);
        if i < 0 || j < 0 || i >= GW as i32 || j >= GH as i32 { None } else { Some(j as usize * GW + i as usize) }
    }
}
fn cell_center(k: usize) -> Vec2 { Vec2::new(MAP_X1 + ((k % GW) as f32 + 0.5) / PPM, MAP_Z1 + ((k / GW) as f32 + 0.5) / PPM) }

pub fn setup_tactical(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>, level: Res<Level>) {
    let mk = |images: &mut Assets<Image>, a: u8| {
        let mut data = vec![0u8; GW * GH * 4];
        for p in data.chunks_exact_mut(4) { p[3] = a; }
        images.add(art::rgba_image(GW as u32, GH as u32, data, true, false, false))
    };
    let overlay = mk(&mut images, 0);
    let fog = mk(&mut images, 255);
    let (w, d, cx, cz) = (MAP_X2 - MAP_X1, MAP_Z2 - MAP_Z1, (MAP_X1 + MAP_X2) / 2.0, (MAP_Z1 + MAP_Z2) / 2.0);
    let om = mats.add(StandardMaterial { base_color_texture: Some(overlay.clone()), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..default() });
    commands.spawn((Mesh3d(meshes.add(Plane3d::default().mesh().size(w, d))), MeshMaterial3d(om.clone()), Transform::from_xyz(cx, 0.02, cz), NotShadowCaster, NotShadowReceiver, Visibility::Hidden, OverlayPlane, RenderLayers::layer(L_FP)));
    // névoa de guerra das vistas de cima: plano grande, com a borda da textura (opaca) cobrindo o entorno
    let r = 70.0;
    let mut fm = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    let corners = [(cx - r, cz - r), (cx - r, cz + r), (cx + r, cz + r), (cx + r, cz - r)];
    fm.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners.iter().map(|c| [c.0, 0.0, c.1]).collect::<Vec<_>>());
    fm.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
    fm.insert_attribute(Mesh::ATTRIBUTE_UV_0, corners.iter().map(|c| [(c.0 - MAP_X1) / w, (c.1 - MAP_Z1) / d]).collect::<Vec<_>>());
    fm.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    let fmat = mats.add(StandardMaterial { base_color_texture: Some(fog.clone()), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..default() });
    if std::env::var("CQB_NOFOW").is_err() {
        commands.spawn((Mesh3d(meshes.add(fm)), MeshMaterial3d(fmat.clone()), Transform::from_xyz(0.0, 3.6, 0.0), RenderLayers::layer(L_TOP), NotShadowCaster, NotShadowReceiver, NoFrustumCulling, FogPlane));
    }
    let mut wm = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    wm.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32, 0.0, 0.0]; 3]);
    wm.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; 3]);
    wm.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; 3]);
    wm.insert_indices(Indices::U32(vec![0, 1, 2]));
    let wedge_mesh = meshes.add(wm);
    let wmat = mats.add(StandardMaterial { base_color: Color::srgba(0.38, 0.84, 0.91, 0.24), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..default() });
    commands.spawn((Mesh3d(wedge_mesh.clone()), MeshMaterial3d(wmat), Transform::from_xyz(0.0, 3.7, 0.0), RenderLayers::layer(L_TOP), NotShadowCaster, NotShadowReceiver, NoFrustumCulling));
    let mut tac = Tac {
        vis: Vis::default(), wedge: Wedge::default(), hfov: 1.7, c: 0.0, seen: vec![0; GW * GH], cur: vec![0; GW * GH], expo: vec![0; GW * GH], zone_idx: vec![-1i8; GW * GH], tot: [0; MAX_ZONES], pct: [0.0; MAX_ZONES],
        expo_deg: 0.0, stat_t: 0.0, overlay, fog, wedge_mesh, overlay_mat: om, fog_mat: fmat, primary: None, body_cm: 25.0, corner_zone: None, pie: vec![[0u8; 180]; openings().len()], pie_now: vec![None; openings().len()], pie_active: None, lean_hint: None,
    };
    tac.rebuild(&level);
    commands.insert_resource(tac);
}

pub fn tac_update(
    time: Res<Time>, game: Res<Game>, p: Res<Player>, mut level: ResMut<Level>, mut tac: ResMut<Tac>, mut targets: ResMut<Targets>,
    mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, proj: Query<&Projection, With<FpCam>>, windows: Query<&Window, With<PrimaryWindow>>,
    mut overlay: Query<&mut Visibility, With<OverlayPlane>>, mut mats: ResMut<Assets<StandardMaterial>>,
) {
    if let Ok(mut v) = overlay.single_mut() {
        let want = if game.tac && game.mode != Mode::Menu { Visibility::Visible } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
    if game.mode != Mode::Play || std::env::var("CQB_NOTAC").is_ok() { return; }
    let dt = time.delta_secs().min(0.05);
    let aspect = windows.single().map(|w| w.width() / w.height().max(1.0)).unwrap_or(1.78);
    let vfov = match proj.single() { Ok(Projection::Perspective(pp)) => pp.fov, _ => 68f32.to_radians() };
    // Setor realmente coberto: não é a tela inteira, e sim a visão central para onde olhos e arma apontam.
    // Cerca de 70° com a arma em pronto (onde dá para identificar um alvo) e 32° olhando pela mira.
    let _ = (aspect, vfov);
    let aim = p.g_ads * p.g_ads * (3.0 - 2.0 * p.g_ads);
    let hfov = (SECTOR_READY + (SECTOR_AIM - SECTOR_READY) * aim).to_radians();
    let (eye, eye_h, f) = (p.eye2(), p.eye.y, p.fwd());
    let c = f.y.atan2(f.x);
    let vis = compute_vis(&level, eye.x, eye.y, eye_h);
    let wedge = compute_wedge(&level, &vis, c, hfov / 2.0, eye_h);
    let tac = tac.as_mut();
    tac.hfov = hfov; tac.c = c;

    // área coberta neste quadro e acumulada
    tac.cur.fill(0);
    let (mut x1, mut x2, mut z1, mut z2) = (eye.x, eye.x, eye.y, eye.y);
    for q in &wedge.pts { x1 = x1.min(q.x); x2 = x2.max(q.x); z1 = z1.min(q.y); z2 = z2.max(q.y); }
    let i1 = (((x1 - MAP_X1) * PPM).floor() as i32 - 3).clamp(0, GW as i32 - 1) as usize;
    let i2 = (((x2 - MAP_X1) * PPM).ceil() as i32 + 3).clamp(0, GW as i32 - 1) as usize;
    let j1 = (((z1 - MAP_Z1) * PPM).floor() as i32 - 3).clamp(0, GH as i32 - 1) as usize;
    let j2 = (((z2 - MAP_Z1) * PPM).ceil() as i32 + 3).clamp(0, GH as i32 - 1) as usize;
    for j in j1..=j2 {
        for i in i1..=i2 {
            let k = j * GW + i;
            if wedge.sees(eye, cell_center(k), 0.16) { tac.cur[k] = 1; tac.seen[k] = 255; }
        }
    }
    if std::env::var("CQB_CURLOG").is_ok() { println!("BOT CUR {:.1} {:.1} {:.4} {} {}", eye.x, eye.y, c, tac.cur.iter().filter(|v| **v > 0).count(), wedge.pts.len()); }
    // alvos e cantos vistos
    for t in targets.list.iter_mut() {
        let tp = Vec2::new(t.def.x, t.def.z);
        t.vis = wedge.sees(eye, tp, 0.05) && level.los(eye.x, eye.y, eye_h, tp.x, tp.y, 1.3);
        if t.vis { t.seen = true; }
    }
    if let Some(z) = tac.corner_zone {
        for k in level.corners.iter_mut().filter(|k| k.zone == z && !k.seen) { if wedge.sees(eye, Vec2::new(k.x, k.z), 0.05) { k.seen = true; } }
    }
    // quina principal: a mais próxima com setor ainda oculto, e quanto do corpo aparece para quem estiver nele
    let prim = wedge.edges.iter().filter(|e| e.hidden_deg > 2.0 && e.dist < 9.0).min_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap()).copied();
    tac.body_cm = match prim {
        Some(e) => {
            let side = e.sdir.perp_dot(e.hdir).signum();
            let d = e.sdir.perp_dot(p.pos - eye) * side;
            (BODY_R - d).clamp(0.09, BODY_R * 2.0) * 100.0
        }
        None => 0.0,
    };
    tac.primary = prim;
    // fatias de cada abertura: o que passa pelo vão E está dentro do setor coberto
    let psi = |o: &Opening| f.dot(o.n).atan2(f.dot(o.w())).to_degrees();
    let mut best: Option<(usize, f32)> = None;
    for (i, o) in openings().iter().enumerate() {
        tac.pie_now[i] = None;
        let mid = o.mid();
        let dist = eye.distance(mid);
        let closed = o.door.map_or(false, |d| level.doors[d].open < 0.5);
        let outside = (eye - mid).dot(o.n) < -0.12;
        if !outside || dist > 9.0 { continue; }
        let zi = zone_index(o.zone);
        if tac.pct[zi] < 0.95 && dist < 7.0 && best.map_or(true, |b| dist < b.1) { best = Some((i, dist)); }
        if closed || !level.los(eye.x, eye.y, eye_h, mid.x, mid.y, 1.2) { continue; }
        if let Some((lo, hi)) = o.fan(eye) {
            let c0 = psi(o);
            let half = (hfov / 2.0).to_degrees();
            let (lo, hi) = (lo.max(c0 - half), hi.min(c0 + half));
            if hi - lo > 0.3 {
                tac.pie_now[i] = Some((lo, hi));
                for b in (lo.floor().max(0.0) as usize)..(hi.ceil().min(180.0) as usize) { tac.pie[i][b] = 1; }
            }
        }
    }
    tac.pie_active = best.map(|b| b.0);
    // dica: quantos graus a mais da abertura em foco entram no setor inclinando para cada lado
    tac.lean_hint = None;
    if let Some(i) = tac.pie_active {
        let o = &openings()[i];
        let open = o.door.map_or(true, |d| level.doors[d].open >= 0.5);
        if open && p.lean.abs() < 0.3 {
            let half = (hfov / 2.0).to_degrees();
            let c0 = psi(o);
            let width = |e: Vec2| -> f32 {
                if !level.los(e.x, e.y, eye_h, o.mid().x, o.mid().y, 1.2) { return 0.0; }
                o.fan(e).map_or(0.0, |(lo, hi)| (hi.min(c0 + half) - lo.max(c0 - half)).max(0.0))
            };
            let now = width(eye);
            let r = p.right();
            for side in [-1.0f32, 1.0] {
                let d = level.cast_ray(p.pos.x, p.pos.y, r.x * side, r.y * side, 1.2, eye_h - 0.35);
                let lim = ((d - crate::player::LEAN_GAP) / crate::player::LEAN_M).clamp(0.0, 1.0);
                let gain = width(p.pos + r * (side * lim * crate::player::LEAN_M)) - now;
                if gain >= 3.0 && tac.lean_hint.map_or(true, |h| gain > h.1) { tac.lean_hint = Some((side, gain)); }
            }
        }
    }

    tac.stat_t -= dt;
    if tac.stat_t <= 0.0 {
        tac.stat_t = 0.1;
        let mut cnt = [0u32; MAX_ZONES];
        for k in 0..GW * GH {
            let z = tac.zone_idx[k];
            if z < 0 { tac.expo[k] = 0; continue; }
            if tac.seen[k] > 0 { cnt[z as usize] += 1; tac.expo[k] = 0; }
            else { tac.expo[k] = if vis_sees(&vis, cell_center(k), 0.0) { 1 } else { 0 }; }
        }
        for z in 0..MAX_ZONES { tac.pct[z] = if tac.tot[z] > 0 { cnt[z] as f32 / tac.tot[z] as f32 } else { 1.0 }; }
        // exposição: direções fora do campo de visão que dão para área não limpa
        let mut expo = 0.0;
        for n in 0..120 {
            let a = -PI + n as f32 * 3f32.to_radians();
            if ang_diff(a, c).abs() < hfov / 2.0 { continue; }
            let (dx, dz) = (a.cos(), a.sin());
            let dist = level.cast_ray(eye.x, eye.y, dx, dz, VIS_RANGE, eye_h);
            let mut t = 0.45;
            while t < dist - 0.1 {
                match Tac::cell(eye.x + dx * t, eye.y + dz * t) {
                    Some(k) => if tac.zone_idx[k] >= 0 && tac.seen[k] == 0 { expo += 3.0; break; },
                    None => break,
                }
                t += 0.3;
            }
        }
        tac.expo_deg = expo;
    }

    // texturas: sobreposição no chão (primeira pessoa) e névoa das vistas de cima, as duas com borda suavizada
    let n = GW * GH;
    let blur = |src: &[u16]| -> Vec<u16> {
        let rad = 2i32;
        let div = 2 * rad as u32 + 1;
        let mut tmp = vec![0u16; n];
        for j in 0..GH { for i in 0..GW {
            let mut acc = 0u32;
            for d in -rad..=rad { acc += src[j * GW + (i as i32 + d).clamp(0, GW as i32 - 1) as usize] as u32; }
            tmp[j * GW + i] = (acc / div) as u16;
        } }
        let mut out = vec![0u16; n];
        for j in 0..GH { for i in 0..GW {
            let mut acc = 0u32;
            for d in -rad..=rad { acc += tmp[(j as i32 + d).clamp(0, GH as i32 - 1) as usize * GW + i] as u32; }
            out[j * GW + i] = (acc / div) as u16;
        } }
        out
    };
    // chão: verde bem leve no que já foi visto, vermelho translúcido no que falta (mais forte onde há linha de tiro para você)
    if game.tac {
        let (mut red, mut green) = (vec![0u16; n], vec![0u16; n]);
        for k in 0..n {
            if tac.zone_idx[k] < 0 { continue; }
            if tac.seen[k] > 0 { green[k] = 16; } else { red[k] = if tac.expo[k] > 0 { 88 } else { 46 }; }
        }
        let (red, green) = (blur(&red), blur(&green));
        if let Some(img) = images.get_mut(&tac.overlay) {
            if let Some(data) = img.data.as_mut() {
                for k in 0..n {
                    let (r, g) = (red[k] as f32, green[k] as f32);
                    let w = (r + g).max(1.0);
                    data[k * 4..k * 4 + 4].copy_from_slice(&[((225.0 * r + 98.0 * g) / w) as u8, ((62.0 * r + 214.0 * g) / w) as u8, ((50.0 * r + 140.0 * g) / w) as u8, (r + g).min(255.0) as u8]);
                }
            }
        }
    }
    // névoa das vistas de cima (vista aérea e minimapa). Com a visão tática ligada ela leva as cores da legenda, em tons
    // escuros e com borda suave: verde no que já foi visto, vinho no que falta limpar (mais vivo onde há linha de tiro
    // para você). Sem a visão tática fica neutra. Na demonstração fica translúcida, para acompanhar a casa e o traçado.
    // O entorno imediato do operador fica sempre visível.
    let dark: u16 = if game.demo { 172 } else { 255 };
    let mut raw = vec![0u16; n];
    let (mut fr, mut fg) = (vec![0u16; n], vec![0u16; n]);
    for k in 0..n {
        let (cur, seen, zone) = (tac.cur[k] > 0, tac.seen[k] > 0, tac.zone_idx[k] >= 0);
        raw[k] = if cur { 0 } else if seen { 118.min(dark) } else { dark };
        if game.tac && zone && !cur {
            if seen { fg[k] = 255; } else { fr[k] = if tac.expo[k] > 0 { 255 } else { 115 }; }
        }
    }
    let (pi, pj) = (((p.pos.x - MAP_X1) * PPM) as i32, ((p.pos.y - MAP_Z1) * PPM) as i32);
    for j in (pj - 9).max(0)..=(pj + 9).min(GH as i32 - 1) { for i in (pi - 9).max(0)..=(pi + 9).min(GW as i32 - 1) {
        let k = j as usize * GW + i as usize;
        if cell_center(k).distance(p.pos) < 0.5 { raw[k] = 0; fr[k] = 0; fg[k] = 0; }
    } }
    let (alpha, fr, fg) = (blur(&raw), blur(&fr), blur(&fg));
    if let Some(img) = images.get_mut(&tac.fog) {
        if let Some(data) = img.data.as_mut() {
            for k in 0..n {
                let (r, g) = (fr[k] as f32 / 255.0, fg[k] as f32 / 255.0);
                data[k * 4..k * 4 + 4].copy_from_slice(&[(104.0 * r + 14.0 * g) as u8, (20.0 * r + 118.0 * g) as u8, (18.0 * r + 64.0 * g) as u8, alpha[k] as u8]);
            }
        }
    }
    // o material precisa ser marcado como alterado para passar a usar a textura atualizada
    let _ = mats.get_mut(&tac.fog_mat);
    if game.tac { let _ = mats.get_mut(&tac.overlay_mat); }
    // fatia coberta agora (malha exata, para as vistas de cima)
    if let Some(m) = meshes.get_mut(&tac.wedge_mesh) {
        let n = wedge.pts.len();
        let mut pos: Vec<[f32; 3]> = Vec::with_capacity(n + 1);
        pos.push([eye.x, 0.0, eye.y]);
        for q in &wedge.pts { pos.push([q.x, 0.0, q.y]); }
        let mut idx: Vec<u32> = Vec::with_capacity(n * 3);
        for i in 1..n as u32 { idx.extend_from_slice(&[0, i, i + 1]); }
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; pos.len()]);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; pos.len()]);
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_indices(Indices::U32(idx));
    }
    tac.vis = vis; tac.wedge = wedge;
}

pub fn tac_gizmos(mut gz: Gizmos, mut game: ResMut<Game>, time: Res<Time>, p: Res<Player>, tac: Res<Tac>, level: Res<Level>, targets: Res<Targets>) {
    // traçantes aparecem sempre
    let dt = time.delta_secs();
    for t in game.tracers.iter_mut() { t.2 -= dt; }
    game.tracers.retain(|t| t.2 > 0.0);
    for t in &game.tracers { gz.line(t.0, t.1, Color::linear_rgba(4.0, 3.0, 1.2, (t.2 / 0.06).clamp(0.0, 1.0))); }
    if game.mode == Mode::Play {
        let rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        for r in [0.47f32, 0.485, 0.5] { gz.circle(Isometry3d::new(Vec3::new(p.pos.x, 3.87, p.pos.y), rot), r, Color::srgb(0.38, 0.84, 0.91)).resolution(48); }
    }
    if !game.tac || game.mode != Mode::Play { return; }
    for y in [0.06f32, 3.8] {
    let v3 = |q: Vec2| Vec3::new(q.x, y, q.y);
    let eye = p.eye2();
    let w = &tac.wedge;
    if w.pts.len() < 2 { return; }
    let cyan = Color::srgb(0.38, 0.84, 0.91);
    gz.line(v3(eye), v3(w.pts[0]), cyan);
    gz.line(v3(eye), v3(*w.pts.last().unwrap()), cyan);
    let arc = |gz: &mut Gizmos, o: Vec2, a0: f32, da: f32, r: f32, col: Color| {
        let n = ((da.abs() / 0.12).ceil() as usize).max(2);
        let mut prev = o + Vec2::new(a0.cos(), a0.sin()) * r;
        for i in 1..=n {
            let a = a0 + da * i as f32 / n as f32;
            let q = o + Vec2::new(a.cos(), a.sin()) * r;
            gz.line(v3(prev), v3(q), col); prev = q;
        }
    };
    arc(&mut gz, eye, w.c - w.h, w.h * 2.0, 1.0, cyan.with_alpha(0.7));
    let f = p.fwd();
    let md = level.cast_ray(eye.x, eye.y, f.x, f.y, VIS_RANGE, p.eye.y);
    gz.line(v3(eye + f * 0.4), v3(eye + f * md), Color::srgba(1.0, 1.0, 1.0, 0.45));
    // limite de visão de cada quina
    let amber = Color::srgb(1.0, 0.82, 0.23);
    let red = Color::srgb(1.0, 0.27, 0.2);
    for e in &w.edges { gz.line(v3(e.a), v3(e.b), amber); }
    // leque de 180° de cada abertura: verde = já fatiado, azul = vendo agora, vermelho = falta
    let green = Color::srgb(0.44, 0.84, 0.56);
    for (i, o) in openings().iter().enumerate() {
        let zi = zone_index(o.zone);
        let any = tac.pie[i].iter().any(|b| *b > 0);
        let near = eye.distance(o.mid()) < 8.0 && (eye - o.mid()).dot(o.n) < 0.0;
        if tac.pct[zi] >= 0.95 || !(any || near) || o.door.map_or(false, |d| level.doors[d].open < 0.5) { continue; }
        let c = o.mid() + o.n * (WALL_T / 2.0);
        let r = 1.5;
        for b in (0..180).step_by(3) {
            let now = tac.pie_now[i].map_or(false, |(lo, hi)| (b as f32) >= lo - 1.5 && (b as f32) <= hi + 1.5);
            let seen = tac.pie[i][b] > 0 || tac.pie[i][(b + 2).min(179)] > 0;
            let col = if now { cyan } else if seen { green } else { red };
            let (p0, p1) = (c + o.dir(b as f32) * r, c + o.dir(b as f32 + 3.0) * r);
            gz.line(v3(p0), v3(p1), col);
            if !seen { gz.line(v3(c + o.dir(b as f32 + 1.5) * (r - 0.14)), v3(c + o.dir(b as f32 + 1.5) * r), red.with_alpha(0.7)); }
        }
        if let Some((lo, hi)) = tac.pie_now[i] { gz.line(v3(c), v3(c + o.dir(lo) * r), cyan); gz.line(v3(c), v3(c + o.dir(hi) * r), cyan); }
    }
    if let Some(z) = tac.corner_zone {
        let rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        for k in level.corners.iter().filter(|k| k.zone == z) {
            let col = if k.seen { Color::srgb(0.38, 0.84, 0.55) } else { red };
            gz.circle(Isometry3d::new(Vec3::new(k.x, y, k.z), rot), 0.2, col);
            if !k.seen { gz.circle(Isometry3d::new(Vec3::new(k.x, y, k.z), rot), 0.12 + 0.05 * (time.elapsed_secs() * 6.0).sin(), col); }
        }
    }
    // marcadores de alvo já vistos
    for t in targets.list.iter().filter(|t| t.seen) {
        let c = Vec2::new(t.def.x, t.def.z);
        let s = 0.22;
        if t.alive {
            let col = Color::srgb(0.95, 0.7, 0.23);
            let pts = [c + Vec2::new(0.0, -s), c + Vec2::new(s, 0.0), c + Vec2::new(0.0, s), c + Vec2::new(-s, 0.0)];
            for i in 0..4 { gz.line(v3(pts[i]), v3(pts[(i + 1) % 4]), col); }
            gz.line(v3(c + t.fwd * 0.3), v3(c + t.fwd * 0.65), col);
        } else {
            let col = if t.def.kind == TKind::Civil { red } else { Color::srgb(0.55, 0.6, 0.62) };
            gz.line(v3(c + Vec2::new(-s, -s)), v3(c + Vec2::new(s, s)), col);
            gz.line(v3(c + Vec2::new(s, -s)), v3(c + Vec2::new(-s, s)), col);
        }
    }
    }
}
