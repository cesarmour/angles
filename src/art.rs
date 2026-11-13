//! Arte gerada em vetor (alvos, letreiros, diagramas) e utilitários de textura com mipmaps.
use crate::map::TKind;
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use resvg::{tiny_skia, usvg};

pub const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/Rajdhani-Bold.ttf");
pub const FONT_SEMI: &[u8] = include_bytes!("../assets/fonts/Rajdhani-SemiBold.ttf");
pub const FONT_BODY: &[u8] = include_bytes!("../assets/fonts/Rajdhani-Medium.ttf");
pub const FONT_REG: &[u8] = include_bytes!("../assets/fonts/Rajdhani-Regular.ttf");
/// Fonte do arco da porta (Barlow Condensed, SIL OFL): leve para o número grande, média e semibold para o resto.
pub const FONT_HUD_LIGHT: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-Light.ttf");
pub const FONT_HUD_REG: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-Regular.ttf");
pub const FONT_HUD_MED: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-Medium.ttf");
pub const FONT_HUD_SEMI: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-SemiBold.ttf");
/// Estêncil para as placas pintadas nas paredes.
pub const FONT_STENCIL: &[u8] = include_bytes!("../assets/fonts/SairaStencilOne-Regular.ttf");


pub fn sampler(repeat: bool) -> ImageSampler {
    let m = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: m, address_mode_v: m, address_mode_w: m,
        mag_filter: ImageFilterMode::Linear, min_filter: ImageFilterMode::Linear, mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16, ..default()
    })
}

/// Acrescenta a cadeia de mipmaps a uma imagem RGBA8 (o Bevy não gera sozinho).
pub fn add_mips(img: &mut Image) {
    let fmt = img.texture_descriptor.format;
    if !matches!(fmt, TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb) || img.texture_descriptor.mip_level_count > 1 { return; }
    let (mut w, mut h) = (img.texture_descriptor.size.width as usize, img.texture_descriptor.size.height as usize);
    let Some(data) = img.data.as_mut() else { return };
    if data.len() != w * h * 4 || w < 2 || h < 2 { return; }
    let mut prev: Vec<u8> = data.clone();
    let mut levels = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let (x0, y0) = ((x * 2).min(w - 1), (y * 2).min(h - 1));
                let (x1, y1) = ((x * 2 + 1).min(w - 1), (y * 2 + 1).min(h - 1));
                for c in 0..4 {
                    let s = prev[(y0 * w + x0) * 4 + c] as u32 + prev[(y0 * w + x1) * 4 + c] as u32 + prev[(y1 * w + x0) * 4 + c] as u32 + prev[(y1 * w + x1) * 4 + c] as u32;
                    next[(y * nw + x) * 4 + c] = ((s + 2) / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        prev = next; w = nw; h = nh; levels += 1;
    }
    img.texture_descriptor.mip_level_count = levels;
}

pub fn rgba_image(w: u32, h: u32, data: Vec<u8>, srgb: bool, repeat: bool, mips: bool) -> Image {
    let fmt = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
    let mut img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, data, fmt, RenderAssetUsages::default());
    if mips { add_mips(&mut img); }
    img.sampler = sampler(repeat);
    img
}

/// Rasteriza um SVG; devolve RGBA sem pré-multiplicação.
pub fn svg_rgba(svg: &str, w: u32, h: u32) -> Vec<u8> {
    let mut opt = usvg::Options::default();
    for f in [FONT_BOLD, FONT_SEMI, FONT_BODY, FONT_REG, FONT_STENCIL] { opt.fontdb_mut().load_font_data(f.to_vec()); }
    let mut pix = tiny_skia::Pixmap::new(w, h).expect("pixmap");
    match usvg::Tree::from_str(svg, &opt) {
        Ok(tree) => {
            let s = tree.size();
            resvg::render(&tree, tiny_skia::Transform::from_scale(w as f32 / s.width(), h as f32 / s.height()), &mut pix.as_mut());
        }
        Err(e) => warn!("SVG inválido: {e}"),
    }
    let mut out = pix.data().to_vec();
    for p in out.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 { for c in 0..3 { p[c] = ((p[c] as u32 * 255 + a / 2) / a).min(255) as u8; } }
    }
    out
}
pub fn svg_image(svg: &str, w: u32, h: u32, mips: bool) -> Image { rgba_image(w, h, svg_rgba(svg, w, h), true, false, mips) }

/* ---------- alvos de papelão tipo IPSC: pardo = ameaça, branco com X = não atire ---------- */
pub const TGT_W_MM: f32 = 460.0;
pub const TGT_H_MM: f32 = 760.0;
/// Contorno do alvo em milímetros: cabeça quadrada sobre corpo oitavado.
const OUTLINE: [(f32, f32); 12] = [(155.0, 0.0), (305.0, 0.0), (305.0, 150.0), (400.0, 150.0), (460.0, 210.0), (460.0, 580.0), (340.0, 760.0), (120.0, 760.0), (0.0, 580.0), (0.0, 210.0), (60.0, 150.0), (155.0, 150.0)];
const C_ZONE: [(f32, f32); 8] = [(110.0, 175.0), (350.0, 175.0), (395.0, 220.0), (395.0, 560.0), (310.0, 690.0), (150.0, 690.0), (65.0, 560.0), (65.0, 220.0)];

fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi, xj, yj) = (poly[i].0, poly[i].1, poly[j].0, poly[j].1);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi { c = !c; }
        j = i;
    }
    c
}
/// 0 = fora do alvo, 1 = zona D, 2 = zona C, 3 = zona A. `u`,`v` em 0..1 a partir do canto superior esquerdo.
pub fn target_zone(u: f32, v: f32) -> u8 {
    let (x, y) = (u * TGT_W_MM, v * TGT_H_MM);
    if !inside(&OUTLINE, x, y) { return 0; }
    if ((155.0..=305.0).contains(&x) && (215.0..=495.0).contains(&y)) || ((180.0..=280.0).contains(&x) && (40.0..=95.0).contains(&y)) { return 3; }
    if inside(&C_ZONE, x, y) || y < 150.0 { return 2; }
    1
}
fn pts(poly: &[(f32, f32)]) -> String { poly.iter().map(|p| format!("{},{}", p.0, p.1)).collect::<Vec<_>>().join(" ") }
pub fn ipsc_svg(no_shoot: bool) -> String {
    let (fill, line, ink) = if no_shoot { ("#edeae2", "#8f8b80", "#8f8b80") } else { ("#c39c6c", "#6b4f2c", "#6b4f2c") };
    let o = pts(&OUTLINE);
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 460 760\" width=\"460\" height=\"760\"><defs><clipPath id=\"c\"><polygon points=\"{o}\"/></clipPath><linearGradient id=\"g\" x1=\"0\" x2=\"1\" y1=\"0\" y2=\"1\"><stop offset=\"0\" stop-color=\"#fff\" stop-opacity=\"0.10\"/><stop offset=\"1\" stop-color=\"#000\" stop-opacity=\"0.10\"/></linearGradient></defs><polygon points=\"{o}\" fill=\"{fill}\"/><g clip-path=\"url(#c)\">");
    // ondulado do papelão
    for k in 0..77 { s += &format!("<path d=\"M{0} 0 V760\" stroke=\"#000\" stroke-opacity=\"{1}\" stroke-width=\"1\"/>", k as f32 * 6.0 + 2.0, if k % 3 == 0 { 0.05 } else { 0.025 }); }
    s += "<rect width=\"460\" height=\"760\" fill=\"url(#g)\"/>";
    // linhas de pontuação picotadas e letras das zonas
    s += &format!("<g fill=\"none\" stroke=\"{line}\" stroke-opacity=\"0.55\" stroke-width=\"2.2\" stroke-dasharray=\"7 4\"><rect x=\"155\" y=\"215\" width=\"150\" height=\"280\"/><rect x=\"180\" y=\"40\" width=\"100\" height=\"55\"/><polygon points=\"{}\"/><path d=\"M155 150 H305\"/></g>", pts(&C_ZONE));
    s += &format!("<g font-family=\"Rajdhani\" font-weight=\"700\" fill=\"{ink}\" fill-opacity=\"0.5\" text-anchor=\"middle\"><text x=\"230\" y=\"372\" font-size=\"52\">A</text><text x=\"230\" y=\"82\" font-size=\"30\">A</text><text x=\"352\" y=\"380\" font-size=\"38\">C</text><text x=\"108\" y=\"380\" font-size=\"38\">C</text><text x=\"428\" y=\"400\" font-size=\"30\">D</text><text x=\"32\" y=\"400\" font-size=\"30\">D</text></g>");
    if no_shoot { s += "<g stroke=\"#141414\" stroke-opacity=\"0.88\" stroke-width=\"26\" stroke-linecap=\"round\"><path d=\"M70 230 L390 660\"/><path d=\"M390 230 L70 660\"/></g>"; }
    s += &format!("</g><polygon points=\"{o}\" fill=\"none\" stroke=\"{line}\" stroke-opacity=\"0.8\" stroke-width=\"5\"/></svg>");
    s
}

/// Furo de projétil em papel: miolo escuro com borda rasgada clara.
pub fn paper_hole_svg(seed: u32) -> String {
    let mut pts = String::new();
    for k in 0..14 {
        let a = k as f32 / 14.0 * std::f32::consts::TAU;
        let r = 20.0 + ((seed.wrapping_mul(2654435761).wrapping_add(k * 40503)) % 13) as f32;
        pts += &format!("{:.1},{:.1} ", 32.0 + a.cos() * r, 32.0 + a.sin() * r);
    }
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\" width=\"64\" height=\"64\"><polygon points=\"{pts}\" fill=\"#e3c9a2\" stroke=\"#7a5c38\" stroke-width=\"1.5\"/><circle cx=\"32\" cy=\"32\" r=\"11\" fill=\"#060606\"/><circle cx=\"32\" cy=\"32\" r=\"14\" fill=\"none\" stroke=\"#000\" stroke-opacity=\"0.35\" stroke-width=\"3\"/></svg>")
}

/// Cratera de impacto em alvenaria: miolo escuro, lascas claras e trincas.
pub fn crater_svg() -> String {
    // Impacto em bloco de concreto: furo pequeno e escuro, lasca irregular em volta (concreto exposto, mais claro que a
    // pintura mas sem brilho), facetas, trincas finas e um pouco de pó. Nada de halo branco.
    let rnd = |k: u32, m: u32| ((k.wrapping_mul(2654435761).rotate_left(13) ^ (k * 97 + 31)) % m) as f32 / m as f32;
    let ring = |n: u32, r0: f32, dr: f32, seed: u32| -> String {
        (0..n).map(|k| { let a = k as f32 / n as f32 * std::f32::consts::TAU + (rnd(k + seed, 97) - 0.5) * 0.3; let r = r0 + rnd(k * 7 + seed, 89) * dr; format!("{:.1},{:.1}", 64.0 + a.cos() * r, 64.0 + a.sin() * r) }).collect::<Vec<_>>().join(" ")
    };
    let mut s = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 128 128\" width=\"128\" height=\"128\"><defs><radialGradient id=\"h\"><stop offset=\"0\" stop-color=\"#000\"/><stop offset=\"0.75\" stop-color=\"#0a0908\"/><stop offset=\"1\" stop-color=\"#231f1a\"/></radialGradient><filter id=\"d\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"5\"/></filter><filter id=\"e\"><feGaussianBlur stdDeviation=\"0.7\"/></filter></defs>");
    s += "<circle cx=\"64\" cy=\"66\" r=\"34\" fill=\"#3a3731\" opacity=\"0.22\" filter=\"url(#d)\"/>";
    s += &format!("<polygon points=\"{}\" fill=\"#77726a\" stroke=\"#26231f\" stroke-width=\"1.6\" stroke-linejoin=\"round\" opacity=\"0.96\" filter=\"url(#e)\"/>", ring(15, 19.0, 9.0, 3));
    s += &format!("<polygon points=\"{}\" fill=\"#59554e\"/>", ring(13, 12.5, 5.0, 41));
    // facetas: lascas que pegam luz de um lado e ficam na sombra do outro
    for k in 0..6u32 {
        let a = k as f32 * 1.05 + rnd(k + 5, 53);
        let (r0, r1) = (9.0 + rnd(k + 9, 31) * 3.0, 18.0 + rnd(k + 2, 37) * 6.0);
        let lit = (a.sin() + a.cos()) < 0.0;
        s += &format!("<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{}\" opacity=\"{}\"/>", 64.0 + a.cos() * r0, 64.0 + a.sin() * r0, 64.0 + (a - 0.22).cos() * r1, 64.0 + (a - 0.22).sin() * r1, 64.0 + (a + 0.24).cos() * r1, 64.0 + (a + 0.24).sin() * r1, if lit { "#9a948a" } else { "#3d3a34" }, if lit { 0.55 } else { 0.6 });
    }
    s += &format!("<polygon points=\"{}\" fill=\"url(#h)\"/>", ring(11, 6.2, 2.4, 77));
    // trincas finas, em dois segmentos, saindo da borda da lasca
    for k in 0..6u32 {
        let a = k as f32 * 1.07 + rnd(k + 13, 61) * 0.6;
        let (r0, r1, r2) = (23.0 + rnd(k, 17) * 4.0, 34.0 + rnd(k + 3, 23) * 9.0, 44.0 + rnd(k + 8, 29) * 14.0);
        let b = a + (rnd(k + 21, 19) - 0.5) * 0.5;
        s += &format!("<path d=\"M{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1}\" fill=\"none\" stroke=\"#1b1916\" stroke-opacity=\"0.75\" stroke-width=\"{:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", 64.0 + a.cos() * r0, 64.0 + a.sin() * r0, 64.0 + (a + 0.09).cos() * r1, 64.0 + (a + 0.09).sin() * r1, 64.0 + b.cos() * r2, 64.0 + b.sin() * r2, 0.9 + rnd(k + 4, 7) * 0.7);
    }
    // estilhaços soltos
    for k in 0..9u32 {
        let (a, r) = (rnd(k + 60, 101) * std::f32::consts::TAU, 30.0 + rnd(k + 70, 43) * 20.0);
        s += &format!("<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#2a2723\" opacity=\"0.6\"/>", 64.0 + a.cos() * r, 64.0 + a.sin() * r, 0.6 + rnd(k + 80, 11) * 0.9);
    }
    s + "</svg>"
}

pub fn sign_svg(lines: &[(&str, f32)], w: f32, h: f32, color: &str) -> String {
    let n = lines.len() as f32;
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">");
    for (i, (t, size)) in lines.iter().enumerate() {
        let y = h * (i as f32 + 0.5) / n + size * 0.34;
        s += &format!("<text x=\"{}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"Saira Stencil One\" font-weight=\"400\" font-size=\"{size}\" fill=\"{color}\">{t}</text>", w / 2.0);
    }
    s + "</svg>"
}

pub fn radial_svg(stops: &[(f32, &str, f32)]) -> String {
    let st: String = stops.iter().map(|(o, c, a)| format!("<stop offset=\"{o}\" stop-color=\"{c}\" stop-opacity=\"{a}\"/>")).collect();
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\" width=\"64\" height=\"64\"><defs><radialGradient id=\"g\">{st}</radialGradient></defs><rect width=\"64\" height=\"64\" fill=\"url(#g)\"/></svg>")
}

pub fn flash_svg() -> String {
    let mut s = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-64 -64 128 128\" width=\"128\" height=\"128\"><defs><radialGradient id=\"g\"><stop offset=\"0\" stop-color=\"#fffff0\" stop-opacity=\"1\"/><stop offset=\"1\" stop-color=\"#ffc85a\" stop-opacity=\"0\"/></radialGradient><linearGradient id=\"l\" x1=\"0\" x2=\"1\"><stop offset=\"0\" stop-color=\"#fff0c8\" stop-opacity=\"0.95\"/><stop offset=\"1\" stop-color=\"#ff9628\" stop-opacity=\"0\"/></linearGradient></defs>");
    for k in 0..9 {
        let a = k as f32 * 40.0 + (k * 37 % 23) as f32;
        let len = 30.0 + (k * 13 % 30) as f32;
        let w = 6.0 + (k * 7 % 8) as f32;
        s += &format!("<polygon transform=\"rotate({a})\" points=\"0,{w} {len},0 0,-{w}\" fill=\"url(#l)\"/>");
    }
    s + "<circle r=\"26\" fill=\"url(#g)\"/></svg>"
}

/* ---------- diagramas dos cartões de lição ---------- */
const DIAG_CSS: &str = "<style>text{font-family:'Rajdhani';font-weight:700;font-size:15px;fill:#e9e6dc}.lbl{font-family:'Rajdhani';font-weight:500;font-size:10.5px;fill:#a4a89d}.lbl.ok{fill:#6fd58e}.lbl.bad{fill:#ff5a4f}.wall{fill:none;stroke:#e9e6dc;stroke-width:6;stroke-linecap:square}.thin{stroke-width:2;stroke-dasharray:4 5;opacity:.5}.f1{fill:#60d6e8;fill-opacity:.42}.f2{fill:#60d6e8;fill-opacity:.26}.f3{fill:#60d6e8;fill-opacity:.13}.danger{fill:#ff5a4f;fill-opacity:.28}.op{fill:#f2b23a;stroke:#14100a;stroke-width:2}.ghost{fill:none;stroke:#f2b23a}.sight{fill:none;stroke:#ffd23a;stroke-width:1.4;stroke-dasharray:5 4}.arc{fill:none;stroke:#f2b23a;stroke-width:2;stroke-dasharray:2 5;stroke-linecap:round}.move{fill:none;stroke:#f2b23a;stroke-width:2;stroke-linecap:round}.thr{fill:#ff5a4f}.corner{fill:#ff5a4f;fill-opacity:.25;stroke:#ff5a4f;stroke-width:2}.furn{fill:#5a6258}.mass{fill:#e9e6dc;fill-opacity:.09}.skin{fill:#b08a6a}.cloth{fill:#3c443a}.gunb{fill:#050505;stroke:#777;stroke-width:1}.phone{fill:#5b8fd0;stroke:#dfe5ea;stroke-width:3}.key{fill:#f2b23a;font-size:14px}.kv{font-family:'Rajdhani';font-weight:500;font-size:12px;fill:#e9e6dc}</style>";

pub fn diagram_svg(i: usize) -> String {
    let body = match i {
        0 => {
            let keys = [("W A S D", "move"), ("MOUSE", "look"), ("LEFT BTN", "fire"), ("SPACE · Z", "aim / lock aim"), ("Q · E", "lean"), ("C / CTRL", "crouch"),
                ("SHIFT", "fast step"), ("R", "reload"), ("F", "open door"), ("L", "flashlight"), ("G", "tactical view"), ("V", "top-down view"), ("H", "review lesson"), ("ESC", "pause")];
            let mut s = String::new();
            for (k, (key, val)) in keys.iter().enumerate() {
                let (col, row) = ((k % 2) as f32, (k / 2) as f32);
                let (x, y) = (10.0 + col * 156.0, 20.0 + row * 25.0);
                s += &format!("<text class=\"key\" x=\"{x}\" y=\"{y}\">{key}</text><text class=\"kv\" x=\"{}\" y=\"{y}\">{val}</text><path d=\"M{x} {} H{}\" stroke=\"#e9e6dc\" stroke-opacity=\"0.16\"/>", x + 72.0, y + 7.0, x + 146.0);
            }
            s
        }
        1 => "<path class=\"f1\" d=\"M150 96 L150 6 L194 6 Z\"/><path class=\"f2\" d=\"M150 96 L194 6 L265 6 Z\"/><path class=\"f3\" d=\"M150 96 L265 6 L310 6 L310 57 Z\"/><path class=\"danger\" d=\"M150 96 L310 57 L310 96 Z\"/><rect class=\"mass\" x=\"150\" y=\"96\" width=\"160\" height=\"90\"/><path class=\"wall\" d=\"M10 186 V4 H310\"/><path class=\"wall\" d=\"M150 186 V96 H310\"/><path class=\"arc\" d=\"M112 176 Q 82 174 70 160 Q 44 148 36 124\"/><path class=\"sight\" d=\"M112 176 L150 96 L194 6\"/><path class=\"sight\" d=\"M70 160 L150 96 L265 6\"/><path class=\"sight\" d=\"M36 124 L150 96 L310 57\"/><circle class=\"op\" cx=\"112\" cy=\"176\" r=\"7\"/><circle class=\"op\" cx=\"70\" cy=\"160\" r=\"7\"/><circle class=\"op\" cx=\"36\" cy=\"124\" r=\"7\"/><text x=\"122\" y=\"170\">1</text><text x=\"80\" y=\"152\">2</text><text x=\"46\" y=\"114\">3</text><rect class=\"thr\" x=\"270\" y=\"26\" width=\"12\" height=\"12\" transform=\"rotate(45 276 32)\"/><text class=\"lbl\" x=\"160\" y=\"128\">wall</text><text class=\"lbl\" x=\"160\" y=\"174\">each side step opens a slice</text><text class=\"lbl\" x=\"246\" y=\"88\">still hidden</text>".to_string(),
        2 => "<path class=\"danger\" d=\"M134 70 L186 70 L250 186 L70 186 Z\"/><path class=\"wall\" d=\"M10 70 H134 M186 70 H310 M10 70 V10 H310 V70\"/><path class=\"wall thin\" d=\"M10 70 V186 M310 70 V186\"/><circle class=\"corner\" cx=\"26\" cy=\"54\" r=\"11\"/><circle class=\"corner\" cx=\"294\" cy=\"54\" r=\"11\"/><path class=\"move\" d=\"M160 150 L160 78 Q160 44 108 40\"/><path class=\"move\" d=\"M160 78 Q160 44 212 40\"/><circle class=\"op\" cx=\"160\" cy=\"156\" r=\"7\"/><text class=\"lbl\" x=\"196\" y=\"140\">fatal funnel</text><text class=\"lbl\" x=\"44\" y=\"30\">near corner</text><text class=\"lbl\" x=\"200\" y=\"30\">near corner</text>".to_string(),
        3 => {
            let o = pts(&OUTLINE);
            format!("<g transform=\"translate(52 10) scale(0.2)\"><polygon points=\"{o}\" fill=\"#c39c6c\" stroke=\"#6b4f2c\" stroke-width=\"10\"/><rect x=\"155\" y=\"215\" width=\"150\" height=\"280\" fill=\"none\" stroke=\"#6b4f2c\" stroke-width=\"8\" stroke-dasharray=\"24 14\"/></g><text class=\"lbl bad\" x=\"98\" y=\"182\" text-anchor=\"middle\">tan: threat</text><g transform=\"translate(176 10) scale(0.2)\"><polygon points=\"{o}\" fill=\"#edeae2\" stroke=\"#8f8b80\" stroke-width=\"10\"/><path d=\"M70 230 L390 660 M390 230 L70 660\" stroke=\"#141414\" stroke-width=\"30\" stroke-linecap=\"round\"/></g><text class=\"lbl ok\" x=\"222\" y=\"182\" text-anchor=\"middle\">white with X: no-shoot</text>")
        }
        4 => "<path class=\"f2\" d=\"M118 96 L118 60 L10 60 L10 96 Z\"/><path class=\"f1\" d=\"M202 96 L202 60 L310 60 L310 96 Z\"/><path class=\"wall\" d=\"M10 60 H310 M10 96 H134 M186 96 H310\"/><path class=\"wall thin\" d=\"M10 96 V186 M310 96 V186\"/><path class=\"danger\" d=\"M140 100 H180 V186 H140 Z\"/><circle class=\"op\" cx=\"118\" cy=\"120\" r=\"7\"/><path class=\"sight\" d=\"M118 120 L186 96 L310 62\"/><circle class=\"op ghost\" cx=\"202\" cy=\"120\" r=\"7\"/><path class=\"sight\" d=\"M202 120 L134 96 L10 62\"/><text class=\"lbl\" x=\"196\" y=\"170\">don't stand in front of the door</text><text class=\"lbl\" x=\"14\" y=\"50\">side 2</text><text class=\"lbl\" x=\"268\" y=\"50\">side 1</text>".to_string(),
        _ => "<path class=\"danger\" d=\"M120 14 L120 106 L14 36 L14 14 Z\"/><path class=\"wall\" d=\"M10 10 H310 V180 H250 M210 180 H10 V10\"/><rect class=\"furn\" x=\"120\" y=\"12\" width=\"18\" height=\"96\"/><circle class=\"op\" cx=\"230\" cy=\"172\" r=\"7\"/><path class=\"sight\" d=\"M230 172 L129 108 L14 34\"/><path class=\"arc\" d=\"M222 160 Q170 150 80 150\"/><circle class=\"op ghost\" cx=\"80\" cy=\"150\" r=\"7\"/><path class=\"sight\" d=\"M80 150 L60 40\"/><rect class=\"thr\" x=\"52\" y=\"34\" width=\"12\" height=\"12\" transform=\"rotate(45 58 40)\"/><text class=\"lbl\" x=\"150\" y=\"60\">area the door doesn't show</text>".to_string(),
    };
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 320 190\" width=\"320\" height=\"190\">{DIAG_CSS}{body}</svg>")
}

/// Grava a arte dos alvos em RGBA cru para conferência visual (CQB_DUMP=pasta).
/* ---------- operador visto de cima (512 px = 1,1 m; centro do corpo em 256,330; a frente é para cima) ---------- */
pub const OP_PX: u32 = 512;
pub const OP_M: f32 = 1.1;
/// Quanto o centro do corpo fica atrás do centro da imagem, em metros.
pub const OP_BACK: f32 = (330.0 - 256.0) / 512.0 * 1.1;
pub fn operator_svg() -> String {
    // uniforme preto tático (GATE, o grupo de ações táticas especiais da PM de São Paulo): tons de grafite para o desenho continuar legível sob a luz da cena
    let arm = |sx: f32| {
        let d = format!("M {} -4 L {} -90 L {} -168", sx * 100.0, sx * 97.0, sx * 17.0);
        format!(r##"<path d="{d}" fill="none" stroke="#050506" stroke-width="50" stroke-linecap="round" stroke-linejoin="round"/>
<path d="{d}" fill="none" stroke="#2b2e33" stroke-width="43" stroke-linecap="round" stroke-linejoin="round"/>
<path d="{d}" fill="none" stroke="#454a52" stroke-width="19" stroke-linecap="round" stroke-linejoin="round" opacity="0.6" transform="translate({hx} -3)"/>
<path d="M {a} -128 l {b} 8 M {c} -52 l {b} 4 M {e} -30 l {b} 3" fill="none" stroke="#141518" stroke-width="4" stroke-linecap="round" opacity="0.8"/>
<rect x="{px}" y="-50" width="26" height="30" rx="4" fill="#3d4149" stroke="#0a0a0b" stroke-width="2"/>
<rect x="{qx}" y="-44" width="16" height="9" rx="1.5" fill="{flag}"/>
<ellipse cx="{ex}" cy="-90" rx="20" ry="24" fill="#17181b" stroke="#050506" stroke-width="3"/>
<ellipse cx="{ex}" cy="-92" rx="11" ry="14" fill="#30333a"/>
<path d="M {wx} -150 l {wl} -15" stroke="#0a0a0b" stroke-width="9" stroke-linecap="round"/>"##,
            hx = -sx * 4.0, a = sx * 56.0 - 13.0, b = 26.0, c = sx * 98.0 - 13.0, e = sx * 99.0 - 13.0, ex = sx * 97.0,
            px = sx * 101.0 - 13.0, qx = sx * 101.0 - 8.0, flag = if sx < 0.0 { "#8d939c" } else { "#6b7078" }, wx = sx * 34.0, wl = -sx * 14.0)
    };
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
<defs>
<filter id="soft" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="13"/></filter>
<filter id="hl" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="6"/></filter>
<radialGradient id="helm" cx="0.4" cy="0.32" r="0.8"><stop offset="0" stop-color="#5b616b"/><stop offset="0.45" stop-color="#30343a"/><stop offset="1" stop-color="#0d0e10"/></radialGradient>
<radialGradient id="tor" cx="0.5" cy="0.45" r="0.6"><stop offset="0" stop-color="#383c43"/><stop offset="0.7" stop-color="#26292e"/><stop offset="1" stop-color="#121316"/></radialGradient>
<linearGradient id="pack" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#34383f"/><stop offset="1" stop-color="#17191c"/></linearGradient>
<linearGradient id="glove" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3a3d44"/><stop offset="1" stop-color="#1b1d20"/></linearGradient>
<linearGradient id="strap" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#1c1e22"/><stop offset="0.5" stop-color="#363a41"/><stop offset="1" stop-color="#1c1e22"/></linearGradient>
<clipPath id="ch"><ellipse cx="0" cy="-8" rx="58" ry="66"/></clipPath>
</defs>
<g transform="translate(256 330)">
<ellipse cx="0" cy="8" rx="140" ry="94" fill="#000" opacity="0.4" filter="url(#soft)"/>
<rect x="-84" y="10" width="168" height="96" rx="20" fill="url(#pack)" stroke="#050506" stroke-width="3"/>
<rect x="-60" y="62" width="120" height="36" rx="3" fill="#0e0f11" stroke="#4a4f57" stroke-width="1.5"/>
<text x="3" y="91" text-anchor="middle" font-family="Rajdhani" font-weight="700" font-size="34" letter-spacing="5" fill="#e6e7e2">GATE</text>
<ellipse cx="0" cy="0" rx="124" ry="58" fill="url(#tor)" stroke="#050506" stroke-width="3"/>
<path d="M -118 6 Q -70 30 0 32 Q 70 30 118 6" fill="none" stroke="#0d0e10" stroke-width="3" opacity="0.8"/>
<rect x="-77" y="-46" width="32" height="86" rx="9" fill="url(#strap)" stroke="#050506" stroke-width="3"/>
<rect x="45" y="-46" width="32" height="86" rx="9" fill="url(#strap)" stroke="#050506" stroke-width="3"/>
<path d="M -71 -24 h 20 M -71 -8 h 20 M -71 8 h 20 M 51 -24 h 20 M 51 -8 h 20 M 51 8 h 20" stroke="#0b0c0d" stroke-width="3.5"/>
<rect x="56" y="-40" width="13" height="24" rx="3" fill="#101113" stroke="#4a4f57" stroke-width="1.5"/>
{arm_l}
{arm_r}
<g transform="translate(0 50)">
<rect x="-9.5" y="-314" width="19" height="104" rx="3.5" fill="#0e0f11" stroke="#000" stroke-width="2.5"/>
<rect x="-6" y="-308" width="12" height="62" rx="2" fill="#2a2c31"/>
<rect x="-3.5" y="-306" width="2.5" height="58" fill="#50545c" opacity="0.8"/>
<rect x="0" y="-278" width="7" height="20" fill="#3d4047"/>
<rect x="-2" y="-311" width="4" height="6" fill="#e9e4c4"/>
<ellipse cx="-15" cy="-222" rx="21" ry="27" fill="url(#glove)" stroke="#050506" stroke-width="3"/>
<ellipse cx="15" cy="-222" rx="21" ry="27" fill="url(#glove)" stroke="#050506" stroke-width="3"/>
<rect x="-9.5" y="-252" width="19" height="46" rx="3.5" fill="#0e0f11"/>
<rect x="-6" y="-252" width="12" height="20" fill="#2a2c31"/>
<path d="M -8 -236 h 16 M -8 -231 h 16 M -8 -226 h 16 M -8 -221 h 16" stroke="#000" stroke-width="2"/>
<rect x="-7" y="-216" width="14" height="5" fill="#000"/><circle cx="-4" cy="-213.5" r="1.6" fill="#cfe0a0"/><circle cx="4" cy="-213.5" r="1.6" fill="#cfe0a0"/>
<path d="M -30 -236 q 6 -7 13 -3 M -31 -224 q 7 -6 14 -2 M 30 -236 q -6 -7 -13 -3 M 31 -224 q -7 -6 -14 -2" fill="none" stroke="#5b606a" stroke-width="2.5" stroke-linecap="round"/>
<path d="M -33 -212 q 4 6 12 5 M 33 -212 q -4 6 -12 5" fill="none" stroke="#5b606a" stroke-width="2" stroke-linecap="round"/>
<rect x="-35" y="-203" width="23" height="17" rx="5" fill="#111214" stroke="#050506" stroke-width="2"/><rect x="12" y="-203" width="23" height="17" rx="5" fill="#111214" stroke="#050506" stroke-width="2"/>
</g>
<ellipse cx="-63" cy="0" rx="15" ry="26" fill="#1d1f23" stroke="#000" stroke-width="2.5"/>
<ellipse cx="63" cy="0" rx="15" ry="26" fill="#1d1f23" stroke="#000" stroke-width="2.5"/>
<ellipse cx="-66" cy="-2" rx="6" ry="14" fill="#3f434a"/><ellipse cx="66" cy="-2" rx="6" ry="14" fill="#3f434a"/>
<path d="M -58 -22 Q -74 -44 -40 -60" fill="none" stroke="#000" stroke-width="4" stroke-linecap="round"/>
<path d="M 62 22 L 76 84" stroke="#000" stroke-width="3.5" stroke-linecap="round"/>
<ellipse cx="0" cy="-8" rx="58" ry="66" fill="url(#helm)" stroke="#000" stroke-width="3"/>
<g clip-path="url(#ch)">
<path d="M -50 -46 Q 0 -24 50 -46 M -56 14 Q 0 36 56 14" fill="none" stroke="#0a0a0b" stroke-width="2.5" opacity="0.8"/>
<path d="M -50 -43 Q 0 -21 50 -43 M -56 17 Q 0 39 56 17" fill="none" stroke="#6a707a" stroke-width="1.2" opacity="0.5"/>
<path d="M -22 -70 V -34 M 22 -70 V -34 M -22 -4 V 50 M 22 -4 V 50" stroke="#0a0a0b" stroke-width="2" opacity="0.55"/>
<circle cx="-30" cy="-14" r="3" fill="#0a0a0b"/><circle cx="30" cy="-14" r="3" fill="#0a0a0b"/><circle cx="0" cy="-48" r="3" fill="#0a0a0b"/>
</g>
<rect x="-62" y="-36" width="9" height="50" rx="3" fill="#08090a" stroke="#3a3e45" stroke-width="1"/><rect x="53" y="-36" width="9" height="50" rx="3" fill="#08090a" stroke="#3a3e45" stroke-width="1"/>
<rect x="-15" y="-80" width="30" height="18" rx="3.5" fill="#08090a" stroke="#3a3e45" stroke-width="1.5"/><rect x="-7" y="-76" width="14" height="9" rx="2" fill="#2a2d32"/>
<rect x="-20" y="22" width="40" height="22" rx="3" fill="#15161a" stroke="#4a4f57" stroke-width="1.5"/>
<rect x="-15" y="27" width="30" height="12" rx="1" fill="#d9dad4"/><text x="0" y="37" text-anchor="middle" font-family="Rajdhani" font-weight="700" font-size="11" fill="#111">POLICE</text>
<rect x="-46" y="14" width="16" height="12" rx="2" fill="#08090a" stroke="#3a3e45" stroke-width="1"/><circle cx="-38" cy="20" r="2.5" fill="#6b1512"/>
<ellipse cx="-18" cy="-36" rx="25" ry="18" fill="#fff" opacity="0.16" filter="url(#hl)"/>
</g></svg>"##, arm_l = arm(-1.0), arm_r = arm(1.0))
}
/// Bota vista de cima (96 x 192 px = 0,15 x 0,30 m), com a barra da calça.
pub fn boot_svg() -> String {
    r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="192" viewBox="0 0 96 192">
<rect x="10" y="96" width="76" height="90" rx="26" fill="#26292e" stroke="#050506" stroke-width="4"/>
<path d="M 16 34 Q 16 8 48 8 Q 80 8 80 34 L 78 138 Q 78 150 48 150 Q 18 150 18 138 Z" fill="#141518" stroke="#000" stroke-width="4"/>
<path d="M 24 36 Q 24 16 48 16 Q 72 16 72 36 L 72 58 Q 48 50 24 58 Z" fill="#2a2c31"/>
<path d="M 30 72 H 66 M 30 86 H 66 M 30 100 H 66 M 32 114 H 64" stroke="#5a5f68" stroke-width="4" stroke-linecap="round"/>
<path d="M 48 66 V 120" stroke="#000" stroke-width="5"/>
</svg>"##.to_string()
}

/// Ícone do aplicativo (1024 px): o operador visto de cima fatiando uma porta, com a sala ainda não limpa ao fundo.
pub fn icon_svg() -> String {
    // reaproveita o desenho do operador (definições e corpo) dentro do ícone
    let op = operator_svg();
    let cut = |a: &str, b: &str| -> String { let i = op.find(a).map(|i| i + a.len()).unwrap_or(0); let j = op.rfind(b).unwrap_or(op.len()); op[i..j].to_string() };
    let op_defs = { let i = op.find("<defs>").map(|i| i + 6).unwrap_or(0); let j = op.find("</defs>").unwrap_or(i); op[i..j].to_string() };
    let op_body = cut("<g transform=\"translate(256 330)\">", "</g></svg>");
    let mut ticks = String::new();
    for k in 0..=36 {
        let a = (k as f32 * 5.0).to_radians();
        let (c, sn) = (a.cos(), a.sin());
        let col = if (60..=120).contains(&(k * 5)) { "#7be3a0" } else { "#ff5f52" };
        ticks += &format!(r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{col}" stroke-width="10" stroke-linecap="round"/>"#, 512.0 + c * 214.0, 462.0 - sn * 214.0, 512.0 + c * 252.0, 462.0 - sn * 252.0);
    }
    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">
<defs>
{op_defs}
<linearGradient id="ibg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b3d4a"/><stop offset="0.55" stop-color="#16212a"/><stop offset="1" stop-color="#090d11"/></linearGradient>
<radialGradient id="ivg" cx="0.5" cy="0.62" r="0.7"><stop offset="0" stop-color="#3a5566" stop-opacity="0.55"/><stop offset="1" stop-color="#000" stop-opacity="0"/></radialGradient>
<linearGradient id="iwedge" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#7fe6f5" stop-opacity="0.78"/><stop offset="0.55" stop-color="#60d6e8" stop-opacity="0.42"/><stop offset="1" stop-color="#60d6e8" stop-opacity="0.2"/></linearGradient>
<linearGradient id="iwall" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fbf8ee"/><stop offset="0.5" stop-color="#d9d5c8"/><stop offset="1" stop-color="#9c988b"/></linearGradient>
<linearGradient id="igloss" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.16"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
<pattern id="ihatch" width="26" height="26" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="26" height="26" fill="#3d0f0d"/><rect width="9" height="26" fill="#6b1915"/></pattern>
<clipPath id="ir"><rect x="100" y="100" width="824" height="824" rx="186"/></clipPath>
<filter id="ish" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="16"/></filter>
<filter id="iglow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="14"/></filter>
<filter id="iglow2" x="-60%" y="-60%" width="220%" height="220%"><feGaussianBlur stdDeviation="7"/></filter>
</defs>
<rect x="100" y="116" width="824" height="824" rx="186" fill="#000" opacity="0.42" filter="url(#ish)"/>
<rect x="100" y="100" width="824" height="824" rx="186" fill="url(#ibg)"/>
<g clip-path="url(#ir)">
<rect x="100" y="100" width="824" height="366" fill="url(#ihatch)" opacity="0.62"/>
<rect x="100" y="100" width="824" height="824" fill="url(#ivg)"/>
<path d="M100 264 H924 M100 620 H924 M100 784 H924 M264 100 V924 M428 100 V924 M596 100 V924 M760 100 V924" stroke="#bfe8ff" stroke-opacity="0.05" stroke-width="3"/>
<ellipse cx="512" cy="760" rx="300" ry="230" fill="#6f93a8" opacity="0.5" filter="url(#ish)"/>
<path d="M512 748 L300 100 L724 100 Z" fill="#60d6e8" opacity="0.5" filter="url(#iglow)"/>
<path d="M512 748 L300 100 L724 100 Z" fill="url(#iwedge)"/>
<path d="M512 748 L300 100 M512 748 L724 100" stroke="#a6f1fb" stroke-width="7" stroke-linecap="round"/>
<g opacity="0.7" filter="url(#iglow2)">{ticks}</g>
{ticks}
<rect x="100" y="452" width="318" height="52" fill="#000" opacity="0.55" filter="url(#iglow2)"/>
<rect x="606" y="452" width="318" height="52" fill="#000" opacity="0.55" filter="url(#iglow2)"/>
<rect x="100" y="440" width="314" height="46" fill="url(#iwall)"/><rect x="610" y="440" width="314" height="46" fill="url(#iwall)"/>
<rect x="100" y="440" width="314" height="5" fill="#fff" opacity="0.7"/><rect x="610" y="440" width="314" height="5" fill="#fff" opacity="0.7"/>
<rect x="390" y="434" width="30" height="58" rx="5" fill="#f6b93d" stroke="#3a2a08" stroke-width="4"/><rect x="604" y="434" width="30" height="58" rx="5" fill="#f6b93d" stroke="#3a2a08" stroke-width="4"/>
<g transform="translate(760 252)">
<circle r="64" fill="#ff5a4f" opacity="0.45" filter="url(#iglow)"/>
<rect x="-34" y="-34" width="68" height="68" rx="8" fill="#ff5a4f" stroke="#3b0a07" stroke-width="6" transform="rotate(45)"/>
<circle r="11" fill="#3b0a07"/>
</g>
<circle cx="512" cy="748" r="158" fill="none" stroke="#60d6e8" stroke-width="18" opacity="0.5" filter="url(#iglow2)"/>
<circle cx="512" cy="748" r="158" fill="none" stroke="#8eeaf7" stroke-width="9"/>
<g transform="translate(512 766) scale(0.98)">{op_body}</g>
<rect x="100" y="100" width="824" height="330" fill="url(#igloss)"/>
</g>
<rect x="103" y="103" width="818" height="818" rx="183" fill="none" stroke="#ffffff" stroke-opacity="0.14" stroke-width="6"/>
</svg>"##)
}

pub fn dump(dir: &str) {
    let _ = std::fs::write(format!("{dir}/icone.rgba"), svg_rgba(&icon_svg(), 1024, 1024));
    let _ = std::fs::write(format!("{dir}/furo.rgba"), svg_rgba(&crater_svg(), 256, 256));
    let _ = std::fs::write(format!("{dir}/operador.rgba"), svg_rgba(&operator_svg(), OP_PX, OP_PX));
    let _ = std::fs::write(format!("{dir}/bota.rgba"), svg_rgba(&boot_svg(), 96, 192));
    for (n, ns) in [false, true].iter().enumerate() { let _ = std::fs::write(format!("{dir}/alvo{n}.rgba"), svg_rgba(&ipsc_svg(*ns), 460, 760)); }
}
