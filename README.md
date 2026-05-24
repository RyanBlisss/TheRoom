# The Room

**The Room** is a 3D first-person atmospheric horror game built in Rust.  
You are trapped in the Oceancrest Hotel. Every room you unlock makes the hotel worse. Find the keys. Manage your sanity. Survive.

Audio is the primary mechanic. What you hear matters more than what you see.

---

## Table of Contents

- [Building & Running](#building--running)
- [Controls](#controls)
- [Dev Mode](#dev-mode)
- [Scripting (Lua)](#scripting-lua)
- [Modding & Customization](#modding--customization)
- [Game Concept](#game-concept)
- [Gameplay Loop](#gameplay-loop)
- [Sanity System](#sanity-system)
- [Monster System](#monster-system)
- [Items](#items)
- [Locations](#locations)
- [Characters](#characters)
- [Story](#story)
- [Implementation Status](#implementation-status)

---

## Building & Running

**Requirements:** Rust (stable), a GPU with OpenGL 3.3+

```bash
cd game
cargo run --release
```

For development (faster compile, debug info):

```bash
cd game
cargo build
./target/debug/the_room
```

The game expects to be run from the `game/` directory so it can find `assets/` and `scripts/`.

### Dependencies

All managed by Cargo:

| Crate | Purpose |
|---|---|
| `glutin` + `winit` | Window + OpenGL context |
| `gl` + `glow` | OpenGL bindings |
| `egui` + `egui_glow` | All UI (menus, HUD, dev mode) |
| `nalgebra-glm` | 3D math |
| `rodio` | Audio playback |
| `mlua` | Lua 5.4 scripting engine |
| `tobj` | OBJ model loading |
| `serde` + `serde_json` | Settings, layout, character save |
| `rand` | Item shuffle randomization |

---

## Controls

| Key | Action |
|---|---|
| `W A S D` | Move |
| Mouse | Look |
| `Space` | Jump |
| `E` | Interact with door / item — or bag a held item |
| `G` | Grab nearest item in front of you |
| `R` | Throw held item |
| `F` | Use selected hotbar item |
| `1–5` | Select hotbar slot |
| `Tab` | Open / close inventory |
| `Esc` | Pause menu |
| `F2` | Toggle dev mode |

### Mouse sensitivity

Adjust in **Settings** from the main menu or pause menu. The slider range is 0.0003–0.005.

---

## Dev Mode

Press **F2** during gameplay to open the dev overlay. Mouse is released and the cursor becomes visible.

The overlay has four tabs:

### Items tab

Lists every item currently in the world. Click an item to expand it:

- Drag **X / Y / Z** sliders to move it in real time
- Click **Delete** to remove it from the world
- Use **Spawn Item** at the top to create a new SanityPill, WindUpToy, or CD at the origin

### Rooms tab

Lists all rooms with a color swatch. Click a room to expand it and drag **R / G / B** sliders to change its wall/floor/ceiling color live.

### Scripts tab

Shows which `.lua` files are currently loaded. At the bottom is a **Lua console** — type any Lua expression and press Enter or click **Run**. The result appears in the log above. Use this to call any scripting API function interactively.

### Info tab

Shows your current position, item and room counts, and a full keybinding reference.

### Dev toolbar

| Button | Action |
|---|---|
| **Fly ON/OFF** | Toggles fly camera — disables collision so you can move through walls and floors |
| **Save Layout** | Writes all item positions and room colors to `layout.json` |
| **Reload Scripts** | Hot-reloads all `.lua` files from `scripts/` without restarting |

---

## Scripting (Lua)

On first run, a `scripts/` directory is created automatically with an `example.lua` starter file. Drop any `.lua` file in that folder and it will be loaded at game start. Use **Reload Scripts** in dev mode to reload without restarting.

### Hooks

Define these functions in your script to respond to game events:

```lua
function on_tick(dt)
    -- Called every frame. dt = seconds since last frame.
end

function on_door_open(room_id)
    -- Called when any locked door is unlocked.
    -- room_id is the numeric ID of the room that was opened.
end

function on_item_pickup(label)
    -- Called when the player bags an item.
    -- label is a string like "Key", "Sanity Pill", "CD", etc.
end

function on_sanity_change(value)
    -- Called every frame with current sanity (0.0 = gone, 1.0 = full).
end

function on_player_move(x, y, z)
    -- Called every frame with player world position.
end
```

### API

```lua
show_message("text")
-- Display a message in the center-bottom HUD for 3 seconds.

set_room_color(room_id, r, g, b)
-- Change a room's color. Values are 0.0–1.0.
-- Example: set_room_color(ROOM_MAIN, 0.8, 0.2, 0.2)

spawn_item(kind, x, y, z)
-- Spawn an item at a world position.
-- kind: "SanityPill" | "WindUpToy" | "Cd"

move_item(label, x, y, z)
-- Move the first item matching label to a new position.

set_sanity(value)
-- Set current sanity directly. 0.0–1.0.

play_sound(path)
-- (Planned) Play a sound file from disk.
```

### Room ID constants

Pre-defined in every script:

```lua
ROOM_MAIN      -- 0   Floor 1 starting room
ROOM_BATH      -- 1   Floor 1 bathroom
ROOM_BED_A     -- 2   Floor 1 bedroom A
ROOM_BED_B     -- 3   Floor 1 bedroom B
ROOM_HALL      -- 4   Floor 1 hall
ROOM_KITCHEN   -- 5   Floor 1 kitchen
ROOM_DINING    -- 6   Floor 1 dining room
ROOM_F2_BATH   -- 7   Floor 2 bathroom
ROOM_F2_BED_A  -- 8   Floor 2 bedroom A
ROOM_F2_BED_B  -- 9   Floor 2 bedroom B
ROOM_F2_HALL   -- 10  Floor 2 upper hall
ROOM_THE_ROOM  -- 11  The final room
```

### Example script

```lua
-- Make the main room flash red when sanity drops below 25%
function on_sanity_change(value)
    if value < 0.25 then
        set_room_color(ROOM_MAIN, 0.55, 0.08, 0.08)
    else
        set_room_color(ROOM_MAIN, 0.72, 0.62, 0.48)
    end
end

-- Spawn a sanity pill every time a door is opened
function on_door_open(room_id)
    show_message("A door opens. Something shifts.")
    spawn_item("SanityPill", 0.0, 1.0, 0.0)
end
```

---

## Modding & Customization

### Asset overrides

Place files in the following paths to override built-in assets:

| Path | Overrides |
|---|---|
| `assets/sounds/ambient.ogg` | Ambient loop |
| `assets/sounds/heartbeat.ogg` | Low-sanity heartbeat |
| `assets/sounds/door_unlock.ogg` | Door unlock one-shot |
| `assets/sounds/pill_pickup.ogg` | Pill pickup one-shot |
| `assets/textures/wall.png` | Wall texture (not yet wired) |
| `assets/textures/floor.png` | Floor texture (not yet wired) |
| `assets/textures/ceiling.png` | Ceiling texture (not yet wired) |

### Shader overrides

The game currently compiles shaders from source at `src/shaders/`. Editing those files and recompiling gives full control over the rendering pipeline — including changing shading model, adding post-processing, switching to a 2D renderer, or replacing the entire visual system.

Planned: runtime shader hot-reload from `assets/shaders/` without recompiling.

### Layout

After editing item positions and room colors in dev mode, click **Save Layout**. This writes `layout.json` to the game directory. Planned: the game will load this file on start if present, allowing server operators to define custom layouts without recompiling.

### Scripts (server developers)

The Lua scripting system gives server operators full control over:

- item placement and behavior
- room color and atmosphere
- sanity manipulation
- event responses (door opens, item pickups, player movement)
- custom messages and narrative

Drop `.lua` files in `scripts/` and they load automatically. Multiple scripts are supported — all hooks from all scripts fire on each event.

---

## Game Concept

**Genre:** 3D first-person atmospheric horror  
**Engine:** Custom OpenGL renderer written in Rust  
**Setting:** The Oceancrest Hotel — a shifting, hostile building  
**Tone:** Psychological horror, isolation, escalating dread

Audio is the primary mechanic. Ambient loops, directional sound, a heartbeat that scales with insanity, and room-specific atmosphere all communicate danger, monster proximity, and story.

---

## Gameplay Loop

1. Start in the main room.
2. Find keys hidden throughout the hotel.
3. Use a key to unlock a new room.
4. The hotel changes — items move, sanity drops permanently by 20%.
5. The monster becomes more dangerous.
6. Audio and visual horror intensify.
7. Repeat until all rooms are unlocked.
8. Enter **The Room**.
9. Survive the final encounter.
10. Escape.

---

## Sanity System

Sanity has two separate pools:

### Permanent ceiling (`base`)

Starts at 1.0. Decreases by **20%** each time a locked door is opened.  
**Cannot be restored.** Pills and music cannot heal this.

### Current sanity

Drains passively over time (~3 minutes to fully drain at normal rate).  
Can be restored up to the current `base` ceiling using sanity pills or the CD player.

### Effects of low sanity

- The 3D world desaturates toward a cold sickly green-grey
- Colors darken dramatically as sanity approaches zero
- Dark red vignette closes in from the screen edges
- Heartbeat audio fades in above ~35% insanity and intensifies toward 100%

### Restoring sanity

| Method | Amount | Restores permanent? |
|---|---|---|
| Sanity pill | +25% | No |
| CD player (with CD inserted) | +0.6%/min | No |
| Wind-up toy (distraction) | +4% | No |

---

## Monster System

The monster (Isaac Remington / The Entity) does not appear at the start.

It spawns the first time the player uses a key to open a locked door.

### Behavior

- Teleports every 60 seconds to a room that does not contain the player
- Moves toward the player's last known position between teleports
- Gets faster with each catch
- Will visit locations the player frequents (planned)

### Catch system

Max catches: **8**

Each catch:
- triggers room changes
- changes visual and audio atmosphere
- increases monster speed
- decreases current sanity slightly

### Weakness

Isaac Remington is weak to **light** and to a sound frequency of **6798 Hz** — which the CD player may be able to produce.

---

## Items

### Keys

Each key unlocks one specific door. Keys shuffle to new rooms each time a door is opened.

### Sanity Pills

Restore 25% of temporary sanity. Do not heal permanent sanity loss.

Use: select in hotbar, press **F**.

### Wind-Up Toy

Place it as a noise decoy. Using it (**F**) drops it at your feet, making a clicking sound that can lure the monster.  
Also grants a small (+4%) sanity boost from the familiar noise.

### CD

Insert into the CD Player by carrying it and pressing **E** near the player.  
While playing, sanity restores at ~0.6%/minute.

### CD Player

A fixed fixture (cannot be picked up). Interact with **E** while holding a CD to insert it and start music playback.

---

## Locations

### First Floor

| Room | Notes |
|---|---|
| Main Room | Player start. Warm amber tones. |
| Bathroom | Teal tiles. |
| Bedroom A | Dusty mauve. |
| Bedroom B | Sage green. |
| Hall | Cool blue-grey corridor. |
| Kitchen | Warm yellow. |
| Dining Room | Terracotta. |

### Second Floor

| Room | Notes |
|---|---|
| Bathroom | Deeper teal. |
| Bedroom A | Deeper mauve. |
| Bedroom B | Deeper sage. |
| Upper Hall | Cold, darker than floor 1. |
| The Room | Blood red. Final encounter. |

Floor 2 is accessed via the staircase in the Hall area (x: 4–6, z: -1 to 3). The floor transition is smooth — walk up the slope.

No kitchen, dining room, or starting main room on floor 2.

---

## Characters

### Blarg Thompson — Player character

| | |
|---|---|
| Age | 32 |
| Occupation | Software Developer |
| Height | 5'11" / 180 cm |

Blarg grew up in the Midwest, moved to a big city for college, and developed a passion for urban exploration and urban legends. Logical, methodical, and skeptical — but deeply curious about the unexplained.

**Strengths:** analytical thinking, puzzle solving, resourcefulness  
**Weaknesses:** overly logical, dismissive of emotions, mild claustrophobia

---

### Isaac Remington — The Entity

| | |
|---|---|
| Age | 45 |
| Height | 7'0" / 213 cm |
| Former Occupation | Neuroscientist, AI researcher |

Dr. Remington developed an AI capable of interfacing directly with the human brain. Diagnosed with aggressive brain cancer, he replaced parts of his brain and organs with AI-enhanced prosthetics. The procedure went wrong. He is now a malevolent entity haunting the hotel, growing stronger as Blarg's sanity declines.

**Strengths:** manipulates environment, induces hallucinations, teleports between rooms, grows stronger with insanity  
**Weakness:** light and sound at 6798 Hz

---

### Evelyn Parker — Ghostly Hotel Staff

Former hotel manager. Disappeared decades ago. Now exists as a ghost trying to help Blarg navigate the hotel and piece together its history. Her memories are fragmented and not always reliable.

---

### Michael "Mike" Williams — Ghostly Hotel Guest

A traveling musician who died at the hotel. Friendly, charismatic, and optimistic despite his circumstances. His music can temporarily slow sanity loss. He cannot leave the room where he died.

---

### Linda Garcia — Ghostly Hotel Guest

A school teacher attending a conference. Now a gentle guiding spirit. She can provide hints and puzzle clues, but her presence occasionally attracts The Entity — so she appears sparingly.

---

## Story

### Prologue

You are Blarg Thompson. You arrive at the Oceancrest Hotel for a vacation. Florida sun, ocean view, cold drinks. Everything looks perfect.

### Scene I — The Perfect Room

Your suite is exactly what you wanted. Sunlight, white linens, a leather couch, turquoise water visible from the balcony. For a moment, you relax completely.

### Scene II — Beachside Reverie

You head down to the beach. Music plays, people laugh. You forget sunscreen. You go back inside to get it.

That is the last normal moment.

### Scene III — Unnerving Silence

The lobby is empty. The tropical air is gone. The bass from the beach party has disappeared. The front desk is unmanned. Your voice echoes wrong.

### Scene IV — Lights Out

You reach your floor. Every light in the corridor — and your room — snaps off at once. You hear something metallic behind you. Nothing is there.

### Scene V — First Ominous Clue

On the bedside table is a folded note on hotel stationery:

> *Dear Guest,*  
> *We hope you're enjoying your stay. For your safety, please remain in your room until further notice.*  
> *— Management*

No explanation. No signature that feels real.

### Scene VI — The Whispering Vent

The ventilation begins to whisper:

> *"...don't trust the lights..."*  
> *"...he sees you..."*

### Scene VII — Turning Point

The hotel is now a cage. Blarg must survive using logic, whatever he can find, and whatever the ghosts are willing to tell him.

---

## Implementation Status

### Done

- 3D first-person renderer (OpenGL 3.3, custom shaders)
- Two-floor hotel with 12 rooms and a staircase
- WASD movement, mouse look, jump, collision, gravity
- Door unlock chain — each key opens one door in sequence
- Item system — keys, sanity pills, wind-up toy, CD, CD player
- Grab mechanic (G), throw (R), bag while holding (E)
- Inventory with hotbar (1–5), full inventory screen (Tab)
- Sanity system — passive drain, permanent hits on door unlock, pill restore
- Sanity visual effects — desaturation, cold color shift, darkening, edge vignette
- Item shuffle — items move to random positions when a door is unlocked
- CD player interaction — insert CD, sanity restores slowly while playing
- Wind-up toy — drop as noise decoy, small sanity boost
- Audio manager — ambient loop, heartbeat (scales with insanity), door/pickup SFX
- Main menu, pause menu, settings (sensitivity, FOV, volume, fullscreen, FPS counter)
- Character customization screen (name, skin tone, hair/shirt/pants color)
- Multiplayer lobby UI (network stub, not yet functional)
- Dev mode (F2) — item editor, room color editor, Lua console, fly cam, save layout
- Lua scripting engine — hooks, game API, hot-reload, auto-created example script
- 16:9 aspect ratio lock with letterbox/pillarbox at any window size
- Settings persistence (JSON)
- Character save/load (JSON)

### Planned / In Progress

- Monster (Isaac Remington) — spawn, teleport, AI pathfinding, catch system
- Ghost characters (Evelyn, Mike, Linda) — appearances, dialogue, hints
- Real 3D models (OBJ loading via `tobj` is wired; geometry is still boxes)
- Texture mapping (UV data is in the mesh; sampler not yet in shader)
- Hallucinations tied to sanity level
- Final room encounter
- Story notes and narrative events
- Runtime shader hot-reload from files
- `layout.json` loading at startup
- Multiplayer (network layer stubbed out)
- Sound frequency mechanic (6798 Hz / CD player interaction with monster)

---

*Built with Rust. Horror optional. Audio mandatory.*
