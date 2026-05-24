use mlua::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Events fired by the game into Lua hooks.
#[derive(Debug, Clone)]
pub enum GameEvent {
    Tick { dt: f32 },
    DoorOpen { room_id: usize },
    ItemPickup { label: String },
    SanityChange { value: f32 },
    PlayerMove { x: f32, y: f32, z: f32 },
}

/// Commands that Lua scripts can send back to the game.
#[derive(Debug, Clone)]
pub enum ScriptCmd {
    ShowMessage(String),
    SetRoomColor { room_id: usize, r: f32, g: f32, b: f32 },
    SpawnItem { kind: String, x: f32, y: f32, z: f32 },
    MoveItem { label: String, x: f32, y: f32, z: f32 },
    SetSanity(f32),
    PlaySound(String),
}

pub struct ScriptEngine {
    lua: Lua,
    cmd_queue: Arc<Mutex<Vec<ScriptCmd>>>,
    loaded_scripts: Vec<String>,
}

impl ScriptEngine {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let cmd_queue: Arc<Mutex<Vec<ScriptCmd>>> = Arc::new(Mutex::new(Vec::new()));

        // ── Register API functions ────────────────────────────────────────
        let q = Arc::clone(&cmd_queue);
        lua.globals().set("show_message", lua.create_function(move |_, msg: String| {
            q.lock().unwrap().push(ScriptCmd::ShowMessage(msg));
            Ok(())
        })?)?;

        let q = Arc::clone(&cmd_queue);
        lua.globals().set("set_room_color", lua.create_function(move |_, (room_id, r, g, b): (usize, f32, f32, f32)| {
            q.lock().unwrap().push(ScriptCmd::SetRoomColor { room_id, r, g, b });
            Ok(())
        })?)?;

        let q = Arc::clone(&cmd_queue);
        lua.globals().set("spawn_item", lua.create_function(move |_, (kind, x, y, z): (String, f32, f32, f32)| {
            q.lock().unwrap().push(ScriptCmd::SpawnItem { kind, x, y, z });
            Ok(())
        })?)?;

        let q = Arc::clone(&cmd_queue);
        lua.globals().set("move_item", lua.create_function(move |_, (label, x, y, z): (String, f32, f32, f32)| {
            q.lock().unwrap().push(ScriptCmd::MoveItem { label: label.to_string(), x, y, z });
            Ok(())
        })?)?;

        let q = Arc::clone(&cmd_queue);
        lua.globals().set("set_sanity", lua.create_function(move |_, v: f32| {
            q.lock().unwrap().push(ScriptCmd::SetSanity(v.clamp(0.0, 1.0)));
            Ok(())
        })?)?;

        let q = Arc::clone(&cmd_queue);
        lua.globals().set("play_sound", lua.create_function(move |_, path: String| {
            q.lock().unwrap().push(ScriptCmd::PlaySound(path));
            Ok(())
        })?)?;

        // Room ID constants — scoped so `globals` is dropped before lua is moved
        {
            let globals = lua.globals();
            globals.set("ROOM_MAIN",     0usize)?;
            globals.set("ROOM_BATH",     1usize)?;
            globals.set("ROOM_BED_A",    2usize)?;
            globals.set("ROOM_BED_B",    3usize)?;
            globals.set("ROOM_HALL",     4usize)?;
            globals.set("ROOM_KITCHEN",  5usize)?;
            globals.set("ROOM_DINING",   6usize)?;
            globals.set("ROOM_F2_BATH",  7usize)?;
            globals.set("ROOM_F2_BED_A", 8usize)?;
            globals.set("ROOM_F2_BED_B", 9usize)?;
            globals.set("ROOM_F2_HALL",  10usize)?;
            globals.set("ROOM_THE_ROOM", 11usize)?;
        }

        Ok(Self { lua, cmd_queue, loaded_scripts: Vec::new() })
    }

    /// Load all .lua files from the scripts/ directory.
    pub fn load_scripts(&mut self) {
        let dir = Path::new("scripts");
        if !dir.exists() {
            let _ = std::fs::create_dir(dir);
            // Write example script on first run
            let example = r#"-- The Room — example script
-- This file is loaded automatically at game start.
-- Hooks: on_tick(dt), on_door_open(room_id), on_item_pickup(label), on_sanity_change(value)
-- API:   show_message(str), set_room_color(room_id, r, g, b), spawn_item(kind, x, y, z)
--        move_item(label, x, y, z), set_sanity(value), play_sound(path)

function on_door_open(room_id)
    -- Example: flash a message when any door opens
    -- show_message("A door creaks open... room " .. room_id)
end

function on_sanity_change(value)
    -- Example: tint the main room red when sanity is critical
    if value < 0.25 then
        set_room_color(ROOM_MAIN, 0.6, 0.1, 0.1)
    end
end

function on_tick(dt)
    -- Called every frame. dt = delta time in seconds.
end
"#;
            let _ = std::fs::write("scripts/example.lua", example);
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("lua") { continue; }
            match std::fs::read_to_string(&path) {
                Ok(src) => {
                    match self.lua.load(&src).exec() {
                        Ok(_) => {
                            let name = path.to_string_lossy().to_string();
                            eprintln!("[scripts] loaded: {}", name);
                            self.loaded_scripts.push(name);
                        }
                        Err(e) => eprintln!("[scripts] error in {:?}: {}", path, e),
                    }
                }
                Err(e) => eprintln!("[scripts] could not read {:?}: {}", path, e),
            }
        }
    }

    /// Reload all scripts from disk (useful for hot-reloading in dev mode).
    pub fn reload(&mut self) {
        self.loaded_scripts.clear();
        // Reset Lua state
        if let Ok(fresh) = Lua::new().load("").exec() { let _ = fresh; }
        // Re-initialize is complex — for now just re-exec all files
        self.load_scripts();
    }

    /// Fire a game event into all loaded Lua hooks.
    pub fn fire(&self, event: &GameEvent) -> Vec<ScriptCmd> {
        let result = match event {
            GameEvent::Tick { dt } => {
                self.call_hook("on_tick", (*dt,))
            }
            GameEvent::DoorOpen { room_id } => {
                self.call_hook("on_door_open", (*room_id as u64,))
            }
            GameEvent::ItemPickup { label } => {
                self.call_hook("on_item_pickup", (label.clone(),))
            }
            GameEvent::SanityChange { value } => {
                self.call_hook("on_sanity_change", (*value,))
            }
            GameEvent::PlayerMove { x, y, z } => {
                self.call_hook("on_player_move", (*x, *y, *z))
            }
        };
        if let Err(e) = result {
            eprintln!("[scripts] hook error: {}", e);
        }
        // Drain commands generated by this event
        self.cmd_queue.lock().unwrap().drain(..).collect()
    }

    fn call_hook<A>(&self, name: &str, args: A) -> LuaResult<()>
    where A: for<'lua> IntoLuaMulti<'lua> {
        let globals = self.lua.globals();
        if let Ok(f) = globals.get::<_, LuaFunction>(name) {
            f.call::<_, ()>(args)?;
        }
        Ok(())
    }

    /// Drain any pending commands that scripts produced outside of hooks.
    pub fn drain_cmds(&self) -> Vec<ScriptCmd> {
        self.cmd_queue.lock().unwrap().drain(..).collect()
    }

    /// Execute a one-shot Lua chunk (used by the dev console).
    pub fn run_chunk(&self, src: &str) -> Result<String, String> {
        match self.lua.load(src).eval::<mlua::Value>() {
            Ok(v) => {
                // Drain any commands the chunk produced
                Ok(format!("{:?}", v))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn loaded_scripts(&self) -> &[String] {
        &self.loaded_scripts
    }
}
