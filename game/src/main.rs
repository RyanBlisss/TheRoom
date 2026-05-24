mod audio;
mod character;
mod game_phase;
mod items;
mod network;
mod player;
mod renderer;
mod sanity;
mod settings;
mod world;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use rand::Rng;

use glutin::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState,
        MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use winit::window::Fullscreen;
use nalgebra_glm as glm;

#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopExtMacOS};

use audio::AudioManager;
use character::CharacterConfig;
use game_phase::{GameMode, GamePhase};
use items::{Inventory, Item, ItemKind, ITEM_HALF_SIZE};
use network::NetworkManager;
use player::Player;
use renderer::{Mesh, Shader};
use sanity::Sanity;
use settings::Settings;
use world::World;

const VERT_SRC:       &str = include_str!("shaders/basic.vert");
const FRAG_SRC:       &str = include_str!("shaders/basic.frag");
const CROSS_VERT_SRC: &str = include_str!("shaders/crosshair.vert");
const CROSS_FRAG_SRC: &str = include_str!("shaders/crosshair.frag");

// ─── PlayingState ─────────────────────────────────────────────────────────────

struct PlayingState {
    player:        Player,
    world:         World,
    sanity:        Sanity,
    inventory:     Inventory,
    items:         Vec<Item>,
    item_meshes:   Vec<Mesh>,
    audio:         AudioManager,
    message:       Option<(String, Instant)>,
    projection:    glm::Mat4,
    selected_slot: usize,
    cd_playing:    bool,
    cd_restore_acc: f32,
}

impl PlayingState {
    fn new(character: &CharacterConfig, settings: &Settings, w: u32, h: u32) -> Self {
        let world = World::new();
        let s     = ITEM_HALF_SIZE;

        let items = vec![
            Item::key(world::ROOM_HALL,     glm::vec3(-2.0, 1.0, -7.0)),
            Item::key(world::ROOM_BATH,     glm::vec3(-7.5, 1.0,  1.0)),
            Item::key(world::ROOM_BED_A,    glm::vec3( 0.5, 1.0,  9.0)),
            Item::key(world::ROOM_BED_B,    glm::vec3( 7.0, 1.0,  8.0)),
            Item::key(world::ROOM_KITCHEN,  glm::vec3(-7.0, 1.0, -9.0)),
            Item::key(world::ROOM_DINING,   glm::vec3( 9.0, 1.0, -9.0)),
            Item::key(world::ROOM_F2_HALL,  glm::vec3( 3.0, world::FLOOR_2 + 1.0, -8.0)),
            Item::key(world::ROOM_F2_BATH,  glm::vec3(-7.5, world::FLOOR_2 + 1.0,  0.0)),
            Item::key(world::ROOM_F2_BED_A, glm::vec3( 0.0, world::FLOOR_2 + 1.0,  9.0)),
            Item::key(world::ROOM_F2_BED_B, glm::vec3( 7.0, world::FLOOR_2 + 1.0,  9.0)),
            Item::pill(glm::vec3( 2.0, 1.0,  2.0)),
            Item::pill(glm::vec3(-3.0, 1.0, -8.0)),
            Item::pill(glm::vec3( 8.0, 1.0, -8.0)),
            Item::pill(glm::vec3( 1.0, world::FLOOR_2 + 1.0, 8.0)),
            Item::wind_up_toy(glm::vec3(-6.5, 1.0,  1.5)),
            Item::cd(glm::vec3(1.0, 1.0, -3.0)),
            Item::cd_player(glm::vec3(-2.0, 1.0, 3.5)),
        ];

        let item_meshes = items.iter().map(|item| {
            let color = item_color(&item.kind);
            let (v, i) = renderer::build_box(glm::vec3(-s,-s,-s), glm::vec3(s,s,s));
            Mesh::new(&v, &i, color)
        }).collect();

        let aspect = w as f32 / h.max(1) as f32;
        Self {
            player:        Player::new(glm::vec3(0.0, world::FLOOR_1, 0.0)),
            world,
            sanity:        Sanity::new(),
            inventory:     Inventory::default(),
            items,
            item_meshes,
            audio:         AudioManager::new(),
            message:        None,
            projection:     glm::perspective(aspect, settings.fov.to_radians(), 0.05, 200.0),
            selected_slot:  0,
            cd_playing:     false,
            cd_restore_acc: 0.0,
        }
    }

    fn show_msg(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), Instant::now()));
    }

    fn shuffle_items(&mut self) {
        let mut rng = rand::thread_rng();
        let rooms = &self.world.rooms;
        for item in self.items.iter_mut() {
            if item.picked_up || !item.pickupable { continue; }
            // Pick a random unlocked room
            let unlocked: Vec<_> = rooms.iter().filter(|r| r.unlocked).collect();
            if unlocked.is_empty() { continue; }
            let room = unlocked[rng.gen_range(0..unlocked.len())];
            let margin = 1.0_f32;
            let x = rng.gen_range((room.min.x + margin)..(room.max.x - margin));
            let z = rng.gen_range((room.min.z + margin)..(room.max.z - margin));
            let y = if room.floor == 2 { world::FLOOR_2 + 1.0 } else { 1.0 };
            item.position = glm::vec3(x, y, z);
            item.landed   = false;
            item.vel_y    = 0.0;
        }
    }

    fn try_interact(&mut self) {
        let eye   = self.player.eye_position();
        let range = self.player.interact_range;

        for item in self.items.iter_mut() {
            if item.picked_up { continue; }
            if glm::distance(&eye, &item.position) <= range {
                if !item.pickupable {
                    match &item.kind {
                        ItemKind::CdPlayer => {
                            if self.cd_playing {
                                self.show_msg("The music plays softly...");
                            } else if self.inventory.has_cd() {
                                if let Some(pos) = self.inventory.items.iter().position(|k| k == &ItemKind::Cd) {
                                    self.inventory.items.remove(pos);
                                }
                                self.cd_playing = true;
                                self.show_msg("You insert the CD. Music fills the room.");
                            } else {
                                self.show_msg("CD Player — you need a CD to play.");
                            }
                        }
                        _ => self.show_msg("You can't pick that up."),
                    }
                    return;
                }
                item.picked_up = true;
                let label = item.label;
                if matches!(item.kind, ItemKind::SanityPill) {
                    self.audio.play_pill_pickup();
                }
                self.inventory.add(item.kind.clone());
                self.show_msg(format!("Picked up: {}", label));
                return;
            }
        }

        if let Some(door_idx) = self.world.nearest_door(&eye, range) {
            let door = &self.world.doors[door_idx];
            if door.locked {
                let key_room = door.key_room_id;
                if self.inventory.use_key_for(key_room) {
                    self.world.open_door(door_idx);
                    self.sanity.permanent_hit();
                    self.audio.play_door_unlock();
                    self.shuffle_items();
                    self.show_msg("The lock clicks open. Something feels different.");
                } else {
                    self.show_msg("Locked. You need a key.");
                }
            } else if !door.open {
                self.world.open_door(door_idx);
                self.show_msg("You push the door open.");
            }
        }
    }

    fn use_selected(&mut self) {
        let kind = self.inventory.items.get(self.selected_slot).cloned();
        match kind {
            Some(ItemKind::SanityPill) => {
                if self.sanity.use_pill() {
                    self.inventory.items.remove(self.selected_slot);
                    self.audio.play_pill_pickup();
                    self.show_msg("The pill helps. A little.");
                } else {
                    self.show_msg("You're as clear-headed as you're going to get.");
                }
            }
            Some(ItemKind::WindUpToy) => {
                self.inventory.items.remove(self.selected_slot);
                let s = ITEM_HALF_SIZE;
                let drop_pos = glm::vec3(self.player.position.x, 1.0, self.player.position.z);
                let mut toy = Item::wind_up_toy(drop_pos);
                toy.landed = false;
                let (v, ix) = renderer::build_box(glm::vec3(-s,-s,-s), glm::vec3(s,s,s));
                self.item_meshes.push(Mesh::new(&v, &ix, item_color(&ItemKind::WindUpToy)));
                self.items.push(toy);
                self.sanity.current = (self.sanity.current + 0.04).min(self.sanity.base);
                self.show_msg("You wind it up and set it down. The clicking echoes through the hall.");
            }
            Some(_) => self.show_msg("Can't use that right now."),
            None    => self.show_msg("Nothing selected."),
        }
        if self.selected_slot >= self.inventory.items.len() && self.selected_slot > 0 {
            self.selected_slot -= 1;
        }
    }

    fn update(&mut self, keys: &HashSet<VirtualKeyCode>, dt: f32) {
        self.sanity.tick(dt);

        if self.cd_playing {
            self.cd_restore_acc += dt;
            // Restore ~10% sanity over 60 seconds while music plays
            if self.cd_restore_acc >= 1.0 {
                self.cd_restore_acc = 0.0;
                self.sanity.current = (self.sanity.current + 0.006).min(self.sanity.base);
            }
        }

        self.audio.tick_heartbeat(self.sanity.insanity(), dt);
        self.player.update(keys, dt, &self.world.walls, Some(&self.world.stairs));

        for item in &mut self.items {
            let floor_y = if item.position.y > world::FLOOR_2 - 0.5 {
                world::FLOOR_2
            } else {
                world::FLOOR_1
            };
            item.physics_tick(dt, floor_y);
        }

        if let Some((_, t)) = &self.message {
            if t.elapsed().as_secs_f32() > 3.0 {
                self.message = None;
            }
        }
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

struct App {
    phase:     GamePhase,
    settings:  Settings,
    character: CharacterConfig,
    network:   NetworkManager,
    game:      Option<PlayingState>,

    shader:       Shader,
    cross_shader: Shader,
    cross_vao:    u32,
    cross_count:  i32,

    egui_ctx:     egui::Context,
    egui_painter: egui_glow::Painter,
    egui_events:  Vec<egui::Event>,
    egui_scale:   f32,
    egui_start:   Instant,

    keys:           HashSet<VirtualKeyCode>,
    mouse_captured: bool,
    inventory_open: bool,
    modifiers:      ModifiersState,
    mouse_pos:      egui::Pos2,
    window_size:    [u32; 2],
    game_vp:        [i32; 4], // 16:9 sub-viewport [x, y, w, h]
    last:           Instant,
    focused:        bool,
    fps:            f32,
    fps_accum:      f32,
    fps_frames:     u32,
}

impl App {
    fn new(glow_ctx: Arc<glow::Context>, window: &glutin::window::Window) -> Self {
        let shader       = Shader::new(VERT_SRC, FRAG_SRC);
        let cross_shader = Shader::new(CROSS_VERT_SRC, CROSS_FRAG_SRC);
        let (cross_vao, cross_count) = build_crosshair_vao();

        let egui_painter = egui_glow::Painter::new(glow_ctx, "", None)
            .expect("egui painter init failed");

        let egui_ctx = egui::Context::default();

        let mut vis = egui::Visuals::dark();
        vis.window_fill   = egui::Color32::from_rgba_premultiplied(6, 5, 8, 230);
        vis.panel_fill    = egui::Color32::from_rgba_premultiplied(6, 5, 8, 230);
        vis.window_rounding = egui::Rounding::none();
        vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(18, 15, 20);
        vis.widgets.hovered.bg_fill  = egui::Color32::from_rgb(45, 35, 55);
        vis.widgets.active.bg_fill   = egui::Color32::from_rgb(70, 50, 80);
        egui_ctx.set_visuals(vis);

        let scale = window.scale_factor() as f32;
        let size  = window.inner_size();

        Self {
            phase:     GamePhase::MainMenu,
            settings:  Settings::load(),
            character: CharacterConfig::load(),
            network:   NetworkManager::new(),
            game:      None,

            shader,
            cross_shader,
            cross_vao,
            cross_count,

            egui_ctx,
            egui_painter,
            egui_events: Vec::new(),
            egui_scale:  scale,
            egui_start:  Instant::now(),

            keys:           HashSet::new(),
            mouse_captured: false,
            inventory_open: false,
            modifiers:      ModifiersState::empty(),
            mouse_pos:      egui::Pos2::ZERO,
            window_size:    [size.width, size.height],
            game_vp:        compute_game_viewport(size.width, size.height),
            last:           Instant::now(),
            focused:        true,
            fps:            0.0,
            fps_accum:      0.0,
            fps_frames:     0,
        }
    }

    fn capture(&mut self, window: &glutin::window::Window, on: bool) {
        self.mouse_captured = on;
        window.set_cursor_grab(on).ok();
        window.set_cursor_visible(!on);
    }

    // ── egui helpers ──────────────────────────────────────────────────────

    fn take_raw_input(&mut self) -> egui::RawInput {
        let [w, h] = self.window_size;
        let scale  = self.egui_scale;
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(w as f32 / scale, h as f32 / scale),
            )),
            pixels_per_point: Some(scale),
            time: Some(self.egui_start.elapsed().as_secs_f64()),
            events: self.egui_events.drain(..).collect(),
            focused: self.focused,
            ..Default::default()
        }
    }

    fn paint_egui(&mut self, output: egui::FullOutput) {
        let [w, h] = self.window_size;
        let clipped = self.egui_ctx.tessellate(output.shapes);
        unsafe {
            // egui fills the full window — reset scissor and viewport first
            gl::Disable(gl::SCISSOR_TEST);
            gl::Viewport(0, 0, w as i32, h as i32);
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
        }
        self.egui_painter.paint_and_update_textures(
            [w, h], self.egui_scale, &clipped, &output.textures_delta,
        );
        self.restore_gl();
    }

    fn restore_gl(&self) {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Disable(gl::BLEND);
        }
    }

    // ── Event handling ────────────────────────────────────────────────────

    fn on_window_event(&mut self, event: &WindowEvent, window: &glutin::window::Window) {
        // Feed egui events when UI is visible
        let feed_egui = !self.mouse_captured || self.inventory_open;
        if feed_egui {
            self.collect_egui_event(event);
        }

        match event {
            WindowEvent::Resized(_) => {
                // window_size, game_vp, and projection are synced in render() each frame
                self.egui_scale = window.scale_factor() as f32;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.egui_scale = *scale_factor as f32;
            }
            WindowEvent::Focused(f) => {
                self.focused = *f;
                if *f && self.phase == GamePhase::Playing && !self.inventory_open {
                    self.capture(window, true);
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = *m;
            }
            WindowEvent::KeyboardInput {
                input: KeyboardInput { virtual_keycode: Some(key), state: ks, .. }, ..
            } => {
                match ks {
                    ElementState::Pressed  => { self.keys.insert(*key); }
                    ElementState::Released => { self.keys.remove(key); }
                }
                if *ks == ElementState::Pressed {
                    self.on_key(*key, window);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left, ..
            } => {
                if self.phase == GamePhase::Playing && !self.inventory_open {
                    self.capture(window, true);
                }
            }
            _ => {}
        }
    }

    fn collect_egui_event(&mut self, event: &WindowEvent) {
        let mods  = to_egui_mods(self.modifiers);
        let scale = self.egui_scale;

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let pos = egui::pos2(position.x as f32 / scale, position.y as f32 / scale);
                self.mouse_pos = pos;
                self.egui_events.push(egui::Event::PointerMoved(pos));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(btn) = match button {
                    MouseButton::Left   => Some(egui::PointerButton::Primary),
                    MouseButton::Right  => Some(egui::PointerButton::Secondary),
                    MouseButton::Middle => Some(egui::PointerButton::Middle),
                    _                   => None,
                } {
                    self.egui_events.push(egui::Event::PointerButton {
                        pos: self.mouse_pos,
                        button: btn,
                        pressed: *state == ElementState::Pressed,
                        modifiers: mods,
                    });
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(x, y)  => egui::vec2(*x * 12.0, *y * 12.0),
                    MouseScrollDelta::PixelDelta(p)    => egui::vec2(p.x as f32, p.y as f32),
                };
                self.egui_events.push(egui::Event::Scroll(scroll));
            }
            WindowEvent::ReceivedCharacter(c) => {
                if !c.is_control() {
                    self.egui_events.push(egui::Event::Text(c.to_string()));
                }
            }
            WindowEvent::KeyboardInput {
                input: KeyboardInput { virtual_keycode: Some(vk), state, .. }, ..
            } => {
                if let Some(key) = vk_to_egui(*vk) {
                    self.egui_events.push(egui::Event::Key {
                        key,
                        pressed: *state == ElementState::Pressed,
                        repeat: false,
                        modifiers: mods,
                    });
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.egui_events.push(egui::Event::PointerGone);
            }
            _ => {}
        }
    }

    fn on_key(&mut self, key: VirtualKeyCode, window: &glutin::window::Window) {
        match &self.phase.clone() {
            GamePhase::Playing => match key {
                VirtualKeyCode::Escape => {
                    self.capture(window, false);
                    self.inventory_open = false;
                    self.phase = GamePhase::Paused;
                }
                VirtualKeyCode::Tab => {
                    self.inventory_open = !self.inventory_open;
                    self.capture(window, !self.inventory_open);
                }
                VirtualKeyCode::E => { if let Some(g) = &mut self.game { g.try_interact(); } }
                VirtualKeyCode::F => { if let Some(g) = &mut self.game { g.use_selected(); } }
                VirtualKeyCode::Key1 => set_slot(&mut self.game, 0),
                VirtualKeyCode::Key2 => set_slot(&mut self.game, 1),
                VirtualKeyCode::Key3 => set_slot(&mut self.game, 2),
                VirtualKeyCode::Key4 => set_slot(&mut self.game, 3),
                VirtualKeyCode::Key5 => set_slot(&mut self.game, 4),
                _ => {}
            },
            GamePhase::Paused => {
                if key == VirtualKeyCode::Escape {
                    self.phase = GamePhase::Playing;
                    self.capture(window, true);
                }
            }
            _ => {}
        }
    }

    fn on_mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.mouse_captured && !self.inventory_open {
            if let Some(g) = &mut self.game {
                g.player.apply_mouse(dx as f32, dy as f32, self.settings.mouse_sensitivity);
            }
        }
    }

    // ── Update ────────────────────────────────────────────────────────────

    fn update(&mut self) {
        let now = Instant::now();
        let dt  = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;

        self.fps_accum  += dt;
        self.fps_frames += 1;
        if self.fps_accum >= 0.5 {
            self.fps       = self.fps_frames as f32 / self.fps_accum;
            self.fps_accum  = 0.0;
            self.fps_frames = 0;
        }

        self.network.tick();

        if self.phase == GamePhase::Playing {
            let keys = self.keys.clone();
            if let Some(g) = &mut self.game {
                g.update(&keys, dt);
            }
        }
    }

    // ── Render ────────────────────────────────────────────────────────────

    fn render(&mut self, window: &glutin::window::Window) {
        // Sync from actual window every frame — set_fullscreen is async on macOS
        // and the Resized event may arrive a few frames late.
        let phys = window.inner_size();
        let w = phys.width;
        let h = phys.height;
        self.window_size = [w, h];
        self.game_vp     = compute_game_viewport(w, h);
        let [vx, vy, vw, vh] = self.game_vp;

        // Keep projection in sync with the actual game viewport aspect ratio
        if let Some(g) = &mut self.game {
            g.projection = glm::perspective(
                vw as f32 / vh.max(1) as f32,
                self.settings.fov.to_radians(), 0.05, 200.0,
            );
        }

        unsafe {
            // Black letterbox bars
            gl::Viewport(0, 0, w as i32, h as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            // Constrained 16:9 game viewport
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(vx, vy, vw, vh);
            gl::Viewport(vx, vy, vw, vh);
            gl::ClearColor(0.02, 0.02, 0.025, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        match self.phase.clone() {
            GamePhase::Playing => self.render_playing(window),
            GamePhase::Paused  => { self.render_3d(); self.render_pause(window); }
            GamePhase::MainMenu => self.render_main_menu(window),
            GamePhase::CharacterCustomize { mode } => self.render_character(window, mode),
            GamePhase::MultiplayerLobby => self.render_lobby(window),
            GamePhase::Settings { return_to } => self.render_settings(window, *return_to),
        }
    }

    fn render_playing(&mut self, window: &glutin::window::Window) {
        self.render_3d();

        if !self.inventory_open {
            // Crosshair
            unsafe {
                gl::Disable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
            self.cross_shader.use_program();
            unsafe {
                gl::BindVertexArray(self.cross_vao);
                gl::DrawElements(gl::TRIANGLES, self.cross_count, gl::UNSIGNED_INT, std::ptr::null());
                gl::BindVertexArray(0);
            }
            self.restore_gl();

            self.render_hud(window);
        } else {
            self.render_inventory(window);
        }
    }

    fn render_3d(&mut self) {
        let g = match &self.game { Some(g) => g, None => return };

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        }

        self.shader.use_program();
        self.shader.set_mat4("projection", &g.projection);
        self.shader.set_mat4("view",       &g.player.view_matrix());
        let eye = g.player.eye_position();
        self.shader.set_vec3("lightPos",   &glm::vec3(0.0_f32, 2.5, 0.0));
        self.shader.set_vec3("eyePos",     &eye);
        self.shader.set_vec3("lightColor", &glm::vec3(1.0_f32, 0.95, 0.85));
        self.shader.set_float("ambientStrength", lerp(0.15, 0.55, g.sanity.normalised()));
        self.shader.set_float("sanity",    g.sanity.normalised());

        let id: glm::Mat4 = glm::identity();

        for room in &g.world.rooms {
            self.shader.set_mat4("model", &id);
            self.shader.set_vec3("objectColor", &room.color);
            room.mesh.draw();
        }
        for dm in &g.world.door_meshes {
            self.shader.set_mat4("model", &id);
            self.shader.set_vec3("objectColor", &dm.mesh.color);
            dm.mesh.draw();
        }
        for (i, item) in g.items.iter().enumerate() {
            if item.picked_up { continue; }
            let model = glm::translation(&item.position);
            self.shader.set_mat4("model", &model);
            self.shader.set_vec3("objectColor", &g.item_meshes[i].color);
            g.item_meshes[i].draw();
        }
    }

    // ── egui panels ──────────────────────────────────────────────────────

    fn render_hud(&mut self, window: &glutin::window::Window) {
        let sanity   = self.game.as_ref().map(|g| g.sanity.normalised()).unwrap_or(1.0);
        let msg      = self.game.as_ref().and_then(|g| g.message.as_ref()).map(|(m,_)| m.clone());
        let sel      = self.game.as_ref().map(|g| g.selected_slot).unwrap_or(0);
        let inv: Vec<ItemKind> = self.game.as_ref().map(|g| g.inventory.items.clone()).unwrap_or_default();
        let room_name = self.game.as_ref().map(|g| g.world.room_name_at(&g.player.position)).unwrap_or("");
        let fps      = self.fps;
        let show_fps = self.settings.show_fps;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            // ── Sanity vignette overlay ───────────────────────────────────
            let insanity = 1.0 - sanity;
            if insanity > 0.05 {
                egui::Area::new("vignette")
                    .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
                    .show(ctx, |ui| {
                        let screen = ctx.screen_rect();
                        let p = ui.painter();
                        let alpha = (insanity * insanity * 210.0) as u8;
                        let col = egui::Color32::from_rgba_premultiplied(90, 0, 10, alpha);
                        let edge = screen.width().min(screen.height()) * 0.30;
                        // Four edge strips — darkness closing in
                        p.rect_filled(egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, screen.min.y + edge)), 0.0, col);
                        p.rect_filled(egui::Rect::from_min_max(egui::pos2(screen.min.x, screen.max.y - edge), screen.max), 0.0, col);
                        p.rect_filled(egui::Rect::from_min_max(screen.min, egui::pos2(screen.min.x + edge, screen.max.y)), 0.0, col);
                        p.rect_filled(egui::Rect::from_min_max(egui::pos2(screen.max.x - edge, screen.min.y), screen.max), 0.0, col);
                    });
            }

            // ── Room name — top left ──────────────────────────────────────
            egui::Area::new("room_name")
                .anchor(egui::Align2::LEFT_TOP, [14.0, 14.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(room_name)
                        .size(12.0)
                        .color(egui::Color32::from_rgba_premultiplied(220, 210, 190, 160)));
                });

            // ── FPS counter — top right ───────────────────────────────────
            if show_fps {
                egui::Area::new("fps")
                    .anchor(egui::Align2::RIGHT_TOP, [-14.0, 14.0])
                    .show(ctx, |ui| {
                        ui.label(egui::RichText::new(format!("{:.0} fps", fps))
                            .size(11.0)
                            .color(egui::Color32::from_gray(80))
                            .monospace());
                    });
            }

            // ── Sanity bar — bottom center ────────────────────────────────
            egui::Area::new("sanity_bar")
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("SANITY").size(10.0).color(egui::Color32::from_gray(70)));
                    let color = if sanity > 0.5 { egui::Color32::from_rgb(80, 175, 120) }
                                else if sanity > 0.25 { egui::Color32::from_rgb(200, 155, 50) }
                                else { egui::Color32::from_rgb(200, 55, 55) };
                    ui.add(egui::ProgressBar::new(sanity).desired_width(200.0).fill(color));
                });

            // ── Hotbar ────────────────────────────────────────────────────
            egui::Area::new("hotbar")
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -72.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for i in 0..5usize {
                            let label = inv.get(i).map(item_label).unwrap_or("·");
                            let is_sel = i == sel;
                            let col = if is_sel { egui::Color32::from_rgb(235, 195, 90) }
                                      else      { egui::Color32::from_gray(60) };
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgba_premultiplied(10, 8, 12, 185))
                                .stroke(egui::Stroke::new(if is_sel { 2.0 } else { 0.5 }, col))
                                .rounding(egui::Rounding::same(3.0))
                                .inner_margin(egui::style::Margin::symmetric(8.0, 5.0))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(label).size(12.0).color(col).monospace());
                                });
                        }
                    });
                });

            // ── Message ───────────────────────────────────────────────────
            if let Some(m) = &msg {
                egui::Area::new("msg")
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -120.0])
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_premultiplied(6, 5, 8, 160))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::style::Margin::symmetric(10.0, 5.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(m.as_str())
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(230, 220, 200)));
                            });
                    });
            }

            // ── Hint ──────────────────────────────────────────────────────
            egui::Area::new("hint")
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("E Interact  F Use  Tab Inventory  Esc Pause")
                        .size(10.0).color(egui::Color32::from_gray(35)));
                });
        });
        self.paint_egui(out);
    }

    fn render_inventory(&mut self, _window: &glutin::window::Window) {
        let inv_items: Vec<ItemKind> = self.game.as_ref()
            .map(|g| g.inventory.items.clone())
            .unwrap_or_default();
        let sel = self.game.as_ref().map(|g| g.selected_slot).unwrap_or(0);
        let mut new_sel = sel;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::Window::new("INVENTORY")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.set_min_width(300.0);
                    if inv_items.is_empty() {
                        ui.label(egui::RichText::new("Empty").color(egui::Color32::from_gray(70)));
                    } else {
                        // Group by type
                        let mut groups: Vec<(&str, usize)> = Vec::new();
                        for kind in &inv_items {
                            let l = item_label(kind);
                            if let Some(g) = groups.iter_mut().find(|g| g.0 == l) {
                                g.1 += 1;
                            } else {
                                groups.push((l, 1));
                            }
                        }
                        egui::Grid::new("inv").num_columns(2).spacing([12.0,6.0]).show(ui, |ui| {
                            for (i, (label, count)) in groups.iter().enumerate() {
                                let is_sel = i == sel;
                                let col = if is_sel { egui::Color32::from_rgb(220,190,90) }
                                          else      { egui::Color32::from_gray(160) };
                                if ui.selectable_label(is_sel,
                                    egui::RichText::new(*label).color(col)).clicked() {
                                    new_sel = i;
                                }
                                ui.label(egui::RichText::new(format!("×{}", count))
                                    .color(egui::Color32::from_gray(90)));
                                ui.end_row();
                            }
                        });
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Tab — close  •  F — use selected")
                        .size(10.0).color(egui::Color32::from_gray(55)));
                });
        });
        if let Some(g) = &mut self.game { g.selected_slot = new_sel; }
        self.paint_egui(out);
    }

    fn render_main_menu(&mut self, _window: &glutin::window::Window) {
        let mut next: Option<GamePhase> = None;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.22);
                    ui.label(egui::RichText::new("THE ROOM").size(54.0).strong().color(egui::Color32::from_gray(220)));
                    ui.label(egui::RichText::new("Oceancrest Hotel").size(13.0).color(egui::Color32::from_gray(55)).italics());
                    ui.add_space(48.0);

                    let btn = |ui: &mut egui::Ui, label: &str| {
                        ui.add_sized([240.0, 40.0], egui::Button::new(
                            egui::RichText::new(label).size(15.0).color(egui::Color32::from_gray(170))))
                    };

                    if btn(ui, "STORY MODE").clicked() {
                        next = Some(GamePhase::CharacterCustomize { mode: GameMode::Story });
                    }
                    ui.add_space(8.0);
                    if btn(ui, "MULTIPLAYER").clicked() {
                        next = Some(GamePhase::CharacterCustomize { mode: GameMode::Multiplayer });
                    }
                    ui.add_space(8.0);
                    if btn(ui, "SETTINGS").clicked() {
                        next = Some(GamePhase::Settings { return_to: Box::new(GamePhase::MainMenu) });
                    }
                    ui.add_space(8.0);
                    if btn(ui, "QUIT").clicked() { std::process::exit(0); }
                });
            });
        });
        self.paint_egui(out);
        if let Some(p) = next { self.phase = p; }
    }

    fn render_character(&mut self, window: &glutin::window::Window, mode: GameMode) {
        let mut char_copy = self.character.clone();
        let mut next: Option<GamePhase> = None;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(28.0);
                    ui.label(egui::RichText::new("CHARACTER").size(26.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(20.0);

                    ui.label(egui::RichText::new("Name").color(egui::Color32::from_gray(110)));
                    ui.add(egui::TextEdit::singleline(&mut char_copy.name).desired_width(260.0));

                    ui.add_space(14.0);
                    ui.label(egui::RichText::new("Skin Tone").color(egui::Color32::from_gray(110)));
                    ui.horizontal(|ui| {
                        for tone in character::SkinTone::all() {
                            let [r, g, b] = tone.rgb();
                            let col = egui::Color32::from_rgb((r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8);
                            let sel = &char_copy.skin_tone == tone;
                            let (resp, painter) = ui.allocate_painter(egui::Vec2::splat(32.0), egui::Sense::click());
                            painter.rect_filled(resp.rect, 2.0, col);
                            painter.rect_stroke(resp.rect, 2.0,
                                if sel { egui::Stroke::new(2.0, egui::Color32::WHITE) }
                                else   { egui::Stroke::new(1.0, egui::Color32::from_gray(40)) });
                            if resp.clicked() { char_copy.skin_tone = tone.clone(); }
                        }
                    });

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Hair Color").color(egui::Color32::from_gray(110)));
                    color_row(ui, &mut char_copy.hair_color);
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Shirt Color").color(egui::Color32::from_gray(110)));
                    color_row(ui, &mut char_copy.shirt_color);
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Pants Color").color(egui::Color32::from_gray(110)));
                    color_row(ui, &mut char_copy.pants_color);

                    ui.add_space(28.0);
                    ui.horizontal(|ui| {
                        ui.add_space(40.0);
                        if ui.add_sized([110.0, 34.0], egui::Button::new("← BACK")).clicked() {
                            next = Some(GamePhase::MainMenu);
                        }
                        ui.add_space(10.0);
                        let lbl = if mode == GameMode::Story { "START →" } else { "NEXT →" };
                        if ui.add_sized([110.0, 34.0], egui::Button::new(lbl)).clicked() {
                            next = Some(match mode {
                                GameMode::Story       => GamePhase::Playing,
                                GameMode::Multiplayer => GamePhase::MultiplayerLobby,
                            });
                        }
                    });
                });
            });
        });

        self.character = char_copy;
        self.paint_egui(out);

        if let Some(GamePhase::Playing) = &next {
            self.character.save();
            let [_, _, vw, vh] = self.game_vp;
            self.game = Some(PlayingState::new(&self.character, &self.settings, vw as u32, vh as u32));
            self.phase = GamePhase::Playing;
            self.inventory_open = false;
            self.capture(window, true);
        } else if let Some(p) = next {
            self.phase = p;
        }
    }

    fn render_lobby(&mut self, window: &glutin::window::Window) {
        let mut host_addr = self.network.host_address.clone();
        let mut join_addr = self.network.join_address.clone();
        let mut next: Option<GamePhase> = None;
        let mut do_host    = false;
        let mut do_connect = false;

        let state       = self.network.state.clone();
        let is_online   = self.network.is_online();
        let players: Vec<String> = self.network.players.iter().map(|p| p.character.name.clone()).collect();

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(egui::RichText::new("MULTIPLAYER").size(26.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(24.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(280.0);
                        ui.label(egui::RichText::new("Host").size(14.0).color(egui::Color32::from_gray(140)));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Address:").color(egui::Color32::from_gray(100)));
                            ui.add(egui::TextEdit::singleline(&mut host_addr).desired_width(170.0));
                        });
                        if ui.add_sized([110.0, 30.0], egui::Button::new("HOST")).clicked() {
                            do_host = true;
                        }
                    });

                    ui.add_space(12.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(280.0);
                        ui.label(egui::RichText::new("Join").size(14.0).color(egui::Color32::from_gray(140)));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Address:").color(egui::Color32::from_gray(100)));
                            ui.add(egui::TextEdit::singleline(&mut join_addr).desired_width(170.0));
                        });
                        if ui.add_sized([110.0, 30.0], egui::Button::new("JOIN")).clicked() {
                            do_connect = true;
                        }
                    });

                    ui.add_space(10.0);
                    let (status_str, status_col) = match &state {
                        network::ConnectionState::Hosting    => ("Hosting", egui::Color32::from_rgb(90,190,90)),
                        network::ConnectionState::Connected  => ("Connected", egui::Color32::from_rgb(90,190,90)),
                        network::ConnectionState::Connecting => ("Connecting…", egui::Color32::from_rgb(190,150,60)),
                        network::ConnectionState::Failed(e)  => (e.as_str(), egui::Color32::from_rgb(190,70,70)),
                        network::ConnectionState::Offline    => ("Offline", egui::Color32::from_gray(70)),
                    };
                    ui.label(egui::RichText::new(status_str).color(status_col));

                    if !players.is_empty() {
                        ui.add_space(8.0);
                        for name in &players {
                            ui.label(egui::RichText::new(name).color(egui::Color32::from_gray(150)));
                        }
                    }

                    ui.add_space(24.0);
                    ui.horizontal(|ui| {
                        ui.add_space(40.0);
                        if ui.add_sized([100.0, 32.0], egui::Button::new("← BACK")).clicked() {
                            next = Some(GamePhase::MainMenu);
                        }
                        if is_online {
                            ui.add_space(10.0);
                            if ui.add_sized([100.0, 32.0], egui::Button::new("START →")).clicked() {
                                next = Some(GamePhase::Playing);
                            }
                        }
                    });
                });
            });
        });

        self.network.host_address = host_addr;
        self.network.join_address = join_addr;
        if do_host    { self.network.host(); }
        if do_connect { self.network.connect(); }
        self.paint_egui(out);

        match next {
            Some(GamePhase::Playing) => {
                let [_, _, vw, vh] = self.game_vp;
                self.game = Some(PlayingState::new(&self.character, &self.settings, vw as u32, vh as u32));
                self.phase = GamePhase::Playing;
                self.capture(window, true);
            }
            Some(GamePhase::MainMenu) => {
                self.network.disconnect();
                self.phase = GamePhase::MainMenu;
            }
            _ => {}
        }
    }

    fn render_pause(&mut self, window: &glutin::window::Window) {
        let mut next: Option<GamePhase> = None;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::Area::new("pause")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(4, 3, 6, 215))
                        .inner_margin(egui::style::Margin::same(32.0))
                        .show(ui, |ui| {
                            ui.set_min_width(240.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("PAUSED").size(26.0).color(egui::Color32::from_gray(200)));
                                ui.add_space(20.0);
                                let btn = |ui: &mut egui::Ui, lbl: &str| {
                                    ui.add_sized([190.0, 34.0], egui::Button::new(
                                        egui::RichText::new(lbl).size(14.0).color(egui::Color32::from_gray(160))))
                                };
                                if btn(ui, "RESUME").clicked()      { next = Some(GamePhase::Playing); }
                                ui.add_space(6.0);
                                if btn(ui, "SETTINGS").clicked()    { next = Some(GamePhase::Settings { return_to: Box::new(GamePhase::Paused) }); }
                                ui.add_space(6.0);
                                if btn(ui, "QUIT TO MENU").clicked() {
                                    self.game = None;
                                    next = Some(GamePhase::MainMenu);
                                }
                            });
                        });
                });
        });
        self.paint_egui(out);

        if let Some(GamePhase::Playing) = &next {
            self.phase = GamePhase::Playing;
            self.capture(window, true);
        } else if let Some(p) = next {
            self.phase = p;
        }
    }

    fn render_settings(&mut self, window: &glutin::window::Window, return_to: GamePhase) {
        let mut s = self.settings.clone();
        let mut back = false;

        let raw = self.take_raw_input();
        let out = self.egui_ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("SETTINGS").size(26.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(28.0);
                    egui::Grid::new("sg").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| {
                        ui.label(egui::RichText::new("Mouse Sensitivity").color(egui::Color32::from_gray(130)));
                        ui.add(egui::Slider::new(&mut s.mouse_sensitivity, 0.0003..=0.005).fixed_decimals(4));
                        ui.end_row();
                        ui.label(egui::RichText::new("Field of View").color(egui::Color32::from_gray(130)));
                        ui.add(egui::Slider::new(&mut s.fov, 50.0..=110.0).suffix("°").fixed_decimals(0));
                        ui.end_row();
                        ui.label(egui::RichText::new("Master Volume").color(egui::Color32::from_gray(130)));
                        ui.add(egui::Slider::new(&mut s.master_volume, 0.0..=1.0).fixed_decimals(2));
                        ui.end_row();
                        ui.label(egui::RichText::new("Fullscreen").color(egui::Color32::from_gray(130)));
                        ui.checkbox(&mut s.fullscreen, "");
                        ui.end_row();
                        ui.label(egui::RichText::new("Show FPS").color(egui::Color32::from_gray(130)));
                        ui.checkbox(&mut s.show_fps, "");
                        ui.end_row();
                    });
                    ui.add_space(28.0);
                    if ui.add_sized([150.0, 34.0], egui::Button::new("← BACK")).clicked() {
                        back = true;
                    }
                });
            });
        });

        self.settings = s;
        self.paint_egui(out);
        if back {
            self.settings.save();
            if let Some(g) = &mut self.game {
                g.audio.set_volume(self.settings.master_volume);
            }
            if self.settings.fullscreen {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
            self.phase = return_to;
        }
    }
}

// ─── Free helpers ─────────────────────────────────────────────────────────────

fn item_color(kind: &ItemKind) -> glm::Vec3 {
    match kind {
        ItemKind::Key { .. }  => glm::vec3(1.00, 0.85, 0.15), // bright gold
        ItemKind::SanityPill  => glm::vec3(0.20, 0.95, 0.45), // vivid green
        ItemKind::WindUpToy   => glm::vec3(0.95, 0.45, 0.10), // punchy orange
        ItemKind::Cd          => glm::vec3(0.65, 0.80, 1.00), // icy blue-white
        ItemKind::CdPlayer    => glm::vec3(0.55, 0.45, 0.30), // warm brown
    }
}

fn item_label(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::Key { .. }  => "Key",
        ItemKind::SanityPill  => "Sanity Pill",
        ItemKind::WindUpToy   => "Wind-Up Toy",
        ItemKind::Cd          => "CD",
        ItemKind::CdPlayer    => "CD Player",
    }
}

fn color_row(ui: &mut egui::Ui, color: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut color[0], 0.0..=1.0).text("R").fixed_decimals(2));
        ui.add(egui::Slider::new(&mut color[1], 0.0..=1.0).text("G").fixed_decimals(2));
        ui.add(egui::Slider::new(&mut color[2], 0.0..=1.0).text("B").fixed_decimals(2));
        let (r, p) = ui.allocate_painter(egui::Vec2::new(24.0, 16.0), egui::Sense::hover());
        p.rect_filled(r.rect, 2.0, egui::Color32::from_rgb(
            (color[0]*255.0) as u8, (color[1]*255.0) as u8, (color[2]*255.0) as u8,
        ));
    });
}

fn set_slot(game: &mut Option<PlayingState>, slot: usize) {
    if let Some(g) = game { g.selected_slot = slot; }
}

fn to_egui_mods(m: ModifiersState) -> egui::Modifiers {
    egui::Modifiers {
        alt:     m.alt(),
        ctrl:    m.ctrl(),
        shift:   m.shift(),
        mac_cmd: cfg!(target_os = "macos") && m.logo(),
        command: if cfg!(target_os = "macos") { m.logo() } else { m.ctrl() },
    }
}

fn vk_to_egui(vk: VirtualKeyCode) -> Option<egui::Key> {
    use VirtualKeyCode::*;
    Some(match vk {
        Down   => egui::Key::ArrowDown,
        Left   => egui::Key::ArrowLeft,
        Right  => egui::Key::ArrowRight,
        Up     => egui::Key::ArrowUp,
        Escape => egui::Key::Escape,
        Tab    => egui::Key::Tab,
        Back   => egui::Key::Backspace,
        Return => egui::Key::Enter,
        Space  => egui::Key::Space,
        Delete => egui::Key::Delete,
        Home   => egui::Key::Home,
        End    => egui::Key::End,
        PageUp   => egui::Key::PageUp,
        PageDown => egui::Key::PageDown,
        Insert => egui::Key::Insert,
        A => egui::Key::A, B => egui::Key::B, C => egui::Key::C,
        D => egui::Key::D, E => egui::Key::E, F => egui::Key::F,
        G => egui::Key::G, H => egui::Key::H, I => egui::Key::I,
        J => egui::Key::J, K => egui::Key::K, L => egui::Key::L,
        M => egui::Key::M, N => egui::Key::N, O => egui::Key::O,
        P => egui::Key::P, Q => egui::Key::Q, R => egui::Key::R,
        S => egui::Key::S, T => egui::Key::T, U => egui::Key::U,
        V => egui::Key::V, W => egui::Key::W, X => egui::Key::X,
        Y => egui::Key::Y, Z => egui::Key::Z,
        Key0 => egui::Key::Num0, Key1 => egui::Key::Num1,
        Key2 => egui::Key::Num2, Key3 => egui::Key::Num3,
        Key4 => egui::Key::Num4, Key5 => egui::Key::Num5,
        Key6 => egui::Key::Num6, Key7 => egui::Key::Num7,
        Key8 => egui::Key::Num8, Key9 => egui::Key::Num9,
        _ => return None,
    })
}

fn build_crosshair_vao() -> (u32, i32) {
    let aspect = 720.0_f32 / 1280.0;
    let hw = 0.018_f32;
    let ht = 0.003_f32;
    let ht_x = ht / aspect;
    #[rustfmt::skip]
    let verts: [f32; 16] = [
        -hw,-ht,  hw,-ht,  hw,ht,  -hw,ht,
        -ht_x,-hw,  ht_x,-hw,  ht_x,hw,  -ht_x,hw,
    ];
    let idx: [u32; 12] = [0,1,2, 0,2,3, 4,5,6, 4,6,7];
    let (mut vao, mut vbo, mut ebo) = (0u32, 0u32, 0u32);
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER, (verts.len()*4) as isize, verts.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (idx.len()*4) as isize, idx.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 8, std::ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);
    }
    (vao, idx.len() as i32)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// Returns [x, y, w, h] for a centered 16:9 sub-viewport within window (w×h).
fn compute_game_viewport(w: u32, h: u32) -> [i32; 4] {
    const ASPECT: f32 = 16.0 / 9.0;
    let actual = w as f32 / h.max(1) as f32;
    if actual > ASPECT {
        // window wider than 16:9 — pillarbox
        let vw = (h as f32 * ASPECT) as i32;
        let vx = (w as i32 - vw) / 2;
        [vx, 0, vw, h as i32]
    } else {
        // window taller than 16:9 — letterbox
        let vh = (w as f32 / ASPECT) as i32;
        let vy = (h as i32 - vh) / 2;
        [0, vy, w as i32, vh]
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut event_loop = EventLoop::new();

    #[cfg(target_os = "macos")]
    event_loop.set_activation_policy(ActivationPolicy::Regular);

    let wb = WindowBuilder::new()
        .with_title("The Room")
        .with_inner_size(glutin::dpi::LogicalSize::new(1280u32, 720u32))
        .with_resizable(true);

    let ctx = ContextBuilder::new()
        .with_depth_buffer(24)
        .with_vsync(true)
        .build_windowed(wb, &event_loop)
        .expect("Failed to create GL context");

    let ctx = unsafe { ctx.make_current().expect("Failed to make current") };
    ctx.window().focus_window();

    gl::load_with(|s| ctx.get_proc_address(s) as *const _);
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);
        gl::CullFace(gl::BACK);
    }

    let glow_ctx = unsafe {
        Arc::new(glow::Context::from_loader_function(|s| ctx.get_proc_address(s) as *const _))
    };

    let mut app = App::new(glow_ctx, ctx.window());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match &event {
            Event::WindowEvent { event: we, .. } => {
                if matches!(we, WindowEvent::CloseRequested) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                app.on_window_event(we, ctx.window());
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) }, ..
            } => {
                app.on_mouse_motion(*dx, *dy);
            }
            Event::MainEventsCleared => {
                app.update();
                app.render(ctx.window());
                ctx.swap_buffers().unwrap();
            }
            _ => {}
        }
    });
}
