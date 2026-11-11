//! Luz rebatida: um rebatimento difuso das luminárias de teto, calculado na abertura e guardado numa grade 3D
//! (volume de irradiância). Ilumina paredes, forro, móveis e a arma com a luz que piso e paredes devolvem.
use crate::map::*;
use crate::world::{env_f, CEIL_LUMENS};
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::irradiance_volume::IrradianceVolume;
use bevy::pbr::LightProbe;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const VOX: f32 = 0.5;
const RX: usize = 44;
const RY: usize = 6;
const RZ: usize = 38;

/// Ponto de luz virtual: um pedaço de superfície iluminado diretamente, que passa a emitir luz difusa.
#[derive(Clone, Copy)]
struct Vpl { p: Vec3, n: Vec3, rad: Vec3 }

/// Linha de visada em que parede bloqueia em qualquer altura (o bloqueador de sombra vai até acima das luminárias).
fn clear(level: &Level, a: Vec3, b: Vec3) -> bool {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    let d = (dx * dx + dz * dz).sqrt();
    if d < 1e-4 { return true; }
    let (ux, uz) = (dx / d, dz / d);
    for s in &level.solids {
        if !s.on && s.kind != SolidKind::Door { continue; }
        if s.kind == SolidKind::Door { continue; }
        if let Some((t0, t1)) = ray_box(a.x, a.z, ux, uz, s) {
            if t0 >= d - 0.03 { continue; }
            if s.kind == SolidKind::Wall { return false; }
            let t1 = t1.min(d);
            let (h0, h1) = (a.y + (b.y - a.y) * (t0 / d), a.y + (b.y - a.y) * (t1 / d));
            if h0.min(h1) < s.h { return false; }
        }
    }
    true
}

fn direct(level: &Level, p: Vec3, n: Vec3) -> Vec3 {
    let mut e = Vec3::ZERO;
    for l in LIGHTS.iter() {
        let lp = Vec3::new(l.x, 3.9, l.z);
        let v = lp - p;
        let d2 = v.length_squared();
        if d2 > 81.0 { continue; }
        let dir = v / d2.sqrt();
        let cos_s = n.dot(dir);
        if cos_s <= 0.0 { continue; }
        // cone da luminária: mesma abertura usada na cena
        let cos_l = dir.y;
        let (ci, co) = (0.75f32.cos(), 1.2f32.cos());
        let spot = ((cos_l - co) / (ci - co)).clamp(0.0, 1.0);
        if spot <= 0.0 || !clear(level, lp, p + n * 0.05) { continue; }
        let cd = CEIL_LUMENS * l.i / (4.0 * std::f32::consts::PI);
        e += Vec3::from(l.color) * (cd * cos_s * spot * spot * (3.0 - 2.0 * spot) / d2);
    }
    e
}

pub fn setup_gi(mut commands: Commands, mut images: ResMut<Assets<Image>>, level: Res<Level>) {
    // Desligado por padrão: o motor aplicava o volume só em alguns quadros (a cena "piscava" mais clara).
    // A luz ambiente fixa em world.rs faz o papel do rebatimento. CQB_GIVOL=1 religa para teste.
    if std::env::var("CQB_GIVOL").is_err() { return; }
    let t0 = std::time::Instant::now();
    // 1) pontos de luz virtuais no piso e nas paredes
    let mut vpls: Vec<Vpl> = Vec::new();
    let step = 0.8;
    for q in ZONES.iter() {
        let (nx, nz) = (((q.x2 - q.x1) / step).round().max(1.0) as usize, ((q.z2 - q.z1) / step).round().max(1.0) as usize);
        let (sx, sz) = ((q.x2 - q.x1) / nx as f32, (q.z2 - q.z1) / nz as f32);
        for i in 0..nx { for j in 0..nz {
            let p = Vec3::new(q.x1 + (i as f32 + 0.5) * sx, 0.0, q.z1 + (j as f32 + 0.5) * sz);
            let e = direct(&level, p, Vec3::Y);
            if e.max_element() > 1.0 { vpls.push(Vpl { p, n: Vec3::Y, rad: e * (0.24 * sx * sz / std::f32::consts::PI) }); }
        } }
    }
    for w in &level.walls {
        let (wx, wz) = (w.x2 - w.x1, w.z2 - w.z1);
        let horiz = wx > wz;
        let len = if horiz { wx } else { wz };
        let n = (len / step).round().max(1.0) as usize;
        let seg = len / n as f32;
        for k in 0..n {
            let along = (k as f32 + 0.5) * seg;
            for side in [-1.0f32, 1.0] {
                let (p2, nrm) = if horiz { (Vec2::new(w.x1 + along, if side < 0.0 { w.z1 } else { w.z2 }), Vec3::new(0.0, 0.0, side)) } else { (Vec2::new(if side < 0.0 { w.x1 } else { w.x2 }, w.z1 + along), Vec3::new(side, 0.0, 0.0)) };
                if zone_at(p2.x + nrm.x * 0.15, p2.y + nrm.z * 0.15).is_none() { continue; }
                // faixa de baixo (pintura escura) e faixa de cima (clara)
                for (y, hgt, alb) in [(0.55f32, 1.05f32, Vec3::new(0.16, 0.2, 0.17)), (1.9, 1.75, Vec3::new(0.46, 0.44, 0.39))] {
                    let p = Vec3::new(p2.x, y, p2.y);
                    let e = direct(&level, p, nrm);
                    if e.max_element() > 1.0 { vpls.push(Vpl { p, n: nrm, rad: e * alb * (seg * hgt / std::f32::consts::PI) }); }
                }
            }
        }
    }
    // 2) irradiância indireta em cada voxel, para as seis direções
    let dirs = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let nvox = RX * RY * RZ;
    let mut val = vec![[Vec3::ZERO; 6]; nvox];
    let mut ok = vec![false; nvox];
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(12);
    let chunk = nvox.div_ceil(threads);
    let (lv, vp) = (&*level, &vpls);
    std::thread::scope(|sc| {
        for (ci, (vc, oc)) in val.chunks_mut(chunk).zip(ok.chunks_mut(chunk)).enumerate() {
            sc.spawn(move || {
                for (k, (v, o)) in vc.iter_mut().zip(oc.iter_mut()).enumerate() {
                    let idx = ci * chunk + k;
                    let (x, y, z) = (idx % RX, (idx / RX) % RY, idx / (RX * RY));
                    let p = Vec3::new(MAP_X1 + (x as f32 + 0.5) * VOX, (y as f32 + 0.5) * VOX, MAP_Z1 + (z as f32 + 0.5) * VOX);
                    if zone_at(p.x, p.z).is_none() || lv.solids.iter().any(|s| s.kind == SolidKind::Wall && p.x > s.x1 && p.x < s.x2 && p.z > s.z1 && p.z < s.z2) { continue; }
                    *o = true;
                    for l in vp.iter() {
                        let d = l.p - p;
                        let d2 = d.length_squared();
                        if d2 > 64.0 { continue; }
                        let dir = d / d2.sqrt().max(1e-3);
                        let cos_l = -l.n.dot(dir);
                        if cos_l <= 0.0 || !clear(lv, p, l.p + l.n * 0.05) { continue; }
                        let g = l.rad * (cos_l / d2.max(0.35));
                        for (s, dn) in dirs.iter().enumerate() { let c = dn.dot(dir); if c > 0.0 { v[s] += g * c; } }
                    }
                }
            });
        }
    });
    // 3) voxels dentro de parede herdam dos vizinhos válidos (evita borda escura na interpolação)
    for _ in 0..3 {
        let (pv, po) = (val.clone(), ok.clone());
        for idx in 0..nvox {
            if po[idx] { continue; }
            let (x, y, z) = ((idx % RX) as i32, ((idx / RX) % RY) as i32, (idx / (RX * RY)) as i32);
            let (mut acc, mut n) = ([Vec3::ZERO; 6], 0.0);
            for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, 1, 0), (0, -1, 0)] {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                if nx < 0 || ny < 0 || nz < 0 || nx >= RX as i32 || ny >= RY as i32 || nz >= RZ as i32 { continue; }
                let j = nx as usize + ny as usize * RX + nz as usize * RX * RY;
                if po[j] { for s in 0..6 { acc[s] += pv[j][s]; } n += 1.0; }
            }
            if n > 0.0 { for s in 0..6 { val[idx][s] = acc[s] / n; } ok[idx] = true; }
        }
    }
    // 4) textura 3D no formato do Bevy: (Rx, 2·Ry, 3·Rz); metade de cima de cada fatia = normal negativa
    let peak = val.iter().flat_map(|v| v.iter()).map(|c| c.max_element()).fold(1e-6f32, f32::max);
    let (w, h, d) = (RX, RY * 2, RZ * 3);
    let mut data = vec![0u8; w * h * d * 4];
    for idx in 0..nvox {
        let (x, y, z) = (idx % RX, (idx / RX) % RY, idx / (RX * RY));
        for s in 0..6 {
            let (axis, neg) = (s / 2, s % 2 == 1);
            let (t, p) = (y + if neg { RY } else { 0 }, z + axis * RZ);
            let o = (x + t * w + p * w * h) * 4;
            let c = val[idx][s] / peak;
            data[o] = (c.x.clamp(0.0, 1.0) * 255.0) as u8; data[o + 1] = (c.y.clamp(0.0, 1.0) * 255.0) as u8; data[o + 2] = (c.z.clamp(0.0, 1.0) * 255.0) as u8; data[o + 3] = 255;
        }
    }
    let mut img = Image::new(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: d as u32 }, TextureDimension::D3, data, TextureFormat::Rgba8Unorm, RenderAssetUsages::default());
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::ClampToEdge, address_mode_v: ImageAddressMode::ClampToEdge, address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear, min_filter: ImageFilterMode::Linear, ..default() });
    // o valor guardado é irradiância/π; o fator converte para a unidade de luz ambiente da cena
    let intensity = peak / std::f32::consts::PI * env_f("CQB_GI", 3.0);
    commands.spawn((LightProbe, IrradianceVolume { voxels: images.add(img), intensity, ..default() },
        Transform::from_xyz((MAP_X1 + MAP_X2) / 2.0, RY as f32 * VOX / 2.0, (MAP_Z1 + MAP_Z2) / 2.0).with_scale(Vec3::new(RX as f32 * VOX, RY as f32 * VOX, RZ as f32 * VOX))));
    info!("luz rebatida: {} pontos virtuais, pico {:.0}, {:.2} s", vpls.len(), peak, t0.elapsed().as_secs_f32());
}
