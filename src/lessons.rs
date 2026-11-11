//! Lições do tutorial: textos, objetivos e progressão.
use crate::bot::Bot;
use crate::map::*;
use crate::state::*;
use crate::tactical::Tac;
use bevy::prelude::*;
use std::collections::VecDeque;

pub struct Lesson { pub title: &'static str, pub kicker: &'static str, pub lead: &'static str, pub points: &'static [&'static str], pub objs: &'static [&'static str], pub corners: Option<&'static str> }

pub const LESSONS: [Lesson; 6] = [
    Lesson { title: "Fundamentals", kicker: "Lesson 1 of 6 · Staging area", corners: None,
        lead: "You are in a shoot house: a training building with cardboard targets. Nobody wins here by shooting fast. You win by seeing first, exposing little and firing only at what you have identified.",
        points: &["Targets are IPSC-style cardboard. They drop with two hits, or a single hit in the A zone (the centre rectangle and the head box).", "Q and E lean your torso. C toggles crouch; Ctrl crouches while held. To aim, hold Space or the right button; Z locks and unlocks the aim (handy on a trackpad).", "The tactical view (G) shows your sector: what eyes and gun cover right now (70° at the ready, 32° through the sights; aiming narrows it). The human eye notices movement across almost 200°, but only identifies a target in that central band.", "Every door or corner has a 180° fan: everything on the other side of the opening, wall to wall. The panel shows two numbers: degrees in view right now (changes as you step, lean, turn or aim) and degrees already cleared. The 180° belong to the door, not to your vision.", "The top-down view (V) shows the same scene from above, with only what you have already seen."],
        objs: &["Look around with the mouse", "Walk with W A S D", "Aim: hold Space or the right button (Z locks the aim)", "Lean with Q or E", "Crouch with C or Ctrl", "Drop the practice target on the back wall", "Reload with R", "Turn on the tactical view with G", "Go to the top-down view with V and back", "Switch the flashlight off or on with L"] },
    Lesson { title: "Slicing the pie", kicker: "Lesson 2 of 6 · Corridor corner", corners: None,
        lead: "Every corner hides an angle. Don't turn it in one go: move away from it and go around in an arc, one side step at a time. Each step opens a few more degrees of the corner's 180° fan, with the gun already pointed at the new slice.",
        points: &["Stay far from the corner: the farther from the wall, the less body you show for each degree you see.", "Eyes and muzzle together: whatever appears in the new slice appears in your sights.", "Lean toward the open side, away from the wall (the panel tells you which key, Q or E, and how many degrees it adds): you see the next slice exposing only head and gun. Leaning into the wall opens nothing.", "Score: follow the ideal route (T switches it: always, tip, off) and drop each threat at the smallest slice angle with the least body showing. The ODDS box shows it live. O hides the lesson box.", "Don't flag your muzzle: if you touch the wall, the gun pulls back."],
        objs: &["Advance to near the corner", "At the corner, lean: under 15 cm of body exposed", "Go around in an arc: see the threat before you are side-on to it", "Neutralize the corridor threat", "Corridor 2 clear"] },
    Lesson { title: "Fatal funnel and near corners", kicker: "Lesson 3 of 6 · Room A, centre door", corners: Some("A"),
        lead: "The door is the one place everyone knows you will come through. Whoever is inside is already aiming at it. That is the fatal funnel: clear what you can from outside, then go through without stopping.",
        points: &["From outside: slice the door from one jamb to the other. Watch the fan degrees go up; what is left are the near corners.", "On entry: don't stop in the doorway. Go in and get off the door line.", "Near corners first: they are the only part of the room you haven't seen yet.", "The barricade in the middle of the room covers anyone crouching behind it, on either side."],
        objs: &["From outside, slice the door from both sides (110° of 180°)", "Cross the doorway without stopping (1.5 s max)", "Check both near corners", "Neutralize the 2 threats", "Room A clear"] },
    Lesson { title: "Positive identification", kicker: "Lesson 4 of 6 · Room B, corner door", corners: Some("B"),
        lead: "Not every target is a shoot target. A tan target is a threat. A white target with an X is a no-shoot: hostage, resident, teammate. Before every shot, confirm what it is and what is behind it.",
        points: &["White with an X: don't shoot. Hitting one is the most serious error on the course.", "Tan: threat. Two shots in the A zone.", "Corner door: the whole room lies to one side. The near corner is the one that runs along the door wall.", "The flashlight (L) helps tell tan from white in dark corners."],
        objs: &["Slice the door before entering (100° of 180°)", "Neutralize the 2 threats (tan targets)", "Don't shoot a white target", "Room B clear"] },
    Lesson { title: "Closed door and T-junction", kicker: "Lesson 5 of 6 · Corridor 3", corners: None,
        lead: "A closed door tells you nothing about what is behind it. Don't stand in front of it: open it from the side, off the line of fire of whoever is on the other side.",
        points: &["Stand by the jamb and open with F.", "Behind this door the corridor runs both ways: two corners at once.", "Slice one side from inside the room, then the other.", "When you step out you turn your back on one side. Only step out when both are clear."],
        objs: &["Open the door from the side, off the doorway (F)", "Slice both sides of the corridor without leaving the room (70° of 180°)", "Neutralize the 2 threats"] },
    Lesson { title: "Dead space", kicker: "Lesson 6 of 6 · Room C", corners: Some("C"),
        lead: "Tall furniture creates areas you can't see from the door or from the middle of the room. While any floor is still red, the room is not clear.",
        points: &["Treat the furniture like a corner: slice it in an arc, from a distance.", "Don't walk right past the furniture.", "Hostage (white) and threat (tan) can be side by side. Identify each one."],
        objs: &["Open the door from the side, off the doorway (F)", "Neutralize the 2 threats", "Don't hit the hostage (white target)", "Clear the dead space behind the cabinet"] },
];

pub struct Ctx<'a> { pub p: &'a Player, pub game: &'a Game, pub tac: &'a Tac, pub targets: &'a Targets, pub level: &'a Level }
const ZA: usize = 3; const ZB: usize = 4; const ZC2: usize = 2; const ZC3: usize = 5; const ZC: usize = 6;

fn tri(done: bool, fail: bool) -> u8 { if fail { 2 } else if done { 1 } else { 0 } }
fn entry(e: Option<f32>, min: f32) -> u8 { match e { None => 0, Some(v) => if v >= min { 1 } else { 2 } } }

/// 0 pendente · 1 cumprido · 2 falhou
pub fn check(l: usize, i: usize, c: &Ctx) -> u8 {
    let (p, g, t, tg) = (c.p, c.game, c.tac, c.targets);
    match (l, i) {
        (0, 0) => tri(p.look_acc > 2.5, false),
        (0, 1) => tri(p.move_acc > 3.0, false),
        (0, 2) => tri(p.ads_acc > 0.6, false),
        (0, 3) => tri(p.lean_acc > 0.5, false),
        (0, 4) => tri(p.crouch_acc > 0.6, false),
        (0, 5) => tri(tg.down(&["P0"]), false),
        (0, 6) => tri(p.reloads > 0, false),
        (0, 7) => tri(g.flag_tac, false),
        (0, 8) => tri(g.flag_bird_back, false),
        (0, 9) => tri(g.flag_flash, false),
        (1, 0) => tri(p.pos.y < 8.8 && p.zone != 0, false),
        (1, 1) => tri(t.primary.map_or(false, |e| e.a.distance(Vec2::new(5.9, 5.9)) < 0.4 && e.hidden_deg > 4.0) && t.body_cm <= 15.0 && p.lean.abs() > 0.5, false),
        (1, 2) => match tg.get("T1").first_facing { None => 0, Some(true) => 1, Some(false) => 2 },
        (1, 3) => tri(tg.down(&["T1"]), false),
        (1, 4) => tri(t.pct[ZC2] >= 0.9, false),
        (2, 0) => entry(g.entry_pie[0], 110.0),
        (2, 1) => if p.zone == ZA && p.pos.y > 6.9 { if g.dwell[0] <= 1.5 { 1 } else { 2 } } else { 0 },
        (2, 2) => tri(c.level.corners.iter().filter(|k| k.zone == "A").all(|k| k.seen), false),
        (2, 3) => tri(tg.down(&["T2", "T3"]), false),
        (2, 4) => tri(t.pct[ZA] >= 0.95, false),
        (3, 0) => entry(g.entry_pie[1], 100.0),
        (3, 1) => tri(tg.down(&["T4", "T5"]), false),
        (3, 2) => tri(tg.down(&["T4", "T5"]) && t.pct[ZB] >= 0.95, !tg.get("N1").alive || !tg.get("N2").alive),
        (3, 3) => tri(t.pct[ZB] >= 0.95, false),
        (4, 0) => match g.door_side[0] { None => 0, Some(true) => 1, Some(false) => 2 },
        (4, 1) => match g.entry_pie[2] { None => tri(t.pie_deg(3) >= 70.0 && c.level.doors[0].open > 0.9, false), Some(v) => if v >= 70.0 { 1 } else { 2 } },
        (4, 2) => tri(tg.down(&["T6", "T7"]), false),
        (5, 0) => match g.door_side[1] { None => 0, Some(true) => 1, Some(false) => 2 },
        (5, 1) => tri(tg.down(&["T8", "T9"]), false),
        (5, 2) => tri(tg.down(&["T8", "T9"]) && t.pct[ZC] >= 0.95, !tg.get("N3").alive),
        (5, 3) => tri(t.pct[ZC] >= 0.95, false),
        _ => 1,
    }
}
fn pct(v: f32) -> String { format!("{}%", (v * 100.0).round() as i32) }
pub fn live(l: usize, i: usize, c: &Ctx) -> Option<String> {
    let (g, t) = (c.game, c.tac);
    match (l, i) {
        (1, 1) => t.primary.map(|_| format!("{} cm", t.body_cm.round() as i32)),
        (1, 4) => Some(pct(t.pct[ZC2])),
        (2, 0) => Some(format!("{}°", g.entry_pie[0].unwrap_or(t.pie_deg(1)).round() as i32)),
        (2, 1) => Some(format!("{:.1} s", g.dwell[0])),
        (2, 4) => Some(pct(t.pct[ZA])),
        (3, 0) => Some(format!("{}°", g.entry_pie[1].unwrap_or(t.pie_deg(2)).round() as i32)),
        (3, 3) => Some(pct(t.pct[ZB])),
        (4, 1) => Some(format!("{}°", g.entry_pie[2].unwrap_or(t.pie_deg(3)).round() as i32)),
        (5, 3) => Some(pct(t.pct[ZC])),
        _ => None,
    }
}
pub fn fail_msg(l: usize, i: usize) -> Option<&'static str> {
    match (l, i) {
        (1, 2) => Some("You appeared in the corridor without looking at it. Go around the corner facing the slice that is opening."),
        (2, 0) => Some("You entered with too little of the fan open. From the corridor, move from one side of the door to the other, gun on the opening, before entering."),
        (2, 1) => Some("You stood still in the doorway. In the fatal funnel, whoever stops becomes the target."),
        (3, 0) => Some("You entered with too little of the fan open. From inside Room A, open the door angle in an arc."),
        (3, 2) => Some("You hit a white target (no-shoot). Confirm the colour before pressing the trigger."),
        (4, 0) | (5, 0) => Some("You opened the door standing in front of it. Stay by the jamb."),
        (4, 1) => Some("You stepped into the corridor with one side still red."),
        (5, 2) => Some("You hit the hostage (white target). Identify every target before firing."),
        _ => None,
    }
}
pub fn waypoint(c: &Ctx) -> Option<(Vec2, &'static str)> {
    let g = c.game;
    if g.free { return None; }
    match g.lesson {
        1 if g.obj.first() == Some(&0) => Some((Vec2::new(5.0, 8.2), "Corner")),
        2 if g.entry[0].is_none() => Some((Vec2::new(10.0, 5.1), "Room A")),
        3 if g.entry[1].is_none() => Some((Vec2::new(13.2, 10.95), "Room B")),
        4 if c.level.doors[0].open < 0.1 => Some((Vec2::new(17.55, 7.0), "Door")),
        5 if c.level.doors[1].open < 0.1 => Some((Vec2::new(19.55, 5.0), "Room C")),
        _ => None,
    }
}

pub fn start_lesson(game: &mut Game, tac: &mut Tac, i: usize) {
    game.lesson = i; game.obj = vec![0; LESSONS[i].objs.len()]; game.lesson_end = 0.0; game.ui_dirty = true; game.track.clear(); game.guide_i = 0; game.coach = None; game.guide_t = TIP_SECS; game.lroute_sum = 0.0; game.lroute_t = 0.0;
    tac.corner_zone = LESSONS[i].corners;
    game.mode = Mode::Card;
}

pub fn update_lessons(time: Res<Time>, mut game: ResMut<Game>, mut tac: ResMut<Tac>, p: Res<Player>, targets: Res<Targets>, level: Res<Level>, mut sfx: EventWriter<Sfx>) {
    if game.mode != Mode::Play { return; }
    let dt = time.delta_secs().min(0.05);
    if game.free {
        if !game.debriefed && targets.list.iter().all(|t| t.def.kind != TKind::Threat || !t.alive) {
            game.lesson_end += dt;
            if game.lesson_end > 1.5 { game.mode = Mode::Debrief; game.debriefed = true; game.ui_dirty = true; sfx.write(Sfx::Good); }
        }
        return;
    }
    let l = game.lesson;
    // amostra o trajeto para o replay
    if game.track.last().map_or(true, |t| game.time - t.0 >= 0.1) && game.track.len() < 900 {
        let hid = tac.primary.map_or(0.0, |e| e.hidden_deg);
        let sample = (game.time, p.pos, p.yaw, hid);
        game.track.push(sample);
    }
    let mut open = 0;
    let mut changes: Vec<(usize, u8)> = Vec::new();
    {
        let ctx = Ctx { p: &p, game: &game, tac: &tac, targets: &targets, level: &level };
        for i in 0..game.obj.len() {
            if game.obj[i] != 0 { continue; }
            let r = check(l, i, &ctx);
            if r == 0 { open += 1; } else { changes.push((i, r)); }
        }
    }
    for (i, r) in changes {
        game.obj[i] = r; game.ui_dirty = true;
        if game.demo { continue; }
        if r == 1 { sfx.write(Sfx::Good); } else {
            sfx.write(Sfx::Bad); game.stats.failed += 1;
            if let Some(m) = fail_msg(l, i) { game.toast(m, 2, 6.0); }
        }
    }
    if open == 0 && !game.demo {
        if game.lesson_end == 0.0 { let msg = format!("Lesson complete: {}", LESSONS[l].title); game.toast(&msg, 1, 2.5); }
        game.lesson_end += dt;
        if game.lesson_end > 1.8 {
            let next = if l + 1 < LESSONS.len() { Some(l + 1) } else { None };
            if l >= 1 && game.track.len() > 20 { game.mode = Mode::Replay; game.replay_t = 0.0; game.replay_next = next; game.ui_dirty = true; }
            else { after_replay(&mut game, &mut tac, next); if next.is_none() { sfx.write(Sfx::Good); } }
        }
    }
}

fn after_replay(game: &mut Game, tac: &mut Tac, next: Option<usize>) {
    match next { Some(i) => start_lesson(game, tac, i), None => { game.mode = Mode::Debrief; game.debriefed = true; game.ui_dirty = true; } }
}
pub const REPLAY_SECS: f32 = 7.0;
/// Índice da amostra mostrada agora no replay.
pub fn replay_index(game: &Game) -> usize {
    let n = game.track.len().max(1);
    (((n - 1) as f32) * (game.replay_t / REPLAY_SECS).clamp(0.0, 1.0)) as usize
}
/// Replay da entrada, visto de cima: trajeto percorrido e direção do olhar ao longo do tempo.
pub fn replay_system(time: Res<Time>, mut game: ResMut<Game>, mut tac: ResMut<Tac>, c: Res<Controls>, mut gz: Gizmos, mut sfx: EventWriter<Sfx>) {
    if game.mode != Mode::Replay { return; }
    game.replay_t += time.delta_secs().min(0.05);
    let n = game.track.len();
    let idx = replay_index(&game);
    let y = 3.9;
    let v3 = |q: Vec2| Vec3::new(q.x, y, q.y);
    for i in 1..n {
        let (a, b) = (game.track[i - 1], game.track[i]);
        let col = if i <= idx { if b.3 > 45.0 { Color::srgb(1.0, 0.35, 0.28) } else if b.3 > 15.0 { Color::srgb(0.95, 0.7, 0.23) } else { Color::srgb(0.44, 0.84, 0.56) } } else { Color::srgba(1.0, 1.0, 1.0, 0.18) };
        gz.line(v3(a.1), v3(b.1), col);
    }
    if let Some(s) = game.track.get(idx) {
        let f = Vec2::new(-s.2.sin(), -s.2.cos());
        gz.circle(Isometry3d::new(v3(s.1), Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)), 0.28, Color::WHITE);
        gz.line(v3(s.1), v3(s.1 + f * 1.4), Color::srgb(0.38, 0.84, 0.91));
        let l = Vec2::new(-(s.2 + 0.87).sin(), -(s.2 + 0.87).cos());
        let r = Vec2::new(-(s.2 - 0.87).sin(), -(s.2 - 0.87).cos());
        gz.line(v3(s.1), v3(s.1 + l * 2.2), Color::srgba(0.38, 0.84, 0.91, 0.5));
        gz.line(v3(s.1), v3(s.1 + r * 2.2), Color::srgba(0.38, 0.84, 0.91, 0.5));
    }
    if game.replay_t > REPLAY_SECS + 1.5 || (c.confirm && game.replay_t > 0.3) {
        let next = game.replay_next;
        after_replay(&mut game, &mut tac, next);
        if next.is_none() { sfx.write(Sfx::Good); }
    }
}

/* ------------------------------------------------------------------ traçado ideal ------------------------------------------------------------------ */

/// Passo do traçado ideal de uma lição. O mesmo roteiro alimenta a linha no chão, a demonstração e o teste automático.
#[derive(Clone, Copy)]
pub enum Step {
    /// Onde a demonstração começa (posição e para onde olha).
    Start(f32, f32, f32, f32),
    /// Vá até (x, z) olhando para (x, z).
    Go(f32, f32, f32, f32),
    /// Vá até (x, z) sem mudar a direção do olhar.
    Mv(f32, f32),
    Shoot(&'static str), Open(usize), Lean(f32), Say(&'static str), Scan, Look(f32, f32),
    /// Abre a porta e segue direto para o próximo passo (sem esperar parado na frente dela).
    OpenNow(usize),
}

pub fn ideal(l: usize) -> &'static [Step] {
    use Step::*;
    match l {
        1 => &[
            Start(3.0, 14.9, 5.0, 13.0),
            Say("Walk along the side opposite the corner: the farther from it, the less body you show for each degree you open."),
            Mv(5.0, 13.0), Go(5.0, 9.0, 5.0, 5.0),
            Say("One side step at a time, gun already pointed at the new slice. Leaning opens a few more degrees without taking the step."),
            Go(4.7, 7.6, 8.0, 5.0), Lean(-1.0), Lean(0.0), Go(4.6, 6.6, 10.0, 5.0), Go(4.8, 5.7, 12.9, 5.0), Shoot("T1"),
            Say("Corridor seen to the end. Only now advance, and check the side left behind you."),
            Go(7.0, 5.0, 13.0, 5.0), Look(0.0, 5.0),
        ],
        2 => &[
            Start(7.0, 5.0, 13.0, 5.0),
            Say("Before entering: slice the door from outside, from one jamb to the other, hugging the far wall of the corridor."),
            Go(7.0, 4.45, 13.5, 7.0), Go(9.0, 4.45, 13.2, 11.2), Shoot("T2"), Go(10.0, 4.45, 10.0, 11.0), Go(11.0, 4.45, 7.0, 11.0), Go(13.0, 4.45, 6.6, 7.0),
            Say("What is left of the fan are the two near corners. Cross the doorway without stopping and check one, then the other."),
            Go(10.0, 5.2, 10.0, 9.0), Go(10.0, 7.3, 13.45, 6.5), Shoot("T3"), Look(6.5, 6.5),
            Say("Get off the door line and clear what the barricade hides."),
            Mv(8.0, 8.2), Scan, Mv(8.0, 10.6), Go(11.0, 10.6, 12.5, 7.5),
            Say("Room clear. Finish by covering the next opening: the door to Room B."),
            Look(14.0, 10.95),
        ],
        3 => &[
            Start(10.5, 8.0, 14.0, 10.9),
            Say("Corner door: the whole room lies to one side. Slice it in an arc from inside Room A."),
            Go(12.5, 8.2, 15.0, 12.0), Go(12.2, 9.5, 17.0, 11.5), Go(12.2, 10.95, 20.5, 10.95), Go(12.6, 11.6, 20.0, 7.0), Shoot("T4"), Go(13.4, 11.6, 16.0, 6.5),
            Say("Identify before you shoot: tan is a threat, white with an X is a no-shoot."),
            Mv(13.4, 10.95), Go(14.9, 10.95, 14.75, 6.85), Shoot("T5"), Mv(16.0, 10.6), Scan, Mv(19.5, 10.0), Scan,
        ],
        4 => &[
            Start(19.5, 10.0, 19.5, 7.6),
            Say("Closed door: open it from the side, by the jamb, off the line of whoever is on the other side."),
            Mv(19.5, 7.6), Mv(16.85, 7.6), Go(16.85, 6.45, 17.5, 6.0), OpenNow(0), Shoot("T7"),
            Say("Two corners at once. One side of the corridor is done; now the other, without leaving the room."),
            Go(18.25, 6.45, 14.6, 4.7), Shoot("T6"),
            Say("What is left hugs the door wall. Step into the junction only now, and check one side, then the other."),
            Go(17.55, 6.3, 17.55, 4.6), Go(17.55, 5.45, 14.6, 5.4), Look(20.6, 5.4), Look(14.4, 5.2),
        ],
        5 => &[
            Start(17.55, 5.45, 17.55, 4.6), Open(0),
            Mv(17.55, 5.0), Scan,
            Say("Another closed door: from the side, by the jamb."),
            Go(18.4, 4.5, 19.5, 4.0), Open(1),
            Say("Slice the door from outside. Then treat the cabinet like a corner: from a distance, in an arc, never right past it."),
            Go(18.3, 4.5, 20.5, 1.0), Go(19.55, 5.5, 19.55, 0.5), Go(20.7, 4.5, 16.0, 2.0),
            Go(19.55, 4.6, 20.4, 0.7), Shoot("T9"), Go(19.55, 3.2, 16.0, 3.2), Go(18.6, 3.2, 15.8, 1.0), Go(16.9, 3.1, 16.2, 0.9), Shoot("T8"), Scan, Mv(16.0, 1.5), Scan,
        ],
        _ => &[],
    }
}

/// Pontos do traçado (posição e, quando houver, para onde olhar).
pub fn ideal_points(l: usize) -> Vec<(Vec2, Option<Vec2>)> {
    ideal(l).iter().filter_map(|s| match *s {
        Step::Start(x, z, lx, lz) | Step::Go(x, z, lx, lz) => Some((Vec2::new(x, z), Some(Vec2::new(lx, lz)))),
        Step::Mv(x, z) => Some((Vec2::new(x, z), None)),
        _ => None,
    }).collect()
}

fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() < 1e-6 { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
    p.distance(a + ab * t)
}
/// Distância do operador ao trecho que ainda falta do traçado.
pub fn guide_dist(game: &Game, pos: Vec2) -> Option<f32> {
    let pts = ideal_points(game.lesson);
    let n = pts.len();
    if n < 2 || game.guide_i >= n { return None; }
    let from = game.guide_i.saturating_sub(1);
    let mut best = pos.distance(pts[game.guide_i].0);
    for k in from..n - 1 { best = best.min(seg_dist(pos, pts[k].0, pts[k + 1].0)); }
    Some(best)
}

/// Linha tracejada no chão com o traçado ideal da lição: setas no sentido do percurso e, em cada parada, para onde apontar a arma.
pub fn guide_system(mut gz: Gizmos, mut game: ResMut<Game>, p: Res<Player>, time: Res<Time>) {
    if game.mode != Mode::Play || game.free { return; }
    let dt = time.delta_secs().min(0.05);
    let pts = ideal_points(game.lesson);
    let n = pts.len();
    if n < 2 { return; }
    let mut i = game.guide_i.min(n);
    for j in i..(i + 3).min(n) { if p.pos.distance(pts[j].0) < 0.55 { i = j + 1; } }
    if i != game.guide_i { game.guide_i = i; }
    // nota do traçado: distância média à linha enquanto o jogador percorre a lição (conta depois de chegar ao primeiro ponto)
    if !game.demo && i >= 1 && i < n && p.speed > 0.25 {
        if let Some(d) = guide_dist(&game, p.pos) { game.route_sum += d * dt; game.route_t += dt; game.lroute_sum += d * dt; game.lroute_t += dt; }
    }
    if game.guide == 2 && game.guide_t > 0.0 { game.guide_t -= dt; if game.guide_t <= 0.0 { game.ui_dirty = true; } }
    if !game.route_visible() || i >= n { return; }
    // no modo dica a linha some aos poucos no último segundo e meio
    let fade = if game.guide == 2 && !game.demo { (game.guide_t / 1.5).clamp(0.0, 1.0) } else { 1.0 };
    let amber = Color::srgba(1.0, 0.8, 0.25, fade);
    let cyan = Color::srgba(0.38, 0.84, 0.91, fade);
    let white = Color::srgba(1.0, 1.0, 1.0, fade);
    let rot = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    for y in [0.045f32, 3.86] {
        let v3 = |q: Vec2| Vec3::new(q.x, y, q.y);
        for k in i.saturating_sub(1)..n - 1 {
            let (a, b) = (pts[k].0, pts[k + 1].0);
            let len = a.distance(b);
            if len < 0.05 { continue; }
            let d = (b - a) / len;
            let mut s = 0.0;
            while s < len { gz.line(v3(a + d * s), v3(a + d * (s + 0.2).min(len)), amber); s += 0.36; }
            let (m, nr) = (a + d * (len * 0.5), d.perp());
            gz.line(v3(m + d * 0.13), v3(m - d * 0.08 + nr * 0.11), amber);
            gz.line(v3(m + d * 0.13), v3(m - d * 0.08 - nr * 0.11), amber);
        }
        for k in i..n {
            let (q, look) = pts[k];
            let next = k == i;
            gz.circle(Isometry3d::new(v3(q), rot), if next { 0.2 + 0.04 * (time.elapsed_secs() * 5.0).sin() } else { 0.08 }, if next { white } else { amber });
            if let Some(lk) = look {
                let d = (lk - q).normalize_or_zero();
                let (tip, nr) = (q + d * 0.6, d.perp());
                gz.line(v3(q + d * 0.1), v3(tip), cyan);
                gz.line(v3(tip), v3(tip - d * 0.12 + nr * 0.07), cyan);
                gz.line(v3(tip), v3(tip - d * 0.12 - nr * 0.07), cyan);
            }
        }
    }
}

/// Mede uma derrubada: ângulo da fatia e quanto do corpo o alvo via naquele instante.
pub fn kill_metrics(level: &Level, p: &Player, t: &TargetState, now: f32, a_zone: bool) -> Kill {
    let (e, tp) = (p.eye2(), Vec2::new(t.def.x, t.def.z));
    let v = tp - e;
    let d = v.length().max(0.2);
    let a0 = v.y.atan2(v.x);
    // ângulo da fatia: quantos graus separam o alvo da borda mais próxima do que encobre a visão (parede, batente, móvel alto)
    let mut angle = 45.0;
    'scan: for k in 1..=90 {
        let da = (k as f32 * 0.5).to_radians();
        for s in [-1.0f32, 1.0] {
            let a = a0 + s * da;
            if level.cast_ray(e.x, e.y, a.cos(), a.sin(), d, p.eye.y) < d - 0.15 { angle = k as f32 * 0.5; break 'scan; }
        }
    }
    // exposição: largura do tronco (50 cm) com linha de visada para o alvo
    let perp = (p.pos - tp).normalize_or_zero().perp();
    let h = p.head_h() - 0.35;
    let seen = (-5..=5).filter(|k| { let q = p.pos + perp * (*k as f32 / 5.0 * BODY_R); level.los(tp.x, tp.y, 1.4, q.x, q.y, h) }).count();
    let expo_cm = seen as f32 / 11.0 * BODY_R * 200.0;
    let dt = if t.contact { (now - t.contact_t).max(0.0) } else { 0.0 };
    // três partes: ângulo da fatia (40%), corpo à mostra (35%) e tempo sob a linha de tiro da ameaça (25%; a ameaça reage em 1,9 s)
    let (ang_s, exp_s, time_s) = ((1.0 - (angle - 6.0) / 30.0).clamp(0.0, 1.0), 1.0 - 0.7 * ((expo_cm - 10.0) / 40.0).clamp(0.0, 1.0), (1.0 - (dt - 0.6) / 1.3).clamp(0.0, 1.0));
    Kill { angle, expo_cm, dt, pts: 100.0 * (0.4 * ang_s + 0.35 * exp_s + 0.25 * time_s), a_zone }
}

/// Badges com os nomes técnicos de cada fundamento (nome, o que foi feito para ganhar).
pub const BADGES: [(&str, &str); 10] = [
    ("SLICE THE PIE", "opened 110° or more of a doorway's fan before entering"),
    ("FATAL FUNNEL", "crossed a doorway in under a second, without stopping in it"),
    ("DIG THE CORNERS", "checked every near corner within 3 seconds of entering"),
    ("POSITIVE ID", "cleared a room with no-shoot targets without hitting one"),
    ("MINIMUM EXPOSURE", "dropped a threat showing 10 cm of body or less"),
    ("TIGHT ANGLE", "dropped a threat at a slice angle of 8° or less"),
    ("SEE FIRST, SHOOT FIRST", "dropped a threat within a second of first contact"),
    ("DOOR FROM THE JAMB", "opened a closed door from the side, off the doorway"),
    ("DEAD SPACE", "cleared the area hidden behind the tall furniture"),
    ("ON THE LINE", "finished a lesson within 25 cm of the ideal route on average"),
];

/* ------------------------------------------------------------------ avisos de técnica ------------------------------------------------------------------ */

pub const WARN_NAMES: [&str; 6] = ["too close to the corner", "sliced too fast", "gun off the opening", "stopped in the doorway", "back to uncleared area", "off the route"];

#[derive(Default)]
pub struct CoachState {
    last_time: f32, near_t: f32, gun_t: f32, expo_t: f32, off_t: f32, door_t: [f32; MAX_DOORWAYS], door_warned: [bool; MAX_DOORWAYS], kind_cd: [f32; 6],
    fan: VecDeque<(f32, f32)>, fan_i: Option<usize>, entered: [bool; MAX_DOORWAYS], entry_t: [f32; MAX_DOORWAYS], funnel_done: [bool; MAX_DOORWAYS],
}

/// Observa como o jogador fatia e se movimenta, e avisa na hora o que corrigir (com intervalo mínimo entre avisos).
pub fn coach_system(time: Res<Time>, mut game: ResMut<Game>, p: Res<Player>, tac: Res<Tac>, level: Res<Level>, targets: Res<Targets>, mut sfx: EventWriter<Sfx>, mut st: Local<CoachState>) {
    if game.mode != Mode::Play { return; }
    let dt = time.delta_secs().min(0.05);
    if game.time < st.last_time { *st = CoachState::default(); }
    st.last_time = game.time;
    if let Some(c) = game.coach.as_mut() { c.2 -= dt; if c.2 <= 0.0 { game.coach = None; game.ui_dirty = true; } }
    if game.demo { return; }

    /* chances de sucesso ao vivo: perfeição do traçado + angulação (nota das derrubadas e exposição do momento) */
    let route_now = if game.free { None } else { guide_dist(&game, p.pos).filter(|_| game.guide_i >= 1).map(|d| (1.0 - (d - 0.2) / 1.0).clamp(0.0, 1.0) * 100.0) };
    let route = match (route_now, game.route_score()) { (Some(a), Some(b)) => 0.5 * a + 0.5 * b, (Some(a), None) => a, (None, Some(b)) => b, _ => 100.0 };
    let mut angle = game.kill_score().unwrap_or(100.0);
    // agora: área não limpa fora do setor e corpo à mostra na quina em foco derrubam as chances
    // (nos primeiros segundos de um percurso o operador ainda nem olhou em volta: a penalidade entra aos poucos)
    angle -= (tac.expo_deg / 180.0).min(1.0) * 35.0 * ((game.time - 3.0) / 5.0).clamp(0.0, 1.0);
    // tempo exposto: quanto mais tempo sob a linha de tiro de uma ameaça viva, menores as chances (ela reage em 1,9 s)
    angle -= (game.expo_now / 1.9).min(1.0) * 45.0;
    if tac.pie_active.is_some() && tac.primary.is_some() { angle -= ((tac.body_cm - 15.0) / 35.0).clamp(0.0, 1.0) * 15.0; }
    let angle = angle.clamp(0.0, 100.0);
    let want = ((if game.free { angle } else { 0.45 * route + 0.55 * angle }) - game.stats.hits_taken as f32 * 10.0 - game.stats.civ_hit as f32 * 20.0).clamp(3.0, 99.0);
    game.odds = damp(game.odds, want, 3.0, dt); game.odds_route = damp(game.odds_route, route, 3.0, dt); game.odds_angle = damp(game.odds_angle, angle, 3.0, dt);

    /* badges */
    let mut earn: Vec<usize> = Vec::new();
    if game.entry_pie.iter().any(|e| e.map_or(false, |v| v >= 110.0)) { earn.push(0); }
    for (k, w) in doorways().iter().enumerate() {
        if game.entry[k].is_some() && !st.funnel_done[k] && zone_index(w.zone) == p.zone && p.pos.distance(Vec2::new(w.cx, w.cz)) > 1.0 {
            st.funnel_done[k] = true;
            if game.dwell[k] <= 1.0 { earn.push(1); }
        }
        if let Some(z) = tac.corner_zone {
            if z == w.zone && game.entry[k].is_some() && st.entered[k] && game.time - st.entry_t[k] <= 3.0 && level.corners.iter().filter(|c| c.zone == z).all(|c| c.seen) { earn.push(2); }
        }
    }
    for z in zones().iter().map(|q| q.id) {
        let here = |k: TKind| targets.list.iter().filter(move |t| t.def.zone == z && t.def.kind == k && !t.off);
        if here(TKind::Civil).count() > 0 && here(TKind::Civil).all(|t| t.alive) && here(TKind::Threat).count() > 0 && here(TKind::Threat).all(|t| !t.alive) { earn.push(3); }
    }
    if game.kills.iter().any(|k| k.expo_cm <= 10.0) { earn.push(4); }
    if game.kills.iter().any(|k| k.angle <= 8.0) { earn.push(5); }
    if game.kills.iter().any(|k| k.dt <= 1.0) { earn.push(6); }
    if game.door_side.iter().any(|d| *d == Some(true)) { earn.push(7); }
    // espaço morto: a Sala C da casa do tutorial; nos outros mapas, a casa inteira vista (há estantes e armários altos em todos)
    if (map_index() == 0 && tac.pct[6] >= 0.95) || (map_index() != 0 && (0..zones().len()).all(|z| tac.pct[z] >= 0.95)) { earn.push(8); }
    if !game.free && game.lesson >= 1 && game.lesson_end > 0.0 && game.lroute_t > 3.0 && game.lroute_sum / game.lroute_t <= 0.25 { earn.push(9); }
    for i in earn {
        if game.badges & (1 << i) == 0 {
            game.badges |= 1 << i;
            let msg = format!("BADGE EARNED · {} · {}", BADGES[i].0, BADGES[i].1);
            game.toast(&msg, 3, 5.5);
            sfx.write(Sfx::Good);
        }
    }

    if !game.free && game.lesson == 0 { return; }
    game.coach_cd -= dt;
    for k in st.kind_cd.iter_mut() { *k -= dt; }
    let eye = p.eye2();
    let mut warn: Option<(usize, String)> = None;
    let threats_here = targets.list.iter().any(|t| t.alive && t.def.kind == TKind::Threat && !t.def.passive && zone_index(t.def.zone) == p.zone);
    let zone_ok = tac.pct[p.zone] >= 0.9 && !threats_here;

    // abertura em foco: a mais próxima que dá para uma área ainda não limpa
    let mut focus = false;
    if let Some(i) = tac.pie_active {
        let o = &openings()[i];
        let closed = o.door.map_or(false, |d| level.doors[d].open < 0.5);
        let deg = tac.pie_deg(i);
        let dist = eye.distance(o.mid());
        if !closed && deg < 100.0 {
            focus = true;
            // 0 · colado na quina ou no batente antes de fatiar
            if o.door.is_none() {
                let dj = if i == 0 { p.pos.distance(o.b) } else { p.pos.distance(o.a).min(p.pos.distance(o.b)) };
                if dj < 0.7 { st.near_t += dt; } else { st.near_t = 0.0; }
                if st.near_t > 0.5 { warn = Some((0, format!("You are {} cm from the corner. Step away from the wall: from farther back you open the same degrees showing less body.", (dj * 100.0).round() as i32))); }
            }
            // 1 · o leque varre rápido demais para dar tempo de olhar
            let fan = o.fan(eye);
            if st.fan_i != Some(i) { st.fan.clear(); st.fan_i = Some(i); }
            match fan {
                Some((lo, hi)) => {
                    st.fan.push_back((game.time, (lo + hi) / 2.0));
                    while st.fan.front().map_or(false, |f| game.time - f.0 > 0.5) { st.fan.pop_front(); }
                    if let (Some(a), Some(b)) = (st.fan.front(), st.fan.back()) {
                        let span = b.0 - a.0;
                        if span > 0.35 && p.speed > 1.0 && o.door.is_none() {
                            let rate = (b.1 - a.1).abs() / span;
                            if rate > 95.0 { warn = Some((1, format!("You opened {}° per second: too fast to see what appears. One side step at a time, pausing to look.", rate.round() as i32))); }
                        }
                    }
                }
                None => st.fan.clear(),
            }
            if p.fast && dist < 4.0 { warn = Some((1, "Fast step with the gun low a few metres from an uncleared opening. Slow down and point at the opening.".into())); }
            // 2 · olhos e cano fora da abertura
            if zone_ok && dist < 3.2 && fan.is_some() && tac.pie_now[i].is_none() && level.los(eye.x, eye.y, p.eye.y, o.mid().x, o.mid().y, 1.2) { st.gun_t += dt; } else { st.gun_t = 0.0; }
            if st.gun_t > 1.5 { st.gun_t = 0.0; warn = Some((2, "Your gun is not on the opening. Eyes and muzzle together on the slice you are opening.".into())); }
        }
    }
    if !focus { st.near_t = 0.0; st.gun_t = 0.0; st.fan.clear(); st.fan_i = None; }

    // 3 · parado no vão da porta (funil fatal)
    for (k, w) in doorways().iter().enumerate() {
        let zi = zone_index(w.zone);
        let inside = p.pos.x >= w.x1 && p.pos.x <= w.x2 && p.pos.y >= w.z1 && p.pos.y <= w.z2;
        let hot = tac.pct[zi] < 0.95 || targets.list.iter().any(|t| t.alive && t.def.kind == TKind::Threat && t.def.zone == w.zone);
        if inside && hot { st.door_t[k] += dt; } else { st.door_t[k] = 0.0; }
        if st.door_t[k] > 1.0 && !st.door_warned[k] { st.door_warned[k] = true; warn = Some((3, "You stopped in the doorway. In the fatal funnel whoever stops becomes the target: go through and get off the door line.".into())); }
        // acerto: entrou com bastante fatia aberta
        if !st.entered[k] {
            if let Some(v) = game.entry_pie[k] {
                st.entered[k] = true; st.entry_t[k] = game.time;
                if v >= (if map_index() == 0 { [110.0, 100.0, 70.0, 90.0][k.min(3)] } else { 100.0 }) && game.coach.is_none() {
                    game.coach = Some((format!("Good entry: {}° of the 180° fan already open before crossing the door. Only the near corners were left.", v.round() as i32), 1, 4.5));
                    game.ui_dirty = true;
                }
            }
        }
    }
    // 4 · muita área não limpa fora do setor
    if tac.expo_deg >= 90.0 && game.time > 8.0 { st.expo_t += dt; } else { st.expo_t = 0.0; }
    if st.expo_t > 2.5 { st.expo_t = 0.0; warn = warn.or(Some((4, format!("{}° of uncleared area outside your sector. Don't turn your back on what you haven't seen.", tac.expo_deg.round() as i32)))); }
    // 5 · longe do traçado ideal (só no tutorial, com a linha ligada)
    if !game.free && game.guide == 1 {
        match guide_dist(&game, p.pos) { Some(d) if d > 1.6 && game.guide_i >= 1 => st.off_t += dt, _ => st.off_t = 0.0 }
        if st.off_t > 3.0 { st.off_t = 0.0; warn = warn.or(Some((5, "You left the ideal route. Follow the dashed line on the floor (T turns the line on and off).".into()))); }
    }

    if let Some((k, msg)) = warn {
        if game.coach_cd <= 0.0 && st.kind_cd[k] <= 0.0 {
            game.coach = Some((msg, 2, 5.5)); game.coach_cd = 3.5; st.kind_cd[k] = 9.0; game.warns[k] += 1; game.ui_dirty = true;
            sfx.write(Sfx::Ui);
        }
    }
}

/* ------------------------------------------------------------------ demonstração ------------------------------------------------------------------ */

struct TSnap { alive: bool, fall: f32, fall_v: f32, hits: u8, seen: bool, vis: bool, react: f32, contact: bool, contact_t: f32, first_facing: Option<bool>, sw: f32, swv: f32, landed: bool, marks: usize }
pub struct Snap { game: Game, p: Player, seen: Vec<u8>, pie: Vec<[u8; 180]>, pct: [f32; MAX_ZONES], doors: Vec<Door>, solids: Vec<Solid>, corners: Vec<Corner>, targets: Vec<TSnap> }
/// Estado guardado antes da demonstração, devolvido quando ela termina: a demonstração não conta para a lição.
#[derive(Resource, Default)]
pub struct DemoSnap(pub Option<Box<Snap>>);

/// Modo demonstração: o operador percorre sozinho o traçado ideal da lição, visto de cima e com a visão tática ligada.
#[allow(clippy::too_many_arguments)]
pub fn demo_system(time: Res<Time>, mut game: ResMut<Game>, mut p: ResMut<Player>, mut tac: ResMut<Tac>, mut level: ResMut<Level>, mut targets: ResMut<Targets>,
    mut bot: ResMut<Bot>, mut snap: ResMut<DemoSnap>, mut commands: Commands) {
    if game.demo_req {
        game.demo_req = false;
        if ideal(game.lesson).is_empty() || game.demo { return; }
        snap.0 = Some(Box::new(Snap {
            game: game.clone(), p: p.clone(), seen: tac.seen.clone(), pie: tac.pie.clone(), pct: tac.pct, doors: level.doors.clone(), solids: level.solids.clone(), corners: level.corners.clone(),
            targets: targets.list.iter().map(|t| TSnap { alive: t.alive, fall: t.fall, fall_v: t.fall_v, hits: t.hits, seen: t.seen, vis: t.vis, react: t.react, contact: t.contact, contact_t: t.contact_t,
                first_facing: t.first_facing, sw: t.sw, swv: t.swv, landed: t.landed, marks: t.marks.len() }).collect(),
        }));
        p.vel = Vec2::ZERO; p.lean = 0.0; p.crouching = false; p.mag = 8; p.reloading = false; p.pitch = -0.04;
        game.mode = Mode::Play; game.demo = true; game.demo_end = false; game.demo_idle = 0.0; game.view = View::Bird; game.tac = true; game.toast = None; game.coach = None; game.guide_i = 0; game.ui_dirty = true;
        bot.demo_q = crate::bot::steps_cmds(game.lesson, true).into();
        bot.n = 0; bot.demo_frames = 0; bot.hold_ads = false; bot.hold_lean = 0.0; bot.hold_crouch = false;
        // no teste automático a demonstração roda num jogo recém-aberto: deixa a casa como estaria ao chegar nesta lição
        if bot.active {
            tac.corner_zone = LESSONS[game.lesson].corners;
            for t in targets.list.iter_mut().filter(|t| zone_index(t.def.zone) < game.lesson + 1) { t.alive = false; t.fall = 1.0; t.landed = true; }
        }
        return;
    }
    if !game.demo { return; }
    if bot.demo_q.is_empty() { game.demo_idle += time.delta_secs().min(0.05); }
    if !(game.demo_end || game.demo_idle > 2.5 || game.mode != Mode::Play) { return; }
    if bot.active { println!("BOT demo L{}: obj={:?} atingido={} fatias={:?}", game.lesson, game.obj, game.stats.hits_taken, (0..5).map(|i| tac.pie_deg(i) as i32).collect::<Vec<_>>()); }
    if let Some(s) = snap.0.take() {
        let s = *s;
        *game = s.game; *p = s.p;
        tac.seen = s.seen; tac.pie = s.pie; tac.pct = s.pct; tac.cur.fill(0);
        level.doors = s.doors; level.solids = s.solids; level.corners = s.corners;
        for (t, ts) in targets.list.iter_mut().zip(s.targets) {
            t.alive = ts.alive; t.fall = ts.fall; t.fall_v = ts.fall_v; t.hits = ts.hits; t.seen = ts.seen; t.vis = ts.vis; t.react = ts.react; t.contact = ts.contact; t.contact_t = ts.contact_t;
            t.first_facing = ts.first_facing; t.sw = ts.sw; t.swv = ts.swv; t.landed = ts.landed;
            while t.marks.len() > ts.marks { if let Some(m) = t.marks.pop() { commands.entity(m).despawn(); } }
        }
    }
    game.mode = Mode::Card; game.demo = false; game.demo_req = false; game.demo_end = false; game.ui_dirty = true;
    bot.demo_q.clear(); bot.n = 0; bot.hold_ads = false; bot.hold_lean = 0.0; bot.hold_crouch = false;
}
