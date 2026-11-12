# Point of Entry

A teaching simulator for close-quarters battle (CQB) fundamentals, built with Rust and Bevy 0.16.
A shoot house with IPSC-style cardboard targets, a six-lesson tutorial, first-person and top-down views,
a tactical view that shows your sector and how much of each doorway's 180° fan you have opened,
an ideal route with a demo mode, live technique feedback, scoring, odds of success and badges.
Four maps (training house, apartment, office, warehouse); enemy and hostage positions are random on every run.

> A teaching tool for fundamentals. Real technique is learned with a qualified instructor, as a team, in a controlled environment.

## Controls
W A S D move · mouse look · left button fire · Space or right button aim (Z locks the aim) · Q / E lean · C or Ctrl crouch ·
Shift fast step · R reload (tactical with rounds left, emergency when empty) · 1 2 or X switch weapon (pistol, SCAR) · F open door · L flashlight · G tactical view · V top-down view ·
T ideal route (always / tip / off) · O hide the lesson box · H review the lesson and watch the demo · Esc pause

## Build and run (macOS)
    cargo run --release
    cargo build --release && ./bundle.sh     # makes dist/Point of Entry.app

## Automatic tests (scripted operator, no mouse needed)
    ./run.sh run 2      # runs the six lessons along the ideal route and prints the state of each one
    ./run.sh demo       # runs the five lesson demos
    ./run.sh guns       # the two weapons: ready, aiming, firing, switching and reloading
    ./run.sh enemy      # a threat raising its pistol and firing; `host` walks up to hostages and threats; `falls` drops four threats
    ./run.sh shots      # captures screens into shots/
    ./run.sh maps       # loads each extra map, checks its data and captures it
The Mac screen must be on: with it off the game stops drawing.

## Source layout
- `src/map.rs` map tables (`MAPS`), collision, visibility, openings, map validator · `src/world.rs` scenery, materials, lights
- `src/player.rs` movement, stance, cameras, gun · `src/combat.rs` targets, shooting, doors, effects
- `src/tactical.rs` tactical view · `src/lessons.rs` lessons, ideal route, coach, scoring, badges, demo
- `src/ui.rs` interface · `src/audio.rs` sound · `src/art.rs` vector art (targets, operator, icon) · `src/bot.rs` test scripts

## Credits and licences
See [CREDITS.md](CREDITS.md). The first-person weapons, the operator and the threats are CC BY 4.0 models from Sketchfab; the hostage is CC BY-NC 4.0 (non-commercial only). All are credited there with the changes made.
