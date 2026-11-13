//! Mapa "Family home": casa térrea (defesa da casa), em OSB como uma kill house: quartos ao fundo (z 0..6), corredor (z 6..8), sala e área gourmet (z 7..13), quintal (z 13..17,2) e rua (z 17,2..18,4). Os invasores nascem na rua, atrás do muro, e entram pelos portões.
use crate::map::*;
use crate::map::PropKind::*;
#[allow(unused_imports)]
use crate::map::TKind::{Civil, Threat};

pub const MAP: MapDef = MapDef {
    name: "Family home", blurb: "single floor: bedrooms at the back, a corridor, living area, yard and street",
    zones: &[
        zn("Hl", "Bedroom hall", 0.0, 0.0, 9.3, 1.4), zn("S1", "Suite 1", 0.0, 1.4, 4.7, 6.0), zn("S2", "Suite 2", 4.7, 1.4, 9.3, 6.0), zn("Ms", "Master suite", 9.3, 0.0, 14.0, 6.0),
        // planta térrea: um corredor largo liga os quartos (ao fundo) à sala; o braço leste dele dá na suíte principal e na área gourmet
        zn("Cr", "Corridor", 0.0, 6.0, 10.0, 8.0), zn("Ce", "Side corridor", 10.0, 6.0, 14.0, 7.0),
        zn("Lv", "Living room", 0.0, 8.0, 10.0, 13.0), zn("Gm", "Gourmet area", 10.0, 7.0, 14.0, 13.0), zn("Yd", "Yard", 0.0, 13.0, 14.0, 17.2),
        zn("Rd", "Street", 0.0, 17.2, 14.0, 18.4),
    ],
    doorways: &[dh("S1", 3.6, 4.5, 1.4, 1.0, 0), dh("S2", 6.6, 7.5, 1.4, 1.0, 1), dv("Ms", 9.3, 0.2, 1.2, 1.0, 2), dh("Lv", 3.3, 4.4, 13.0, -1.0, 3), dh("Gm", 10.6, 13.4, 13.0, -1.0, 4),
        dh("Yd", 2.6, 5.4, 17.2, -1.0, 5)],
    openings: &[
        op("Suite 1 door", "S1", 3.6, 1.4, 4.5, 1.4, 0.0, 1.0, None), op("Suite 2 door", "S2", 6.6, 1.4, 7.5, 1.4, 0.0, 1.0, None), op("Master suite door", "Ms", 9.3, 0.2, 9.3, 1.2, 1.0, 0.0, None),
        op("Front door", "Lv", 3.3, 13.0, 4.4, 13.0, 0.0, -1.0, None), op("Gourmet opening", "Gm", 10.6, 13.0, 13.4, 13.0, 0.0, -1.0, None), op("Garage gate", "Yd", 2.6, 17.2, 5.4, 17.2, 0.0, -1.0, None),
    ],
    lights: &[lt(1.9, 0.7, WARM, 0.9, false), lt(4.3, 0.7, WARM, 0.7, false), lt(7.6, 0.7, WARM, 0.7, false), lt(2.6, 3.7, WARM, 0.85, false), lt(6.2, 3.7, WARM, 0.85, false), lt(11.0, 4.1, WARM, 0.9, false), lt(11.6, 1.1, WARM, 0.6, false),
        lt(2.6, 7.0, NEUTRAL, 0.8, false), lt(7.4, 7.0, NEUTRAL, 0.8, false), lt(12.0, 6.5, NEUTRAL, 0.5, false),
        lt(3.0, 10.5, NEUTRAL, 0.95, false), lt(7.4, 10.5, NEUTRAL, 0.95, false), lt(12.0, 10.0, NEUTRAL, 0.9, false), lt(3.6, 15.1, COOL, 0.6, false), lt(10.6, 15.1, COOL, 0.6, false)],
    // dez invasores de reserva: esperam na rua, fora da vista de quem está na casa, até a onda chamá-los
    targets: &[
        tg("I1", Threat, 0, 0.5, 17.8, 4.0, 17.2, "Rd", false), tg("I2", Threat, 1, 10.4, 17.8, 8.9, 17.2, "Rd", false), tg("I3", Threat, 0, 1.2, 17.8, 4.0, 17.2, "Rd", false),
        tg("I4", Threat, 1, 11.3, 17.8, 8.9, 17.2, "Rd", false), tg("I5", Threat, 0, 1.9, 17.8, 4.0, 17.2, "Rd", false), tg("I6", Threat, 1, 12.2, 17.8, 8.9, 17.2, "Rd", false),
        tg("I7", Threat, 0, 6.1, 17.8, 4.0, 17.2, "Rd", false), tg("I8", Threat, 1, 13.1, 17.8, 8.9, 17.2, "Rd", false), tg("I9", Threat, 0, 6.8, 17.8, 4.0, 17.2, "Rd", false),
        tg("I10", Threat, 1, 7.5, 17.8, 8.9, 17.2, "Rd", false),
    ],
    slots: &[
        ("Rd", 0.5, 17.8, 4.0, 17.2), ("Rd", 10.4, 17.8, 8.9, 17.2), ("Rd", 1.2, 17.8, 4.0, 17.2), ("Rd", 11.3, 17.8, 8.9, 17.2), ("Rd", 1.9, 17.8, 4.0, 17.2),
        ("Rd", 12.2, 17.8, 8.9, 17.2), ("Rd", 6.1, 17.8, 4.0, 17.2), ("Rd", 13.1, 17.8, 8.9, 17.2), ("Rd", 6.8, 17.8, 4.0, 17.2), ("Rd", 7.5, 17.8, 8.9, 17.2),
    ],
    walls_h: &[(0.0, 0.0, 14.0, &[]), (6.0, 0.0, 14.0, &[(1.5, 2.4), (3.0, 4.2), (5.0, 6.2), (6.8, 7.6), (10.4, 11.3)]), (1.4, 0.0, 9.3, &[(1.6, 2.8), (3.6, 4.5), (5.0, 6.2), (6.6, 7.5)]), (2.2, 10.7, 14.0, &[]), (3.7, 7.7, 9.3, &[]), (3.6, 0.0, 1.3, &[]),
        (7.0, 10.0, 14.0, &[(11.6, 12.5)]), (8.0, 0.0, 10.0, &[(2.0, 3.1), (7.0, 8.1)]), (13.0, 0.0, 14.0, &[(3.3, 4.4), (10.6, 13.4)]), (17.2, 0.0, 14.0, &[(2.6, 5.4), (8.3, 9.5)]), (18.4, 0.0, 14.0, &[])],
    walls_v: &[(0.0, 0.0, 18.4, &[]), (14.0, 0.0, 18.4, &[]), (4.7, 1.4, 6.0, &[(3.0, 4.2)]), (7.7, 1.4, 6.0, &[(2.2, 3.1), (4.4, 5.3)]), (9.3, 0.0, 6.0, &[(0.2, 1.2), (4.3, 5.5)]),
        (10.7, 0.0, 2.2, &[(0.5, 1.5)]), (12.7, 2.2, 6.0, &[(2.9, 3.8)]), (1.3, 3.6, 6.0, &[(4.4, 5.3)]),
        (10.0, 7.0, 13.0, &[(8.6, 9.7)]), (7.5, 13.0, 17.2, &[]),
        // a rua é dividida em dois lados: quem chega pelo lado da piscina só tem o portão lateral, e a casa é atacada por duas frentes
        (7.95, 17.2, 18.4, &[])],
    props: &[
        // kill house vazia: sem camas nem móveis. Peitoris das janelas dos quartos (para os dois corredores e de um
        // quarto para o outro): por elas os invasores veem, atiram e jogam granada, e ninguém passa
        pr(Sill, 1.6, 1.3, 2.8, 1.5, 0.95), pr(Sill, 5.0, 1.3, 6.2, 1.5, 0.95), pr(Sill, 3.0, 5.9, 4.2, 6.1, 0.95), pr(Sill, 5.0, 5.9, 6.2, 6.1, 0.95),
        pr(Sill, 9.2, 4.3, 9.4, 5.5, 0.95), pr(Sill, 4.6, 3.0, 4.8, 4.2, 0.95),
        pr(Pool, 10.7, 14.0, 13.3, 16.4, 0.12),
    ],
    doors: &[],
    corners: &[],
    start: (12.1, 5.0, 1.5708), pick: false, defense: true,
    // planta térrea: sem escada, sem passagem entre andares, sem vãos escuros
    portals: &[], ramps: &[], pit: None, darks: &[],
};
