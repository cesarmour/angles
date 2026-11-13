# Angles

Current version: **Kill House** (0.12.0).

A teaching simulator for close-quarters battle (CQB) fundamentals, built with Rust and Bevy 0.16.
A shoot house with IPSC-style cardboard targets, a six-lesson tutorial, first-person and top-down views,
a tactical view with one bar for the doorway in front of you (how much of its 180° fan you have opened, and which way to step),
an ideal route with a demo mode, live technique feedback, scoring, odds of success and badges.
Four maps (training house, apartment, office, warehouse); enemy and hostage positions are random on every run.

> A teaching tool for fundamentals. Real technique is learned with a qualified instructor, as a team, in a controlled environment.

## Install (macOS, Apple silicon)
Paste this line in Terminal; it downloads the latest release to `~/Applications` and opens it (the app is signed; Apple's notarization is in review, so a zip downloaded with a browser is still blocked by macOS):

    curl -L https://github.com/cesarmour/point-of-entry/releases/latest/download/Angles-mac.zip -o /tmp/Angles.zip && ditto -x -k /tmp/Angles.zip ~/Applications && open ~/Applications/Angles.app

## Controls
W A S D move · mouse look · left button fire · Space or right button aim (Z locks the aim) · Q / E lean · C or Ctrl crouch ·
Shift fast step · R reload (tactical with rounds left, emergency when empty) · 1 2 or X switch weapon (pistol, SCAR) · F open door · L flashlight · G tactical view · V top-down view ·
T ideal route (always / tip / off) · O hide the lesson box · H review the lesson and watch the demo · Esc pause

## Build and run (macOS)
    cargo run --release
    cargo build --release && ./bundle.sh     # makes dist/Angles.app

## Automatic tests (scripted operator, no mouse needed)
    ./run.sh run 2      # runs the six lessons along the ideal route and prints the state of each one
    ./run.sh demo       # runs the five lesson demos
    ./run.sh guns       # the two weapons: ready, aiming, firing, switching and reloading
    CQB_NODEATH=1 ./run.sh walker   # scenario 9: rounds bounce off the mech's front plate, then hits from behind (CQB_NOREACT=1 to see it go down)
    CQB_NODEATH=1 ./run.sh nade    # an alerted threat without a line of fire throws a grenade at the stationary operator; `alarm` shows a room's threats leaving their posts to attack
    CQB_NODEATH=1 CQB_NOREACT=1 ./run.sh slice   # home defense: an intruder stacks on the jamb, leans and slices a bedroom door; `scare` forces the flinch and the retreat to cover
    cargo test --release home_map_routes   # no window: checks the family home data, the routes, the windows and the window post
    ./run.sh scns       # opens each of the eight scenarios (briefing card and first frame), then the scenario and ranking screens
    ./run.sh lock       # locked door: breach; `rescue` (CQB_CLOCK=5), `mind` (surrender / false surrender), `rear` (roaming threat), `shield`, `tops`
    CQB_POS="12.3,5.6,9.3,0.7" CQB_BOTLAG=250 ./run.sh hold   # home defense: scripted operator holds a position (x,z,look x,z), 250 ms reaction; CQB_WAVE=n starts at wave n; CQB_FLEE="x,z;x,z" is the path the operator runs when a grenade lands
    ./run.sh diff       # difficulty: menu, EASY (cardboard targets) and HARD (operator down); CQB_DIFF=0|1|2 sets it for any script
    ./run.sh down       # stand in front of a threat until the operator is down, then try again; `back` starts behind it, `down2` does it in the tutorial
    ./run.sh enemy      # a threat raising its rifle and firing a burst; `host` walks up to hostages and threats; `falls` drops four threats
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
