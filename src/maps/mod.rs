//! Os mapas do jogo, um arquivo por mapa. A ordem aqui é o índice do mapa (cenários e ranking usam esse número).
mod training;
mod apartment;
mod office;
mod warehouse;
mod hallway;
mod home;

use crate::map::MapDef;

pub static MAPS: [MapDef; 6] = [training::MAP, apartment::MAP, office::MAP, warehouse::MAP, hallway::MAP, home::MAP];
