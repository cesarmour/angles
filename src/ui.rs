//! Interface: HUD, cartões de lição, pausa e avaliação.
use crate::art;
use crate::combat::{Casing, Decal, Fx, Puff};
use crate::lessons::*;
use crate::map::*;
use crate::state::*;
use crate::tactical::Tac;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const BONE: Color = Color::srgb(0.914, 0.902, 0.863);
const MUTE: Color = Color::srgb(0.643, 0.659, 0.616);
const AMBER: Color = Color::srgb(0.949, 0.698, 0.227);
const GOOD: Color = Color::srgb(0.435, 0.835, 0.557);
const BAD: Color = Color::srgb(1.0, 0.353, 0.31);
const CYAN: Color = Color::srgb(0.376, 0.839, 0.91);
const PANEL: Color = Color::srgba(0.035, 0.05, 0.055, 0.8);
const SCREEN: Color = Color::srgba(0.02, 0.03, 0.035, 0.88);

#[derive(Resource)]
pub struct UiFonts { pub bold: Handle<Font>, pub semi: Handle<Font>, pub body: Handle<Font> }
#[derive(Resource, Default)]
pub struct Diagrams(pub Vec<Handle<Image>>);

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    Hud, LessonKick, LessonTitle, ObjList, TacPanel, TacFov, TacFovNote, TacCorner, TacCornerNote, TacBody, TacZoneName, TacZone, TacExpo, MiniFrame, Legend,
    AmmoMag, AmmoRes, AmmoMode, Stance, ViewTag, Prompt, Toast, ToastText, Wp, WpText, Hit, Dmg, Cross(u8), Label(u8),
    Coach, CoachKick, CoachText, DemoBar, DemoBtn, FlashBtn, GuideBtn, BoxBtn, LessonBox, Odds, OddsVal, OddsNote, OddsBar, BadgeList,
    Menu, Card, Pause, Debrief, Credits, MapBtn(u8), MapNote, Replay, ReplayTitle, ReplayBars, ReplayCursor, CardKick, CardTitle, CardLead, CardPoints, CardImg, DbGrade, DbScore, DbRows,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Act { Start, Free, CardGo, Resume, Restart, Quit, Mission2, Demo, Flash, Guide, Box, Credits, Back, Map(u8) }
#[derive(Event)]
pub struct ResetGame { pub free: bool, pub mission2: bool }

/// A fonte da interface (Rajdhani) tem letras mais baixas que a anterior: compensa no tamanho.
const FS: f32 = 1.1;
const CREDITS: &str = "ENGINE   Bevy 0.16 (MIT / Apache-2.0)  ·  resvg, usvg, tiny-skia (Apache-2.0 / MIT)\n\nFONTS   Rajdhani, by Indian Type Foundry  ·  Saira Stencil One, by Omnibus-Type  ·  SIL Open Font License 1.1\n\nTEXTURES AND FURNITURE MODELS   Poly Haven (polyhaven.com), CC0\n\nSOUNDS   Freesound.org, all CC0 1.0, recorded by: Anthousai, areniporgen, BenDrain, CosmoX45, discofield, dslrguide, florianreichelt, jessepash, Kinoton, Krokulator, kyles, mahecic, Manim8, michorvath, nioczkus, Nox_Sound, Physon712, RFCaldas, SecureSubset, Sheyvan, TampaJoey, vabadus, WasabiWielder, worthahep88\n\nPISTOL AND GLOVES   model \"animatedpistol\" downloaded from Sketchfab; author and licence still to be confirmed and credited before the game is distributed\n\nLEVELS, target_defs(), OPERATOR ART, ICON, INTERFACE, LESSONS AND CODE   made for this project with Claude Code\n\nFull list with links in CREDITS.md.";
fn txt(s: &str, font: &Handle<Font>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(s), TextFont { font: font.clone(), font_size: size * FS, ..default() }, TextColor(color))
}
fn abs() -> Node { Node { position_type: PositionType::Absolute, ..default() } }

fn button(p: &mut ChildSpawnerCommands, f: &UiFonts, label: &str, act: Act, primary: bool) {
    p.spawn((Button, Node { padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)), border: UiRect::all(Val::Px(1.5)), ..default() },
        BorderColor(if primary { AMBER } else { BONE }), BackgroundColor(if primary { AMBER } else { Color::NONE }), act))
        .with_children(|b| { b.spawn(txt(label, &f.bold, 19.0, if primary { Color::srgb(0.08, 0.06, 0.04) } else { BONE })); });
}
/// Botão cujo texto muda (ligado/desligado): o texto leva uma etiqueta para ser atualizado.
fn toggle_button(p: &mut ChildSpawnerCommands, f: &UiFonts, act: Act, tag: Tag) {
    p.spawn((Button, Node { padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)), border: UiRect::all(Val::Px(1.5)), ..default() }, BorderColor(CYAN), BackgroundColor(Color::NONE), act))
        .with_children(|b| { b.spawn((txt("", &f.bold, 19.0, BONE), tag)); });
}
fn screen(tag: Tag) -> (Node, BackgroundColor, Tag, GlobalZIndex) {
    (Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), display: Display::None, flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center, padding: UiRect::new(Val::Px(72.0), Val::Px(40.0), Val::Px(30.0), Val::Px(30.0)), row_gap: Val::Px(16.0), ..default() },
     BackgroundColor(SCREEN), tag, GlobalZIndex(5))
}

pub fn setup_ui(mut commands: Commands, mut fonts: ResMut<Assets<Font>>, mut images: ResMut<Assets<Image>>, mini: Res<crate::player::MiniTex>) {
    let f = UiFonts {
        bold: fonts.add(Font::try_from_bytes(art::FONT_BOLD.to_vec()).expect("fonte")),
        semi: fonts.add(Font::try_from_bytes(art::FONT_SEMI.to_vec()).expect("fonte")),
        body: fonts.add(Font::try_from_bytes(art::FONT_BODY.to_vec()).expect("fonte")),
    };
    let mut diagrams = Diagrams::default();
    for i in 0..6 { diagrams.0.push(images.add(art::svg_image(&art::diagram_svg(i), 960, 570, false))); }

    /* ---------- HUD ---------- */
    commands.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), display: Display::None, ..default() }, Tag::Hud)).with_children(|h| {
        h.spawn((Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::NONE), Tag::Dmg));
        // lição
        h.spawn((Node { left: Val::Px(16.0), top: Val::Px(16.0), width: Val::Px(380.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(12.0)), border: UiRect::left(Val::Px(2.0)), row_gap: Val::Px(3.0), ..abs() },
            BackgroundColor(PANEL), BorderColor(AMBER), Tag::LessonBox)).with_children(|p| {
            p.spawn((txt("", &f.semi, 13.0, AMBER), Tag::LessonKick));
            p.spawn((txt("", &f.bold, 26.0, BONE), Tag::LessonTitle));
            p.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(5.0), margin: UiRect::top(Val::Px(4.0)), ..default() }, Tag::ObjList));
        });
        // painel tático
        // painel tático, minimapa e legenda empilhados numa coluna: o painel pode crescer sem cobrir o minimapa
        h.spawn(Node { right: Val::Px(16.0), top: Val::Px(16.0), width: Val::Px(320.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), ..abs() }).with_children(|h| {
        h.spawn((Node { width: Val::Px(320.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(10.0)), border: UiRect::right(Val::Px(2.0)), row_gap: Val::Px(5.0), display: Display::None, ..default() },
            BackgroundColor(PANEL), BorderColor(CYAN), Tag::TacPanel)).with_children(|p| {
            p.spawn(txt("TACTICAL VIEW", &f.semi, 13.0, CYAN));
            let mut row = |p: &mut ChildSpawnerCommands, label: &str, ltag: Option<Tag>, vtag: Tag, note: Option<Tag>| {
                p.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::FlexEnd, ..default() }).with_children(|r| {
                    r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                        let mut e = c.spawn(txt(label, &f.body, 12.5, MUTE));
                        if let Some(t) = ltag { e.insert(t); }
                        if let Some(t) = note { c.spawn((txt("", &f.body, 11.0, MUTE), t)); }
                    });
                    r.spawn((txt("0", &f.bold, 25.0, BONE), TextLayout::new_with_no_wrap(), Node { flex_shrink: 0.0, margin: UiRect::left(Val::Px(8.0)), ..default() }, vtag));
                });
            };
            row(p, "YOUR SECTOR (EYES AND GUN)", None, Tag::TacFov, Some(Tag::TacFovNote));
            row(p, "OPENING: NOW / CLEARED (OF 180°)", None, Tag::TacCorner, Some(Tag::TacCornerNote));
            row(p, "BODY EXPOSED TO THE CORNER", None, Tag::TacBody, None);
            row(p, "ZONE", Some(Tag::TacZoneName), Tag::TacZone, None);
            row(p, "EXPOSURE TO UNCLEARED AREA", None, Tag::TacExpo, None);
        });
        h.spawn((Node { width: Val::Px(320.0), height: Val::Px(320.0), border: UiRect::all(Val::Px(1.0)), display: Display::None, ..default() }, ImageNode::new(mini.0.clone()), BorderColor(CYAN.with_alpha(0.6)), Tag::MiniFrame));
        h.spawn((Node { width: Val::Px(320.0), padding: UiRect::all(Val::Px(6.0)), column_gap: Val::Px(10.0), flex_wrap: FlexWrap::Wrap, display: Display::None, ..default() }, BackgroundColor(PANEL), Tag::Legend)).with_children(|l| {
            for (c, s) in [(CYAN, "covering now"), (GOOD, "seen"), (BAD, "not clear"), (AMBER, "corner limit")] {
                l.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(4.0), ..default() }).with_children(|i| {
                    i.spawn((Node { width: Val::Px(9.0), height: Val::Px(9.0), ..default() }, BackgroundColor(c)));
                    i.spawn(txt(s, &f.body, 11.0, MUTE));
                });
            }
        });
        });
        // retículo e marcador de acerto
        for k in 0..5u8 { h.spawn((Node { width: Val::Px(2.0), height: Val::Px(2.0), ..abs() }, BackgroundColor(Color::srgba(0.94, 0.94, 0.9, 0.9)), Tag::Cross(k))); }
        h.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, display: Display::None, ..abs() }, Tag::Hit))
            .with_children(|c| { c.spawn(txt("×", &f.body, 34.0, Color::WHITE)); });
        // munição e postura
        h.spawn(Node { right: Val::Px(16.0), bottom: Val::Px(14.0), flex_direction: FlexDirection::Column, align_items: AlignItems::FlexEnd, ..abs() }).with_children(|a| {
            a.spawn(Node { align_items: AlignItems::FlexEnd, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                r.spawn((txt("8", &f.bold, 58.0, BONE), Tag::AmmoMag));
                r.spawn((txt("/ 64", &f.semi, 24.0, MUTE), Tag::AmmoRes));
            });
            a.spawn((txt("", &f.body, 12.0, MUTE), Tag::AmmoMode));
        });
        h.spawn((Node { left: Val::Px(16.0), bottom: Val::Px(70.0), width: Val::Px(236.0), flex_direction: FlexDirection::Column, padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)), border: UiRect::left(Val::Px(2.0)), row_gap: Val::Px(2.0), display: Display::None, ..abs() },
            BackgroundColor(PANEL), BorderColor(CYAN), Tag::Odds)).with_children(|o| {
            o.spawn(txt("ODDS OF SUCCESS", &f.semi, 12.0, CYAN));
            o.spawn(Node { align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
                r.spawn((txt("—", &f.bold, 34.0, BONE), TextLayout::new_with_no_wrap(), Tag::OddsVal));
                r.spawn((Node { width: Val::Px(120.0), height: Val::Px(7.0), ..default() }, BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)))).with_children(|b| {
                    b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(GOOD), Tag::OddsBar));
                });
            });
            o.spawn((txt("", &f.body, 11.5, MUTE), Tag::OddsNote));
        });
        h.spawn(Node { left: Val::Px(16.0), bottom: Val::Px(14.0), flex_direction: FlexDirection::Column, ..abs() }).with_children(|a| {
            a.spawn((txt("STANDING", &f.semi, 20.0, BONE), Tag::Stance));
            a.spawn((txt("", &f.body, 12.0, MUTE), Tag::ViewTag));
        });
        h.spawn((Node { width: Val::Percent(100.0), top: Val::Percent(62.0), justify_content: JustifyContent::Center, display: Display::None, ..abs() }, Tag::Prompt)).with_children(|c| {
            c.spawn((Node { padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)), column_gap: Val::Px(8.0), align_items: AlignItems::Center, ..default() }, BackgroundColor(PANEL))).with_children(|r| {
                r.spawn((Node { padding: UiRect::axes(Val::Px(7.0), Val::Px(0.0)), ..default() }, BackgroundColor(AMBER))).with_children(|k| { k.spawn(txt("F", &f.bold, 18.0, Color::srgb(0.05, 0.05, 0.05))); });
                r.spawn(txt("OPEN DOOR", &f.semi, 18.0, BONE));
            });
        });
        h.spawn((Node { width: Val::Percent(100.0), top: Val::Percent(17.0), justify_content: JustifyContent::Center, display: Display::None, ..abs() }, Tag::Toast)).with_children(|c| {
            c.spawn((Node { max_width: Val::Px(580.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)), border: UiRect::top(Val::Px(2.0)), ..default() }, BackgroundColor(PANEL), BorderColor(AMBER)))
                .with_children(|t| { t.spawn((txt("", &f.body, 15.5, BONE), Tag::ToastText)); });
        });
        h.spawn((Node { width: Val::Percent(100.0), bottom: Val::Px(112.0), justify_content: JustifyContent::Center, display: Display::None, ..abs() }, Tag::Coach)).with_children(|c| {
            c.spawn((Node { max_width: Val::Px(640.0), padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)), border: UiRect::left(Val::Px(3.0)), column_gap: Val::Px(12.0), align_items: AlignItems::Center, ..default() }, BackgroundColor(PANEL), BorderColor(BONE)))
                .with_children(|t| {
                    t.spawn((txt("TECHNIQUE", &f.bold, 13.0, AMBER), Node { flex_shrink: 0.0, ..default() }, Tag::CoachKick));
                    t.spawn((txt("", &f.body, 15.5, BONE), Tag::CoachText));
                });
        });
        h.spawn((Node { width: Val::Percent(100.0), top: Val::Px(16.0), justify_content: JustifyContent::Center, display: Display::None, ..abs() }, Tag::DemoBar)).with_children(|c| {
            c.spawn((Node { padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)), border: UiRect::top(Val::Px(2.0)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(2.0), ..default() }, BackgroundColor(PANEL), BorderColor(CYAN)))
                .with_children(|t| {
                    t.spawn(txt("DEMO · IDEAL ROUTE", &f.bold, 20.0, CYAN));
                    t.spawn(txt("The operator runs the lesson on his own. ENTER or ESC returns to the card  ·  V switches the view", &f.body, 13.0, MUTE));
                });
        });
        h.spawn((Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), display: Display::None, ..abs() }, Tag::Wp)).with_children(|w| {
            w.spawn((Node { width: Val::Px(10.0), height: Val::Px(10.0), ..default() }, BackgroundColor(AMBER)));
            w.spawn((txt("", &f.semi, 14.0, BONE), Tag::WpText));
        });
        h.spawn((Node { width: Val::Percent(100.0), bottom: Val::Px(90.0), justify_content: JustifyContent::Center, display: Display::None, ..abs() }, Tag::Replay)).with_children(|c| {
            c.spawn((Node { width: Val::Px(660.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(12.0)), border: UiRect::top(Val::Px(2.0)), row_gap: Val::Px(6.0), ..default() }, BackgroundColor(PANEL), BorderColor(CYAN))).with_children(|p| {
                p.spawn((txt("", &f.bold, 24.0, BONE), Tag::ReplayTitle));
                p.spawn(txt("Your path seen from above. The line colour and the bars show how many degrees the nearest corner was still hiding at each moment.", &f.body, 13.5, MUTE));
                p.spawn((Node { height: Val::Px(64.0), align_items: AlignItems::FlexEnd, column_gap: Val::Px(1.0), ..default() }, Tag::ReplayBars));
                p.spawn((Node { height: Val::Px(3.0), width: Val::Px(8.0), ..default() }, BackgroundColor(Color::WHITE), Tag::ReplayCursor));
                p.spawn(txt("ENTER TO CONTINUE", &f.semi, 13.0, AMBER));
            });
        });
        for k in 0..7u8 { h.spawn((txt("", &f.bold, 20.0, BAD), Node { display: Display::None, ..abs() }, TextShadow { offset: Vec2::splat(1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.85) }, Tag::Label(k))); }
    });

    /* ---------- telas ---------- */
    commands.spawn(screen(Tag::Menu)).with_children(|s| {
        s.spawn(txt("CLOSE-QUARTERS BATTLE SIMULATOR · TUTORIAL", &f.semi, 14.0, AMBER));
        s.spawn(txt("POINT OF ", &f.bold, 104.0, BONE)).with_children(|t| { t.spawn((TextSpan::new("ENTRY"), TextFont { font: f.bold.clone(), font_size: 104.0 * FS, ..default() }, TextColor(AMBER))); });
        s.spawn((txt("A shoot house with cardboard targets and six short lessons on how to enter a room: see first, expose little and fire only at what you have identified.", &f.body, 19.0, BONE), Node { max_width: Val::Px(640.0), ..default() }));
        s.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
            button(r, &f, "START TUTORIAL", Act::Start, true);
            button(r, &f, "MISSION 2 · REVERSE ROUTE", Act::Mission2, false);
            button(r, &f, "FREE RUN", Act::Free, false);
            button(r, &f, "CREDITS", Act::Credits, false);
            button(r, &f, "QUIT GAME", Act::Quit, false);
        });
        // escolha do mapa para o percurso livre (o tutorial e a missão 2 usam sempre a casa de treino)
        s.spawn(Node { column_gap: Val::Px(10.0), align_items: AlignItems::Center, ..default() }).with_children(|r| {
            r.spawn(txt("MAP FOR FREE RUN", &f.semi, 13.0, CYAN));
            for (i, m) in MAPS.iter().enumerate() {
                r.spawn((Button, Node { padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)), border: UiRect::all(Val::Px(1.5)), ..default() }, BorderColor(CYAN), BackgroundColor(Color::NONE), Act::Map(i as u8)))
                    .with_children(|b| { b.spawn((txt(&format!("{} · {}", i + 1, m.name.to_uppercase()), &f.bold, 16.0, BONE), Tag::MapBtn(i as u8))); });
            }
        });
        s.spawn((txt("", &f.body, 13.0, MUTE), Tag::MapNote));
        s.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), margin: UiRect::top(Val::Px(10.0)), ..default() }).with_children(|l| {
            for (i, (a, b)) in [("FUNDAMENTALS", "movement, aiming, leaning, crouching and the two views"), ("SLICING THE PIE", "going around a corner in an arc"), ("FATAL FUNNEL", "centre door and near corners"),
                ("POSITIVE ID", "tan is a threat, white is a no-shoot"), ("CLOSED DOOR AND T", "open from the side, clear both sides"), ("DEAD SPACE", "what tall furniture hides")].iter().enumerate() {
                l.spawn(Node { column_gap: Val::Px(10.0), align_items: AlignItems::Baseline, ..default() }).with_children(|r| {
                    r.spawn(txt(&format!("{}", i + 1), &f.bold, 19.0, AMBER));
                    r.spawn(txt(a, &f.semi, 19.0, BONE));
                    r.spawn(txt(b, &f.body, 14.0, MUTE));
                });
            }
        });
        s.spawn((txt("A teaching tool for fundamentals. Real technique is learned with a qualified instructor, as a team, in a controlled environment.", &f.body, 13.0, MUTE), Node { max_width: Val::Px(640.0), ..default() }));
    });
    commands.spawn(screen(Tag::Credits)).with_children(|s| {
        s.spawn(txt("POINT OF ENTRY", &f.semi, 14.0, AMBER));
        s.spawn(txt("CREDITS", &f.bold, 54.0, BONE));
        s.spawn((txt(CREDITS, &f.body, 15.5, BONE), Node { max_width: Val::Px(980.0), ..default() }));
        s.spawn(Node::default()).with_children(|b| button(b, &f, "BACK · ESC", Act::Back, true));
    });
    commands.spawn(screen(Tag::Card)).with_children(|s| {
        s.spawn((txt("", &f.semi, 14.0, AMBER), Tag::CardKick));
        s.spawn((txt("", &f.bold, 54.0, BONE), Tag::CardTitle));
        s.spawn(Node { column_gap: Val::Px(40.0), align_items: AlignItems::FlexStart, ..default() }).with_children(|r| {
            r.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(14.0), width: Val::Px(560.0), ..default() }).with_children(|c| {
                c.spawn((txt("", &f.body, 18.0, BONE), Tag::CardLead));
                c.spawn((txt("", &f.body, 15.5, BONE), Tag::CardPoints));
                c.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|b| {
                    button(b, &f, "GOT IT · ENTER", Act::CardGo, true);
                    b.spawn((Node::default(), Tag::DemoBtn)).with_children(|d| button(d, &f, "WATCH DEMO", Act::Demo, false));
                });
            });
            r.spawn((ImageNode::new(diagrams.0[0].clone()), Node { width: Val::Px(520.0), height: Val::Px(309.0), border: UiRect::all(Val::Px(1.0)), ..default() }, BorderColor(Color::srgba(0.9, 0.9, 0.86, 0.16)), BackgroundColor(Color::srgba(0.035, 0.05, 0.055, 0.7)), Tag::CardImg));
        });
    });
    commands.spawn(screen(Tag::Pause)).with_children(|s| {
        s.spawn(txt("PAUSE", &f.semi, 14.0, AMBER));
        s.spawn(txt("SIMULATION PAUSED", &f.bold, 54.0, BONE));
        s.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
            button(r, &f, "RESUME · ESC", Act::Resume, true);
            button(r, &f, "RESTART", Act::Restart, false);
            button(r, &f, "QUIT GAME", Act::Quit, false);
        });
        s.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
            toggle_button(r, &f, Act::Flash, Tag::FlashBtn);
            toggle_button(r, &f, Act::Guide, Tag::GuideBtn);
            toggle_button(r, &f, Act::Box, Tag::BoxBtn);
        });
        s.spawn((txt("W A S D move   ·   Space or right button: aim (Z locks the aim)   ·   Q / E lean   ·   C toggles crouch, Ctrl crouches while held\nShift fast step   ·   R reload   ·   F open door   ·   L or middle mouse button: flashlight   ·   G tactical view   ·   V top-down view\nT ideal route: always / tip / off   ·   O hide the lesson box   ·   H review the lesson and watch the demo", &f.body, 15.0, MUTE), Node { max_width: Val::Px(900.0), ..default() }));
        s.spawn((txt("", &f.semi, 15.0, AMBER), Node { max_width: Val::Px(900.0), ..default() }, Tag::BadgeList));
    });
    commands.spawn(screen(Tag::Debrief)).with_children(|s| {
        s.spawn(txt("COURSE DEBRIEF", &f.semi, 14.0, AMBER));
        s.spawn(Node { column_gap: Val::Px(24.0), align_items: AlignItems::FlexEnd, ..default() }).with_children(|r| {
            r.spawn((txt("A", &f.bold, 92.0, GOOD), Tag::DbGrade));
            r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                c.spawn(txt("HOUSE CLEAR", &f.bold, 40.0, BONE));
                c.spawn((txt("", &f.semi, 22.0, MUTE), Tag::DbScore));
            });
        });
        s.spawn((Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), width: Val::Px(820.0), ..default() }, Tag::DbRows));
        s.spawn(Node { column_gap: Val::Px(10.0), ..default() }).with_children(|r| {
            button(r, &f, "REDO TUTORIAL", Act::Start, true);
            button(r, &f, "MISSION 2", Act::Mission2, false);
            button(r, &f, "FREE RUN", Act::Free, false);
            button(r, &f, "QUIT GAME", Act::Quit, false);
        });
    });
    commands.insert_resource(f);
    commands.insert_resource(diagrams);
}

/// Cliques nos botões e teclas de confirmação/pausa.
pub fn ui_actions(mut game: ResMut<Game>, c: Res<Controls>, q: Query<(&Interaction, &Act), Changed<Interaction>>, mut reset: EventWriter<ResetGame>, mut sfx: EventWriter<Sfx>, mut exit: EventWriter<AppExit>, keys: Res<ButtonInput<KeyCode>>, mut mapreq: ResMut<MapReq>) {
    // durante a demonstração só existe uma ação: sair dela
    if game.demo { if c.pause || c.confirm { game.demo_end = true; } return; }
    let mut act: Option<Act> = None;
    for (i, a) in &q { if *i == Interaction::Pressed { act = Some(*a); } }
    match game.mode {
        Mode::Card if c.confirm => act = Some(Act::CardGo),
        Mode::Menu if c.confirm => act = Some(Act::Start),
        Mode::Play if c.pause => { game.mode = Mode::Pause; game.ui_dirty = true; return; }
        Mode::Pause if c.pause => act = Some(Act::Resume),
        Mode::Credits if c.pause || c.confirm => act = Some(Act::Back),
        Mode::Play if c.help && !game.free => { game.mode = Mode::Card; game.ui_dirty = true; return; }
        _ => {}
    }
    if game.mode == Mode::Menu && keys.just_pressed(KeyCode::Escape) && act.is_none() { exit.write(AppExit::Success); }
    let Some(a) = act else { return };
    sfx.write(Sfx::Ui);
    match a {
        // tutorial e missão 2 são da casa de treino: volta para ela se outro mapa estiver montado
        Act::Start => { if map_index() != 0 { mapreq.0 = Some(0); } reset.write(ResetGame { free: false, mission2: false }); }
        Act::Free => { reset.write(ResetGame { free: true, mission2: false }); }
        Act::CardGo | Act::Resume => { game.mode = Mode::Play; }
        Act::Restart => { reset.write(ResetGame { free: game.free, mission2: game.mission2 }); }
        Act::Mission2 => { if map_index() != 0 { mapreq.0 = Some(0); } reset.write(ResetGame { free: true, mission2: true }); }
        Act::Map(i) => { mapreq.0 = Some(i as usize); }
        Act::Quit => { exit.write(AppExit::Success); }
        Act::Demo => { game.demo_req = true; }
        Act::Flash => { game.flash = !game.flash; game.flag_flash = true; }
        Act::Guide => { game.next_guide(); }
        Act::Box => { game.hide_box = !game.hide_box; }
        Act::Credits => { game.mode = Mode::Credits; }
        Act::Back => { game.mode = Mode::Menu; }
    }
    game.ui_dirty = true;
}

#[allow(clippy::too_many_arguments)]
pub fn do_reset(mut ev: EventReader<ResetGame>, mut game: ResMut<Game>, mut p: ResMut<Player>, mut level: ResMut<Level>, mut targets: ResMut<Targets>, mut tac: ResMut<Tac>,
    mut commands: Commands, junk: Query<Entity, Or<(With<Decal>, With<Casing>, With<Puff>)>>, mut fx: ResMut<Fx>) {
    let Some(e) = ev.read().last() else { return };
    let (free, m2) = (e.free, e.mission2);
    let (sens, guide, hide_box) = (game.sens, game.guide, game.hide_box);
    *game = Game { sens, guide, hide_box, ..default() };
    *p = Player::default();
    level.reset();
    // a posição de ameaças e reféns é sorteada a cada partida, em todos os modos (cada alvo continua no seu cômodo)
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7) | 1;
    let mut rnd = move || { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed };
    let mut pool: Vec<usize> = (0..slots().len()).collect();
    for i in (1..pool.len()).rev() { let j = (rnd() as usize) % (i + 1); pool.swap(i, j); }
    for (i, t) in targets.list.iter_mut().enumerate() {
        t.def = target_defs()[i];
        if !t.def.passive {
            if let Some(k) = pool.iter().position(|&s| slots()[s].0 == t.def.zone) {
                let s = slots()[pool.remove(k)];
                t.def.x = s.1; t.def.z = s.2; t.def.fx = s.3; t.def.fz = s.4;
            }
        }
        crate::combat::place_target(&level, t);
        t.alive = true; t.fall = 0.0; t.fall_v = 0.0; t.sw = 0.0; t.swv = 0.0; t.landed = false; t.hits = 0; t.seen = false; t.vis = false; t.react = 0.0; t.contact = false; t.first_facing = None;
        // na missão 2 o operador começa dentro da Sala C: os alvos dela ficam fora
        t.off = m2 && t.def.zone == "C";
        if t.off { t.alive = false; t.fall = 1.0; t.landed = true; }
        let ry = (t.def.fx - t.def.x).atan2(t.def.fz - t.def.z);
        commands.entity(t.root).insert((Transform::from_xyz(t.def.x, 0.0, t.def.z).with_rotation(Quat::from_rotation_y(ry)), if t.off { Visibility::Hidden } else { Visibility::Inherited }));
        for m in t.marks.drain(..) { commands.entity(m).despawn(); }
    }
    for j in &junk { commands.entity(j).despawn(); }
    fx.decals.clear();
    tac.reset();
    tac.corner_zone = None;
    if let Some(z) = std::env::var("CQB_ZOOM").ok().and_then(|v| v.parse::<f32>().ok()) { game.bird_zoom = z; }
    if free {
        game.free = true; game.mission2 = m2; game.mode = Mode::Play;
        if m2 {
            p.pos = Vec2::new(19.5, 1.7); p.yaw = std::f32::consts::PI; p.zone = 6; p.eye = Vec3::new(19.5, EYE_STAND, 1.7);
            game.toast("Mission 2: you start in Room C. Clear the house back to the Staging area. Targets in random positions, no hints.", 1, 6.0);
        } else {
            let st = map().start;
            if map_index() != 0 { p.pos = Vec2::new(st.0, st.1); p.yaw = st.2; p.eye = Vec3::new(st.0, EYE_STAND, st.1); p.zone = zone_at(st.0, st.1).unwrap_or(0); }
            let msg = format!("Free run · {}: targets in random positions, no instructions. Clear every room.", map().name);
            game.toast(&msg, 1, 4.5);
        }
    } else { start_lesson(&mut game, &mut tac, 0); }
}

fn show(node: &mut Node, on: bool) { let d = if on { Display::Flex } else { Display::None }; if node.display != d { node.display = d; } }
fn set(text: &mut Text, s: &str) { if text.0 != s { text.0 = s.to_string(); } }

#[allow(clippy::too_many_arguments)]
pub fn update_ui(
    time: Res<Time>, mut game: ResMut<Game>, p: Res<Player>, tac: Res<Tac>, targets: Res<Targets>, level: Res<Level>, (fonts, diagrams): (Res<UiFonts>, Res<Diagrams>),
    mut commands: Commands, mut ui_scale: ResMut<UiScale>, windows: Query<&Window, With<PrimaryWindow>>,
    mut texts: Query<(&Tag, &mut Text, &mut TextColor)>, mut nodes: Query<(Entity, &Tag, &mut Node)>, mut bgs: Query<(&Tag, &mut BackgroundColor)>, mut imgs: Query<(&Tag, &mut ImageNode)>,
    cams: Query<(&Camera, &GlobalTransform), With<Camera3d>>, mut live_t: Local<f32>,
) {
    let dt = time.delta_secs().min(0.05);
    let Ok(win) = windows.single() else { return };
    let scale = (win.height() / 810.0).max(0.4);
    if (ui_scale.0 - scale).abs() > 0.001 { ui_scale.0 = scale; }
    let (uw, uh) = (win.width() / scale, win.height() / scale);
    let bird = game.view == View::Bird || game.mode == Mode::Replay;
    let play = game.mode == Mode::Play;
    if game.hit_t > 0.0 { game.hit_t -= dt; }
    if game.dmg_t > 0.0 { game.dmg_t -= dt; }
    let mut toast_on = false;
    let mut toast_kind = 0;
    let mut toast_msg = String::new();
    if let Some((m, k, ttl)) = game.toast.as_mut() { *ttl -= dt; if *ttl > 0.0 { toast_on = true; toast_kind = *k; toast_msg = m.clone(); } }
    if !toast_on { game.toast = None; }
    *live_t -= dt;
    let rebuild = game.ui_dirty || *live_t <= 0.0;
    if rebuild { *live_t = 0.25; }
    let ctx = Ctx { p: &p, game: &game, tac: &tac, targets: &targets, level: &level };
    let active_cam = cams.iter().find(|(c, _)| c.is_active && c.order == 0);
    let project = |pos: Vec3| -> Option<Vec2> { active_cam.and_then(|(c, g)| c.world_to_viewport(g, pos).ok()).map(|v| v / scale) };
    let wp = if play && !game.demo { waypoint(&ctx) } else { None };
    // rótulos das aberturas em foco: graus já fatiados de 180
    let mut pies: Vec<(usize, Vec2)> = Vec::new();
    if game.tac && play {
        for (i, o) in openings().iter().enumerate() {
            let near = p.eye2().distance(o.mid()) < 8.0 && (p.eye2() - o.mid()).dot(o.n) < 0.0;
            if tac.pct[zone_index(o.zone)] < 0.95 && near && o.door.map_or(true, |d| level.doors[d].open > 0.5) { pies.push((i, o.mid() + o.n * 0.9)); }
        }
        pies.truncate(if bird { 4 } else { 2 });
    }

    for (e, tag, mut node) in nodes.iter_mut() {
        match *tag {
            Tag::Hud => show(&mut node, !matches!(game.mode, Mode::Menu | Mode::Debrief | Mode::Credits)),
            Tag::Credits => show(&mut node, game.mode == Mode::Credits),
            Tag::Menu => show(&mut node, game.mode == Mode::Menu),
            Tag::Card => show(&mut node, game.mode == Mode::Card),
            Tag::Pause => show(&mut node, game.mode == Mode::Pause),
            Tag::Debrief => show(&mut node, game.mode == Mode::Debrief),
            Tag::TacPanel => show(&mut node, game.tac),
            Tag::MiniFrame | Tag::Legend => show(&mut node, game.tac && !bird),
            Tag::Prompt => show(&mut node, play && game.near_door.is_some()),
            Tag::Replay => show(&mut node, game.mode == Mode::Replay),
            Tag::ReplayCursor if game.mode == Mode::Replay => { node.margin = UiRect::left(Val::Px(628.0 * (game.replay_t / REPLAY_SECS).clamp(0.0, 1.0))); }
            Tag::ReplayBars if game.ui_dirty && game.mode == Mode::Replay => {
                commands.entity(e).despawn_related::<Children>();
                let n = game.track.len();
                let bars = 106usize;
                commands.entity(e).with_children(|b| {
                    for k in 0..bars {
                        let h = game.track.get(k * n / bars).map_or(0.0, |t| t.3);
                        let col = if h > 45.0 { BAD } else if h > 15.0 { AMBER } else { GOOD };
                        b.spawn((Node { width: Val::Px(5.0), height: Val::Px(3.0 + (h / 180.0).clamp(0.0, 1.0) * 60.0), ..default() }, BackgroundColor(col)));
                    }
                });
            }
            Tag::Toast => show(&mut node, toast_on),
            Tag::Coach => show(&mut node, play && game.coach.is_some()),
            Tag::DemoBar => show(&mut node, game.demo),
            Tag::LessonBox => show(&mut node, !game.hide_box),
            Tag::Odds => show(&mut node, play && !game.demo && (game.free || game.lesson >= 1)),
            Tag::OddsBar => { node.width = Val::Percent(game.odds.clamp(0.0, 100.0)); }
            Tag::DemoBtn => show(&mut node, !ideal(game.lesson).is_empty()),
            Tag::Hit => show(&mut node, game.hit_t > 0.0),
            Tag::Cross(k) => {
                let on = play && !bird && p.g_ads < 0.5 && p.g_block < 0.5 && p.g_lower < 0.5;
                show(&mut node, on);
                if on {
                    let gap = 7.0 + p.speed * 5.0 + p.g_kick * 14.0;
                    let (cx, cy) = (uw / 2.0, uh / 2.0);
                    let (x, y, w, h) = match k { 0 => (cx - 1.0, cy - gap - 9.0, 2.0, 9.0), 1 => (cx - 1.0, cy + gap, 2.0, 9.0), 2 => (cx - gap - 9.0, cy - 1.0, 9.0, 2.0), 3 => (cx + gap, cy - 1.0, 9.0, 2.0), _ => (cx - 1.0, cy - 1.0, 2.0, 2.0) };
                    node.left = Val::Px(x); node.top = Val::Px(y); node.width = Val::Px(w); node.height = Val::Px(h);
                }
            }
            Tag::Wp => {
                match wp {
                    Some((pos, _)) => {
                        let v = project(Vec3::new(pos.x, if bird { 0.0 } else { 1.3 }, pos.y)).unwrap_or(Vec2::new(uw / 2.0 - 40.0, uh - 90.0));
                        show(&mut node, true);
                        node.left = Val::Px(v.x.clamp(30.0, uw - 140.0)); node.top = Val::Px(v.y.clamp(70.0, uh - 80.0));
                    }
                    None => show(&mut node, false),
                }
            }
            Tag::Label(k) => {
                let k = k as usize;
                let mut pos: Option<Vec2> = None;
                if k < pies.len() {
                    let q = pies[k].1;
                    pos = project(Vec3::new(q.x, if bird { 0.0 } else { 1.75 }, q.y));
                } else if k == 5 && play && !game.free && game.route_visible() {
                    let pts = ideal_points(game.lesson);
                    if let Some((q, _)) = pts.get(game.guide_i) { pos = project(Vec3::new(q.x, if bird { 0.0 } else { 0.3 }, q.y)); }
                } else if k == 6 && bird && game.tac && play {
                    let f = p.fwd();
                    pos = project(Vec3::new(p.eye.x + f.x * 1.6, 0.0, p.eye.z + f.y * 1.6));
                }
                match pos { Some(v) if v.x > 0.0 && v.y > 0.0 && v.x < uw && v.y < uh => { show(&mut node, true); node.left = Val::Px(v.x + 6.0); node.top = Val::Px(v.y - 12.0); } _ => show(&mut node, false) }
            }
            Tag::ObjList if rebuild && !matches!(game.mode, Mode::Menu | Mode::Debrief) => {
                commands.entity(e).despawn_related::<Children>();
                commands.entity(e).with_children(|l| {
                    let mut line = |l: &mut ChildSpawnerCommands, state: u8, text: &str, live: Option<String>| {
                        let col = match state { 1 => MUTE, 2 => BAD, _ => BONE };
                        l.spawn(Node { column_gap: Val::Px(8.0), align_items: AlignItems::FlexStart, ..default() }).with_children(|r| {
                            r.spawn((Node { width: Val::Px(13.0), height: Val::Px(13.0), margin: UiRect::top(Val::Px(3.0)), border: UiRect::all(Val::Px(1.5)), flex_shrink: 0.0, ..default() },
                                BorderColor(match state { 1 => GOOD, 2 => BAD, _ => MUTE }), BackgroundColor(match state { 1 => GOOD, 2 => BAD, _ => Color::NONE })));
                            r.spawn((txt(text, &fonts.body, 14.5, col), Node { width: Val::Px(262.0), flex_shrink: 0.0, ..default() }));
                            if let Some(v) = live { r.spawn((txt(&v, &fonts.semi, 15.0, CYAN), Node { margin: UiRect::left(Val::Auto), ..default() })); }
                        });
                    };
                    if game.free {
                        let th: Vec<_> = targets.list.iter().filter(|t| t.def.kind == TKind::Threat && !t.off).collect();
                        let dn = th.iter().filter(|t| !t.alive).count();
                        line(l, if dn == th.len() { 1 } else { 0 }, "Threats neutralized", Some(format!("{}/{}", dn, th.len())));
                    } else {
                        for (i, o) in LESSONS[game.lesson].objs.iter().enumerate() {
                            let st = game.obj.get(i).copied().unwrap_or(0);
                            line(l, st, o, live(game.lesson, i, &ctx));
                        }
                    }
                });
            }
            Tag::DbRows if game.ui_dirty && game.mode == Mode::Debrief => {
                let s = &game.stats;
                let th: Vec<_> = targets.list.iter().filter(|t| t.def.kind == TKind::Threat && !t.def.passive && !t.off).collect();
                let dn = th.iter().filter(|t| !t.alive).count();
                let first = th.iter().filter(|t| t.first_facing == Some(true)).count();
                let contact = th.iter().filter(|t| t.first_facing.is_some()).count();
                let acc = if s.shots > 0 { (s.hits as f32 / s.shots as f32 * 100.0).round() as i32 } else { 0 };
                let en = |k: usize| game.entry_pie[k].map_or("—".to_string(), |v| format!("{}°", v.round() as i32));
                let dw = |k: usize| format!("{:.1} s", game.dwell[k]);
                let (nk, kavg) = (game.kills.len().max(1) as f32, |f: fn(&Kill) -> f32| game.kills.iter().map(f).sum::<f32>() / game.kills.len().max(1) as f32);
                let _ = nk;
                let route_txt = match game.route_score() { Some(r) => format!("{}%  ·  {} cm off the line on average", r.round() as i32, (game.route_sum / game.route_t * 100.0).round() as i32), None => "—".to_string() };
                let angle_txt = if game.kills.is_empty() { "—".to_string() } else { format!("average {}°  ·  best {}°", kavg(|k| k.angle).round() as i32, game.kills.iter().map(|k| k.angle).fold(99.0, f32::min).round() as i32) };
                let expo_txt = if game.kills.is_empty() { "—".to_string() } else { format!("average {} cm  ·  best {} cm", kavg(|k| k.expo_cm).round() as i32, game.kills.iter().map(|k| k.expo_cm).fold(99.0, f32::min).round() as i32) };
                let badge_names: Vec<&str> = BADGES.iter().enumerate().filter(|(i, _)| game.badges & (1 << i) != 0).map(|(_, b)| b.0).collect();
                let badge_note = badge_names.join("  ·  ");
                let grade3 = |v: f32, good: f32, ok: f32| if v <= good { GOOD } else if v <= ok { AMBER } else { BAD };
                let rows: Vec<(String, String, Color, &str)> = vec![
                    ("Ideal route (40% of the score)".into(), route_txt, game.route_score().map_or(BONE, |r| if r >= 80.0 { GOOD } else if r >= 55.0 { AMBER } else { BAD }), ""),
                    ("Slice angle at each kill (smaller is better)".into(), angle_txt, if game.kills.is_empty() { BONE } else { grade3(kavg(|k| k.angle), 12.0, 24.0) }, "Degrees between the target and the edge hiding it when it dropped. Engage as soon as the slice shows it."),
                    ("Body exposed at each kill (smaller is better)".into(), expo_txt, if game.kills.is_empty() { BONE } else { grade3(kavg(|k| k.expo_cm), 15.0, 32.0) }, "Width of your torso the target could see. Angle, exposure and time are 60% of the score."),
                    ("Time in a threat's line of fire (smaller is better)".into(), if game.kills.is_empty() { "—".to_string() } else { format!("average {:.1} s before each kill  ·  {:.1} s in total", kavg(|k| k.dt), game.expo_total) }, if game.kills.is_empty() { BONE } else { grade3(kavg(|k| k.dt), 0.8, 1.4) }, ""),
                    ("Badges earned".into(), format!("{} of {}", badge_names.len(), BADGES.len()), if badge_names.is_empty() { BONE } else { AMBER }, &badge_note),
                    ("Threats neutralized".into(), format!("{} of {}", dn, th.len()), if dn == th.len() { GOOD } else { BAD }, ""),
                    ("White targets (no-shoot) hit".into(), s.civ_hit.to_string(), if s.civ_hit > 0 { BAD } else { GOOD }, if s.civ_hit > 0 { "The most serious error on the course. Confirm the target colour before every shot." } else { "" }),
                    ("Times hit (simulated)".into(), s.hits_taken.to_string(), if s.hits_taken > 0 { BAD } else { GOOD }, if s.hits_taken > 0 { "A threat had time to react with a clear line of fire to you." } else { "" }),
                    ("Threats you saw first".into(), format!("{} of {}", first, contact), if first == contact { GOOD } else { AMBER }, if first < contact { "For the others, you appeared without looking at them." } else { "" }),
                    ("Fan opened before entering (of 180°)".into(), doorways().iter().enumerate().map(|(k, w)| format!("{} {}", w.zone, en(k))).collect::<Vec<_>>().join(" · "), BONE, ""),
                    ("Technique warnings during the run".into(), { let n: u32 = game.warns.iter().sum(); if n == 0 { "none".to_string() } else { WARN_NAMES.iter().zip(game.warns.iter()).filter(|(_, c)| **c > 0).map(|(k, c)| format!("{k} {c}")).collect::<Vec<_>>().join(" · ") } },
                        if game.warns.iter().sum::<u32>() == 0 { GOOD } else { AMBER }, ""),
                    ("Time in the fatal funnel".into(), doorways().iter().enumerate().map(|(k, _)| dw(k)).collect::<Vec<_>>().join(" · "), BONE, ""),
                    ("Shots, accuracy and hits by zone".into(), format!("{} shots · {}% on target · A {} · C {} · D {}", s.shots, acc, s.zone_a, s.zone_c, s.zone_d), BONE, ""),
                    ("Total time".into(), format!("{} min {:02} s", (game.time / 60.0) as i32, (game.time % 60.0) as i32), BONE, ""),
                ];
                commands.entity(e).despawn_related::<Children>();
                commands.entity(e).with_children(|l| {
                    for (k, v, col, note) in rows {
                        l.spawn(Node { flex_direction: FlexDirection::Column, border: UiRect::top(Val::Px(1.0)), padding: UiRect::top(Val::Px(2.0)), ..default() }).insert(BorderColor(Color::srgba(0.9, 0.9, 0.86, 0.16))).with_children(|r| {
                            r.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..default() }).with_children(|x| {
                                x.spawn(txt(&k, &fonts.body, 14.0, MUTE));
                                x.spawn(txt(&v, &fonts.bold, 17.0, col));
                            });
                            if !note.is_empty() { r.spawn(txt(note, &fonts.body, 11.5, MUTE)); }
                        });
                    }
                });
            }
            _ => {}
        }
    }
    for (tag, mut bg) in bgs.iter_mut() {
        if *tag == Tag::Dmg { bg.0 = Color::srgba(0.75, 0.06, 0.04, (game.dmg_t / 1.3).clamp(0.0, 1.0) * 0.38); }
        if *tag == Tag::OddsBar { bg.0 = if game.odds >= 75.0 { GOOD } else if game.odds >= 50.0 { AMBER } else { BAD }; }
    }
    let lesson = &LESSONS[game.lesson.min(5)];
    for (tag, mut im) in imgs.iter_mut() {
        if *tag == Tag::CardImg && game.mode == Mode::Card { let h = &diagrams.0[game.lesson.min(5)]; if im.image != *h { im.image = h.clone(); } }
    }
    let score_grade = || {
        let s = &game.stats;
        let th: Vec<_> = targets.list.iter().filter(|t| t.def.kind == TKind::Threat && !t.def.passive && !t.off).collect();
        let dn = th.iter().filter(|t| !t.alive).count();
        // nota: traçado (40%) + ângulo e exposição nas derrubadas (60%), menos as penalidades
        let sc = (game.base_score().round() as i32 - s.civ_hit as i32 * 20 - s.hits_taken as i32 * 10 - s.failed as i32 * 5 - (th.len() - dn) as i32 * 8).clamp(0, 100);
        (sc, if sc >= 90 { ("A", GOOD) } else if sc >= 75 { ("B", CYAN) } else if sc >= 60 { ("C", AMBER) } else { ("D", BAD) })
    };
    for (tag, mut t, mut col) in texts.iter_mut() {
        match *tag {
            Tag::LessonKick => set(&mut t, &if game.mission2 { "MISSION 2 · REVERSE ROUTE".to_string() } else if game.free { format!("FREE RUN · {}", map().name.to_uppercase()) } else { lesson.kicker.to_string() }),
            Tag::MapBtn(i) => { col.0 = if i as usize == map_index() { AMBER } else { BONE }; }
            Tag::MapNote if game.mode == Mode::Menu => set(&mut t, &format!("Selected: {} — {}. Enemy and hostage positions are random on every run.", map().name, map().blurb)),
            Tag::LessonTitle => set(&mut t, if game.mission2 { "FROM ROOM C TO STAGING" } else if game.free { "CLEAR EVERY ROOM" } else { lesson.title }),
            Tag::ReplayTitle => set(&mut t, &format!("ENTRY REPLAY · {}", lesson.title.to_uppercase())),
            Tag::CardKick => set(&mut t, lesson.kicker),
            Tag::CardTitle => set(&mut t, &lesson.title.to_uppercase()),
            Tag::CardLead => set(&mut t, lesson.lead),
            Tag::CardPoints => { let s: Vec<String> = lesson.points.iter().map(|x| format!("–  {x}")).collect(); set(&mut t, &s.join("\n\n")); }
            Tag::AmmoMag => { set(&mut t, &if p.reloading { "··".to_string() } else { p.mag.to_string() }); col.0 = if p.mag <= 2 && !p.reloading { BAD } else { BONE }; }
            Tag::AmmoRes => set(&mut t, &format!("/ {}", p.reserve)),
            Tag::AmmoMode => set(&mut t, &format!("PISTOL .45 · {}", if p.reloading { "RELOADING" } else if p.mag == 0 { "EMPTY · R" } else { "SEMI-AUTO" })),
            Tag::Stance => {
                let mut s = if p.crouch > 0.5 { "CROUCHED".to_string() } else { "STANDING".to_string() };
                if p.fast && p.speed > 2.4 { s = "FAST STEP · GUN LOW".into(); } else if p.ads { s += " · AIMING"; }
                if p.lean.abs() > 0.3 { s += if p.lean > 0.0 { " · LEANING RIGHT" } else { " · LEANING LEFT" }; }
                if p.lean_blocked { s += " · LEAN BLOCKED BY THE WALL"; }
                if p.g_block > 0.5 && !bird { s = "GUN PULLED BACK · STEP AWAY FROM THE WALL".into(); }
                set(&mut t, &s);
            }
            Tag::ViewTag => set(&mut t, &format!("{}   ·   [SPACE] AIM   ·   [L] FLASHLIGHT {}{}", if bird { "TOP-DOWN VIEW" } else { "FIRST PERSON" }, if game.flash { "ON" } else { "OFF" },
                if game.free { String::new() } else { format!("   ·   [T] ROUTE {}   ·   [O] BOX", match game.guide { 1 => "ON", 2 => "TIP", _ => "OFF" }) })),
            Tag::FlashBtn => set(&mut t, if game.flash { "FLASHLIGHT: ON · L" } else { "FLASHLIGHT: OFF · L" }),
            Tag::GuideBtn => set(&mut t, match game.guide { 1 => "IDEAL ROUTE: ALWAYS · T", 2 => "IDEAL ROUTE: TIP (SHOWS, THEN FADES) · T", _ => "IDEAL ROUTE: OFF · T" }),
            Tag::BoxBtn => set(&mut t, if game.hide_box { "LESSON BOX: HIDDEN · O" } else { "LESSON BOX: SHOWN · O" }),
            Tag::OddsVal => { set(&mut t, &format!("{}%", game.odds.round() as i32)); col.0 = if game.odds >= 75.0 { GOOD } else if game.odds >= 50.0 { AMBER } else { BAD }; }
            Tag::OddsNote => set(&mut t, &if game.free { format!("angle, exposure and time {}%", game.odds_angle.round() as i32) } else { format!("route {}%  ·  angle, exposure, time {}%", game.odds_route.round() as i32, game.odds_angle.round() as i32) }),
            Tag::BadgeList if game.mode == Mode::Pause => {
                let names: Vec<&str> = BADGES.iter().enumerate().filter(|(i, _)| game.badges & (1 << i) != 0).map(|(_, b)| b.0).collect();
                set(&mut t, &format!("BADGES {} / {}{}", names.len(), BADGES.len(), if names.is_empty() { String::new() } else { format!(":  {}", names.join("  ·  ")) }));
            }
            Tag::CoachText => if let Some((m, _, _)) = &game.coach { set(&mut t, m); },
            Tag::CoachKick => if let Some((_, k, _)) = &game.coach { set(&mut t, match *k { 1 => "GOOD", 3 => "STEP", 4 => "SCORE", _ => "TECHNIQUE" }); col.0 = match *k { 1 => GOOD, 3 => CYAN, 4 => BONE, _ => AMBER }; },
            Tag::ToastText => { set(&mut t, &toast_msg); col.0 = match toast_kind { 1 => GOOD, 2 => Color::srgb(1.0, 0.72, 0.68), 3 => AMBER, _ => BONE }; }
            Tag::WpText => if let Some((pos, label)) = wp { set(&mut t, &format!("{}  {} m", label.to_uppercase(), pos.distance(p.pos).round() as i32)); },
            Tag::TacFov => set(&mut t, &format!("{}°", tac.hfov.to_degrees().round() as i32)),
            Tag::TacFovNote => set(&mut t, if p.ads { "through the sights: narrow sector" } else { "at the ready: where you can identify a target" }),
            Tag::TacCorner => { match tac.pie_active {
                Some(i) => { let (d, now) = (tac.pie_deg(i), tac.pie_now[i].map_or(0.0, |(lo, hi)| hi - lo)); set(&mut t, &format!("{}° / {}°", now.round() as i32, d.round() as i32)); col.0 = if d >= 140.0 { GOOD } else if d >= 90.0 { AMBER } else { BAD }; }
                None => { set(&mut t, "—"); col.0 = MUTE; } } }
            Tag::TacCornerNote => { match tac.pie_active {
                Some(i) => {
                    let tip = if p.lean_blocked { " · lean blocked by the wall".to_string() } else { match tac.lean_hint { Some((side, gain)) => format!(" · lean {} for +{}°", if side < 0.0 { "Q" } else { "E" }, gain.round() as i32), None => String::new() } };
                    set(&mut t, &format!("{}{}", openings()[i].name, tip));
                }
                None => set(&mut t, "no opening in focus") } }
            Tag::TacBody => { match tac.primary { Some(_) => { set(&mut t, &format!("{} cm", tac.body_cm.round() as i32)); col.0 = if tac.body_cm > 30.0 { BAD } else if tac.body_cm > 15.0 { AMBER } else { GOOD }; } None => { set(&mut t, "—"); col.0 = MUTE; } } }
            Tag::TacZoneName => set(&mut t, &format!("{} · AREA SEEN", zones()[p.zone].name.to_uppercase())),
            Tag::TacZone => set(&mut t, &format!("{}%", (tac.pct[p.zone] * 100.0).round() as i32)),
            Tag::TacExpo => { set(&mut t, &format!("{}°", tac.expo_deg.round() as i32)); col.0 = if tac.expo_deg >= 45.0 { BAD } else { BONE }; }
            Tag::Label(k) => {
                let k = k as usize;
                if k < pies.len() {
                    let (d, now) = (tac.pie_deg(pies[k].0), tac.pie_now[pies[k].0].map_or(0.0, |(lo, hi)| hi - lo));
                    set(&mut t, &format!("{}° now · {}° of 180°", now.round() as i32, d.round() as i32));
                    col.0 = if d >= 140.0 { GOOD } else if d >= 90.0 { AMBER } else { Color::srgb(1.0, 0.5, 0.42) };
                } else if k == 5 {
                    if let Some((q, _)) = ideal_points(game.lesson).get(game.guide_i) { set(&mut t, &format!("{:.1} m", q.distance(p.pos))); col.0 = AMBER; }
                } else if k == 6 { set(&mut t, &format!("{}°", tac.hfov.to_degrees().round() as i32)); col.0 = CYAN; }
            }
            Tag::DbGrade if game.mode == Mode::Debrief => { let (_, (g, c)) = score_grade(); set(&mut t, g); col.0 = c; }
            Tag::DbScore if game.mode == Mode::Debrief => { let (sc, _) = score_grade(); set(&mut t, &format!("{} POINTS", sc)); }
            _ => {}
        }
    }
    game.ui_dirty = false;
}
