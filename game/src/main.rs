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

use egui_glow::EguiGlow;
use glutin::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, ModifiersState, MouseButton,
        MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
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

// ─── Playing state (only exists during gameplay) ─────────────────────────────

struct PlayingState {
    player:      Player,
    world:       World,
    sanity:      Sanity,
    inventory:   Inventory,
    items:       Vec<Item>,
    item_meshes: Vec<Mesh>,
    audio:       AudioManager,
    message:     Option<(String, Instant)>,
    projection:  glm::Mat4,
    selected_slot: usize,
}

impl PlayingState {
    fn new(character: &CharacterConfig, settings: &Settings, w: u32, h: u32) -> Self {
        let player = Player::new(glm::vec3(0.0, world::FLOOR_1, 0.0));
        let world  = World::new();

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

        let s = ITEM_HALF_SIZE;
        let item_meshes = items.iter().map(|item| {
            let color = match &item.kind {
                ItemKind::Key { .. }  => glm::vec3(0.8, 0.7, 0.1),
                ItemKind::SanityPill  => glm::vec3(0.2, 0.8, 0.4),
                ItemKind::WindUpToy   => glm::vec3(0.7, 0.3, 0.1),
                ItemKind::Cd          => glm::vec3(0.6, 0.6, 0.9),
                ItemKind::CdPlayer    => glm::vec3(0.4, 0.4, 0.5),
            };
            let (v, i) = renderer::build_box(
                glm::vec3(-s, -s, -s), glm::vec3(s, s, s),
            );
            Mesh::new(&v, &i, color)
        }).collect();

        let aspect = w as f32 / h as f32;
        let projection = glm::perspective(aspect, settings.fov.to_radians(), 0.05, 200.0);

        Self {
            player,
            world,
            sanity: Sanity::new(),
            inventory: Inventory::default(),
            items,
            item_meshes,
            audio: AudioManager::new(),
            message: None,
            projection,
            selected_slot: 0,
        }
    }

    fn show_message(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), Instant::now()));
    }

    fn try_interact(&mut self) {
        let eye   = self.player.eye_position();
        let range = self.player.interact_range;

        for item in self.items.iter_mut() {
            if item.picked_up { continue; }
            if glm::distance(&eye, &item.position) <= range {
                item.picked_up = true;
                let label = item.label;
                self.inventory.add(item.kind.clone());
                self.show_message(format!("Picked up: {}", label));
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
                    self.show_message("The lock clicks open. Something feels different.");
                } else {
                    self.show_message("Locked. You need a key.");
                }
            } else if !door.open {
                self.world.open_door(door_idx);
                self.show_message("You push the door open.");
            }
        }
    }

    fn use_selected_item(&mut self) {
        let kind = self.inventory.items.get(self.selected_slot).cloned();
        match kind {
            Some(ItemKind::SanityPill) => {
                if self.sanity.use_pill() {
                    self.inventory.items.remove(self.selected_slot);
                    self.show_message("The pill helps. A little.");
                } else {
                    self.show_message("Your mind is as clear as it's going to get.");
                }
            }
            Some(_) => self.show_message("Can't use that right now."),
            None    => self.show_message("Nothing selected."),
        }
        if self.selected_slot >= self.inventory.items.len() && self.selected_slot > 0 {
            self.selected_slot -= 1;
        }
    }

    fn update(&mut self, keys: &HashSet<VirtualKeyCode>, dt: f32) {
        self.sanity.tick(dt);
        self.player.update(keys, dt, &self.world.walls, Some(&self.world.stairs));

        for item in &mut self.items {
            let floor_y = if item.position.y > world::FLOOR_2 - 0.5 {
                world::FLOOR_2
            } else {
                world::FLOOR_1
            };
            item.physics_tick(dt, floor_y);
        }

        // Clear stale message after 3 seconds
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

    // Rendering
    shader:      Shader,
    cross_shader: Shader,
    cross_vao:   u32,
    cross_count: i32,
    egui_glow:   EguiGlow,

    // Input
    keys:           HashSet<VirtualKeyCode>,
    mouse_captured: bool,
    inventory_open: bool,
    modifiers:      ModifiersState,
    mouse_pos:      egui::Pos2,
    window_size:    [u32; 2],
    last:           Instant,
}

impl App {
    fn new<E>(glow_ctx: Arc<glow::Context>, event_loop: &glutin::event_loop::EventLoopWindowTarget<E>) -> Self {
        let shader       = Shader::new(VERT_SRC, FRAG_SRC);
        let cross_shader = Shader::new(CROSS_VERT_SRC, CROSS_FRAG_SRC);
        let (cross_vao, cross_count) = build_crosshair_vao();

        let egui_glow = EguiGlow::new(event_loop, glow_ctx.clone(), None);

        // Dark horror theme
        let mut vis = egui::Visuals::dark();
        vis.window_fill        = egui::Color32::from_rgba_premultiplied(6, 5, 8, 230);
        vis.panel_fill         = egui::Color32::from_rgba_premultiplied(6, 5, 8, 230);
        vis.window_rounding    = egui::Rounding::none();
        vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(18, 15, 20);
        vis.widgets.hovered.bg_fill  = egui::Color32::from_rgb(45, 35, 50);
        vis.widgets.active.bg_fill   = egui::Color32::from_rgb(70, 50, 80);
        egui_glow.egui_ctx.set_visuals(vis);

        let size = window.inner_size();

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
            egui_glow,

            keys:           HashSet::new(),
            mouse_captured: false,
            inventory_open: false,
            modifiers:      ModifiersState::empty(),
            mouse_pos:      egui::Pos2::ZERO,
            window_size:    [size.width, size.height],
            last:           Instant::now(),
        }
    }

    fn capture_mouse(&mut self, window: &glutin::window::Window, capture: bool) {
        self.mouse_captured = capture;
        window.set_cursor_grab(capture).ok();
        window.set_cursor_visible(!capture);
    }

    fn start_game(&mut self, mode: GameMode, window: &glutin::window::Window) {
        let [w, h] = self.window_size;
        self.game = Some(PlayingState::new(&self.character, &self.settings, w, h));
        self.phase = GamePhase::Playing;
        self.inventory_open = false;
        self.capture_mouse(window, true);
        // In multiplayer mode, tell the network manager we're joining
        if mode == GameMode::Multiplayer {
            self.network.send(network::ClientMsg::Join {
                id: self.network.local_id.clone(),
                character: self.character.clone(),
            });
        }
    }

    fn handle_window_event(&mut self, event: &WindowEvent, window: &glutin::window::Window) -> bool {
        // Feed egui when mouse is free
        if !self.mouse_captured || self.inventory_open {
            let consumed = self.egui_glow.on_event(event);
            if consumed { return true; }
        }

        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = egui::pos2(position.x as f32, position.y as f32);
            }
            WindowEvent::Resized(size) => {
                self.window_size = [size.width, size.height];
                if let Some(g) = &mut self.game {
                    let aspect = size.width as f32 / size.height as f32;
                    g.projection = glm::perspective(aspect, self.settings.fov.to_radians(), 0.05, 200.0);
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
                    self.handle_key_press(*key, window);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed, button: MouseButton::Left, ..
            } => {
                if self.phase == GamePhase::Playing && !self.inventory_open {
                    self.capture_mouse(window, true);
                }
            }
            WindowEvent::Focused(true) => {
                if self.phase == GamePhase::Playing && !self.inventory_open {
                    self.capture_mouse(window, true);
                }
            }
            _ => {}
        }
        false
    }

    fn handle_key_press(&mut self, key: VirtualKeyCode, window: &glutin::window::Window) {
        match &self.phase.clone() {
            GamePhase::Playing => match key {
                VirtualKeyCode::Escape => {
                    self.capture_mouse(window, false);
                    self.inventory_open = false;
                    self.phase = GamePhase::Paused;
                }
                VirtualKeyCode::Tab => {
                    self.inventory_open = !self.inventory_open;
                    self.capture_mouse(window, !self.inventory_open);
                }
                VirtualKeyCode::E => {
                    if let Some(g) = &mut self.game { g.try_interact(); }
                }
                VirtualKeyCode::F => {
                    if let Some(g) = &mut self.game { g.use_selected_item(); }
                }
                VirtualKeyCode::Key1 => { if let Some(g) = &mut self.game { g.selected_slot = 0; } }
                VirtualKeyCode::Key2 => { if let Some(g) = &mut self.game { g.selected_slot = 1; } }
                VirtualKeyCode::Key3 => { if let Some(g) = &mut self.game { g.selected_slot = 2; } }
                VirtualKeyCode::Key4 => { if let Some(g) = &mut self.game { g.selected_slot = 3; } }
                VirtualKeyCode::Key5 => { if let Some(g) = &mut self.game { g.selected_slot = 4; } }
                _ => {}
            },
            GamePhase::Paused => {
                if key == VirtualKeyCode::Escape {
                    self.phase = GamePhase::Playing;
                    self.capture_mouse(window, true);
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.mouse_captured && !self.inventory_open {
            if let Some(g) = &mut self.game {
                g.player.apply_mouse(dx as f32, dy as f32);
            }
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt  = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;

        self.network.tick();

        if self.phase == GamePhase::Playing {
            let keys = self.keys.clone();
            if let Some(g) = &mut self.game {
                g.update(&keys, dt);
            }
        }
    }

    fn render(&mut self, window: &glutin::window::Window) {
        let [w, h] = self.window_size;

        unsafe {
            gl::Viewport(0, 0, w as i32, h as i32);
            gl::ClearColor(0.02, 0.02, 0.025, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        match &self.phase.clone() {
            GamePhase::Playing => self.render_game(window),
            GamePhase::Paused  => {
                self.render_game_background();
                self.render_ui_pause(window);
            }
            GamePhase::MainMenu => self.render_ui_main_menu(window),
            GamePhase::CharacterCustomize { mode } => {
                let mode = mode.clone();
                self.render_ui_character(window, &mode);
            }
            GamePhase::MultiplayerLobby => self.render_ui_lobby(window),
            GamePhase::Settings { return_to } => {
                let ret = *return_to.clone();
                self.render_ui_settings(window, ret);
            }
        }
    }

    // ── 3D game rendering ───────────────────────────────────────────────────

    fn render_game(&mut self, window: &glutin::window::Window) {
        self.render_game_background();

        // Crosshair (only when not in inventory)
        if !self.inventory_open {
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
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            }
        }

        // Inventory / HUD overlay
        if self.inventory_open {
            self.render_ui_inventory(window);
        } else {
            // Minimal HUD — sanity bar + message + hint
            self.render_ui_hud(window);
        }
    }

    fn render_game_background(&mut self) {
        let g = match &self.game { Some(g) => g, None => return };

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
        }

        self.shader.use_program();
        self.shader.set_mat4("projection", &g.projection);
        self.shader.set_mat4("view",       &g.player.view_matrix());
        self.shader.set_vec3("lightPos",   &glm::vec3(0.0_f32, 2.5, 0.0));
        self.shader.set_vec3("lightColor", &glm::vec3(1.0_f32, 0.95, 0.85));
        self.shader.set_float("ambientStrength", lerp(0.25, 0.55, g.sanity.normalised()));
        self.shader.set_float("sanity",    g.sanity.normalised());

        let identity: glm::Mat4 = glm::identity();

        for room in &g.world.rooms {
            self.shader.set_mat4("model", &identity);
            self.shader.set_vec3("objectColor", &room.color);
            room.mesh.draw();
        }
        for dm in &g.world.door_meshes {
            self.shader.set_mat4("model", &identity);
            self.shader.set_vec3("objectColor", &dm.mesh.color);
            dm.mesh.draw();
        }
        for (i, item) in g.items.iter().enumerate() {
            if item.picked_up { continue; }
            let p = item.position;
            let model = glm::translation(&p);
            self.shader.set_mat4("model", &model);
            self.shader.set_vec3("objectColor", &g.item_meshes[i].color);
            g.item_meshes[i].draw();
        }
    }

    // ── egui UI panels ──────────────────────────────────────────────────────

    fn render_ui_hud(&mut self, window: &glutin::window::Window) {
        let sanity    = self.game.as_ref().map(|g| g.sanity.normalised()).unwrap_or(1.0);
        let message   = self.game.as_ref().and_then(|g| g.message.as_ref()).map(|(m, _)| m.clone());
        let items     = self.game.as_ref().map(|g| g.inventory.summary()).unwrap_or_default();
        let selected  = self.game.as_ref().map(|g| g.selected_slot).unwrap_or(0);
        let inv_count = self.game.as_ref().map(|g| g.inventory.items.len()).unwrap_or(0);

        self.egui_glow.run(window, |ctx| {
            // Sanity bar — bottom center
            egui::Area::new("sanity")
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0])
                .show(ctx, |ui| {
                    ui.visuals_mut().extreme_bg_color = egui::Color32::from_rgb(10, 10, 12);
                    let sanity_color = if sanity > 0.5 {
                        egui::Color32::from_rgb(100, 180, 220)
                    } else if sanity > 0.25 {
                        egui::Color32::from_rgb(180, 130, 60)
                    } else {
                        egui::Color32::from_rgb(180, 50, 50)
                    };
                    ui.label(egui::RichText::new("SANITY").size(10.0).color(egui::Color32::from_gray(80)));
                    let bar = egui::ProgressBar::new(sanity)
                        .desired_width(200.0)
                        .fill(sanity_color);
                    ui.add(bar);
                });

            // Item hotbar — bottom center above sanity
            egui::Area::new("hotbar")
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -72.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        for i in 0..5 {
                            let label = self.game.as_ref()
                                .and_then(|g| g.inventory.items.get(i))
                                .map(item_label)
                                .unwrap_or("·");
                            let is_sel = i == selected;
                            let color = if is_sel {
                                egui::Color32::from_rgb(200, 170, 100)
                            } else {
                                egui::Color32::from_gray(60)
                            };
                            let frame = egui::Frame::none()
                                .fill(egui::Color32::from_rgba_premultiplied(8, 6, 10, 180))
                                .stroke(egui::Stroke::new(
                                    if is_sel { 1.5 } else { 0.5 },
                                    color,
                                ))
                                .inner_margin(egui::style::Margin::symmetric(6.0, 4.0));
                            frame.show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(label)
                                        .size(11.0)
                                        .color(color)
                                        .monospace(),
                                );
                            });
                        }
                    });
                });

            // Message — above hotbar
            if let Some(msg) = message {
                egui::Area::new("message")
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -120.0])
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(&msg)
                                .size(13.0)
                                .color(egui::Color32::from_gray(180)),
                        );
                    });
            }

            // Controls hint — bottom right
            egui::Area::new("hints")
                .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("E Interact  F Use  Tab Inventory  Esc Pause")
                            .size(10.0)
                            .color(egui::Color32::from_gray(40)),
                    );
                });
        });
        self.restore_gl_state();
    }

    fn render_ui_inventory(&mut self, window: &glutin::window::Window) {
        let inv_items: Vec<(String, usize)> = self.game.as_ref()
            .map(|g| {
                let mut counts: std::collections::HashMap<String, usize> = Default::default();
                for k in &g.inventory.items {
                    *counts.entry(item_label(k).to_string()).or_default() += 1;
                }
                let mut v: Vec<_> = counts.into_iter().collect();
                v.sort();
                v
            })
            .unwrap_or_default();

        let selected = self.game.as_ref().map(|g| g.selected_slot).unwrap_or(0);

        self.egui_glow.run(window, |ctx| {
            egui::Window::new("INVENTORY")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.set_min_width(320.0);
                    if inv_items.is_empty() {
                        ui.label(egui::RichText::new("Empty").color(egui::Color32::from_gray(80)));
                    } else {
                        egui::Grid::new("inv_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                for (i, (label, count)) in inv_items.iter().enumerate() {
                                    let is_sel = i == selected;
                                    let color = if is_sel {
                                        egui::Color32::from_rgb(220, 190, 100)
                                    } else {
                                        egui::Color32::from_gray(160)
                                    };
                                    if ui.selectable_label(is_sel,
                                        egui::RichText::new(label).color(color)
                                    ).clicked() {
                                        if let Some(g) = &mut self.game { g.selected_slot = i; }
                                    }
                                    ui.label(
                                        egui::RichText::new(format!("×{}", count))
                                            .color(egui::Color32::from_gray(100))
                                    );
                                    ui.end_row();
                                }
                            });
                    }
                    ui.separator();
                    ui.label(egui::RichText::new("Tab — close  •  F — use selected")
                        .size(10.0).color(egui::Color32::from_gray(60)));
                });
        });
        self.restore_gl_state();
    }

    fn render_ui_main_menu(&mut self, window: &glutin::window::Window) {
        let mut next: Option<GamePhase> = None;

        self.egui_glow.run(window, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);

                    ui.label(egui::RichText::new("THE ROOM")
                        .size(56.0)
                        .color(egui::Color32::from_gray(220))
                        .strong());

                    ui.label(egui::RichText::new("Oceancrest Hotel")
                        .size(14.0)
                        .color(egui::Color32::from_gray(60))
                        .italics());

                    ui.add_space(48.0);

                    let btn = |ui: &mut egui::Ui, label: &str| {
                        ui.add_sized(
                            [240.0, 40.0],
                            egui::Button::new(
                                egui::RichText::new(label)
                                    .size(16.0)
                                    .color(egui::Color32::from_gray(180))
                            )
                        )
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
                    if btn(ui, "QUIT").clicked() {
                        std::process::exit(0);
                    }
                });
            });
        });
        self.restore_gl_state();
        if let Some(p) = next { self.phase = p; }
    }

    fn render_ui_character(&mut self, window: &glutin::window::Window, mode: &GameMode) {
        let mode = mode.clone();
        let mut next: Option<GamePhase> = None;

        self.egui_glow.run(window, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(egui::RichText::new("CHARACTER").size(28.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(24.0);

                    ui.label(egui::RichText::new("Name").color(egui::Color32::from_gray(120)));
                    ui.add(egui::TextEdit::singleline(&mut self.character.name)
                        .desired_width(260.0)
                        .text_color(egui::Color32::from_gray(200)));

                    ui.add_space(16.0);

                    // Skin tone selector
                    ui.label(egui::RichText::new("Skin Tone").color(egui::Color32::from_gray(120)));
                    ui.horizontal(|ui| {
                        for tone in character::SkinTone::all() {
                            let [r, g, b] = tone.rgb();
                            let col = egui::Color32::from_rgb(
                                (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8,
                            );
                            let selected = &self.character.skin_tone == tone;
                            let stroke = if selected {
                                egui::Stroke::new(2.0, egui::Color32::WHITE)
                            } else {
                                egui::Stroke::new(1.0, egui::Color32::from_gray(50))
                            };
                            let (resp, painter) = ui.allocate_painter(
                                egui::Vec2::splat(32.0), egui::Sense::click(),
                            );
                            painter.rect_filled(resp.rect, 2.0, col);
                            painter.rect_stroke(resp.rect, 2.0, stroke);
                            if resp.clicked() {
                                self.character.skin_tone = tone.clone();
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Hair color
                    ui.label(egui::RichText::new("Hair Color").color(egui::Color32::from_gray(120)));
                    color_edit_row(ui, &mut self.character.hair_color);

                    ui.add_space(8.0);

                    // Shirt color
                    ui.label(egui::RichText::new("Shirt Color").color(egui::Color32::from_gray(120)));
                    color_edit_row(ui, &mut self.character.shirt_color);

                    ui.add_space(8.0);

                    // Pants color
                    ui.label(egui::RichText::new("Pants Color").color(egui::Color32::from_gray(120)));
                    color_edit_row(ui, &mut self.character.pants_color);

                    ui.add_space(32.0);

                    ui.horizontal(|ui| {
                        ui.add_space(60.0);
                        if ui.add_sized([110.0, 36.0], egui::Button::new("← BACK")).clicked() {
                            next = Some(GamePhase::MainMenu);
                        }
                        ui.add_space(12.0);
                        let label = if mode == GameMode::Story { "START →" } else { "NEXT →" };
                        if ui.add_sized([110.0, 36.0], egui::Button::new(label)).clicked() {
                            self.character.save();
                            next = Some(match mode {
                                GameMode::Story       => GamePhase::Playing, // will be handled below
                                GameMode::Multiplayer => GamePhase::MultiplayerLobby,
                            });
                        }
                    });
                });
            });
        });
        self.restore_gl_state();

        if let Some(p) = next {
            match &p {
                GamePhase::Playing => {
                    // start_game needs window — signal via phase first
                    self.phase = GamePhase::Playing;
                    let [w, h] = self.window_size;
                    self.game = Some(PlayingState::new(&self.character, &self.settings, w, h));
                    self.inventory_open = false;
                    window.set_cursor_grab(true).ok();
                    window.set_cursor_visible(false);
                    self.mouse_captured = true;
                }
                _ => { self.phase = p; }
            }
        }
    }

    fn render_ui_lobby(&mut self, window: &glutin::window::Window) {
        let mut next: Option<GamePhase> = None;

        self.egui_glow.run(window, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(egui::RichText::new("MULTIPLAYER").size(28.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(32.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.label(egui::RichText::new("Host a Game").size(16.0).color(egui::Color32::from_gray(160)));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Address:").color(egui::Color32::from_gray(100)));
                            ui.add(egui::TextEdit::singleline(&mut self.network.host_address).desired_width(180.0));
                        });
                        if ui.add_sized([120.0, 32.0], egui::Button::new("HOST")).clicked() {
                            self.network.host();
                        }
                    });

                    ui.add_space(16.0);

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.label(egui::RichText::new("Join a Game").size(16.0).color(egui::Color32::from_gray(160)));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Address:").color(egui::Color32::from_gray(100)));
                            ui.add(egui::TextEdit::singleline(&mut self.network.join_address).desired_width(180.0));
                        });
                        if ui.add_sized([120.0, 32.0], egui::Button::new("JOIN")).clicked() {
                            self.network.connect();
                        }
                    });

                    ui.add_space(16.0);

                    // Status
                    let status = format!("{:?}", self.network.state);
                    let color = match self.network.state {
                        network::ConnectionState::Hosting | network::ConnectionState::Connected
                            => egui::Color32::from_rgb(100, 200, 100),
                        network::ConnectionState::Connecting
                            => egui::Color32::from_rgb(200, 160, 60),
                        network::ConnectionState::Failed(_)
                            => egui::Color32::from_rgb(200, 80, 80),
                        _   => egui::Color32::from_gray(80),
                    };
                    ui.label(egui::RichText::new(status).color(color));

                    // Player list
                    if !self.network.players.is_empty() {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Players:").color(egui::Color32::from_gray(120)));
                        for p in &self.network.players {
                            ui.label(egui::RichText::new(&p.character.name).color(egui::Color32::from_gray(160)));
                        }
                    }

                    ui.add_space(32.0);
                    ui.horizontal(|ui| {
                        ui.add_space(60.0);
                        if ui.add_sized([100.0, 32.0], egui::Button::new("← BACK")).clicked() {
                            self.network.disconnect();
                            next = Some(GamePhase::MainMenu);
                        }
                        if self.network.is_online() {
                            ui.add_space(12.0);
                            if ui.add_sized([100.0, 32.0], egui::Button::new("START →")).clicked() {
                                next = Some(GamePhase::Playing);
                            }
                        }
                    });
                });
            });
        });
        self.restore_gl_state();

        if let Some(GamePhase::Playing) = &next {
            let [w, h] = self.window_size;
            self.game = Some(PlayingState::new(&self.character, &self.settings, w, h));
            self.phase = GamePhase::Playing;
            window.set_cursor_grab(true).ok();
            window.set_cursor_visible(false);
            self.mouse_captured = true;
        } else if let Some(p) = next {
            self.phase = p;
        }
    }

    fn render_ui_pause(&mut self, window: &glutin::window::Window) {
        let mut next: Option<GamePhase> = None;

        self.egui_glow.run(window, |ctx| {
            egui::Area::new("pause_overlay")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_premultiplied(4, 3, 6, 210))
                        .inner_margin(egui::style::Margin::same(32.0))
                        .show(ui, |ui| {
                            ui.set_min_width(260.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("PAUSED").size(28.0).color(egui::Color32::from_gray(200)));
                                ui.add_space(24.0);

                                let btn = |ui: &mut egui::Ui, label: &str| {
                                    ui.add_sized([200.0, 36.0], egui::Button::new(
                                        egui::RichText::new(label).size(14.0).color(egui::Color32::from_gray(170))
                                    ))
                                };

                                if btn(ui, "RESUME").clicked() {
                                    next = Some(GamePhase::Playing);
                                }
                                ui.add_space(6.0);
                                if btn(ui, "SETTINGS").clicked() {
                                    next = Some(GamePhase::Settings { return_to: Box::new(GamePhase::Paused) });
                                }
                                ui.add_space(6.0);
                                if btn(ui, "QUIT TO MENU").clicked() {
                                    self.game = None;
                                    next = Some(GamePhase::MainMenu);
                                }
                            });
                        });
                });
        });
        self.restore_gl_state();

        if let Some(GamePhase::Playing) = &next {
            self.phase = GamePhase::Playing;
            window.set_cursor_grab(true).ok();
            window.set_cursor_visible(false);
            self.mouse_captured = true;
        } else if let Some(p) = next {
            self.phase = p;
        }
    }

    fn render_ui_settings(&mut self, window: &glutin::window::Window, return_to: GamePhase) {
        let mut go_back = false;

        self.egui_glow.run(window, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("SETTINGS").size(28.0).color(egui::Color32::from_gray(200)));
                    ui.add_space(32.0);

                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([24.0, 12.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Mouse Sensitivity").color(egui::Color32::from_gray(140)));
                            ui.add(egui::Slider::new(&mut self.settings.mouse_sensitivity, 0.0003..=0.005)
                                .fixed_decimals(4));
                            ui.end_row();

                            ui.label(egui::RichText::new("Field of View").color(egui::Color32::from_gray(140)));
                            ui.add(egui::Slider::new(&mut self.settings.fov, 50.0..=110.0)
                                .suffix("°").fixed_decimals(0));
                            ui.end_row();

                            ui.label(egui::RichText::new("Master Volume").color(egui::Color32::from_gray(140)));
                            ui.add(egui::Slider::new(&mut self.settings.master_volume, 0.0..=1.0)
                                .fixed_decimals(2));
                            ui.end_row();

                            ui.label(egui::RichText::new("Show FPS").color(egui::Color32::from_gray(140)));
                            ui.checkbox(&mut self.settings.show_fps, "");
                            ui.end_row();
                        });

                    ui.add_space(32.0);

                    if ui.add_sized([160.0, 36.0], egui::Button::new("← BACK")).clicked() {
                        self.settings.save();
                        go_back = true;
                    }
                });
            });
        });
        self.restore_gl_state();

        if go_back { self.phase = return_to; }
    }

    fn restore_gl_state(&self) {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Disable(gl::BLEND);
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn item_label(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::Key { .. }  => "Key",
        ItemKind::SanityPill  => "Sanity Pill",
        ItemKind::WindUpToy   => "Wind-Up Toy",
        ItemKind::Cd          => "CD",
        ItemKind::CdPlayer    => "CD Player",
    }
}

fn color_edit_row(ui: &mut egui::Ui, color: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut color[0], 0.0..=1.0).text("R").fixed_decimals(2));
        ui.add(egui::Slider::new(&mut color[1], 0.0..=1.0).text("G").fixed_decimals(2));
        ui.add(egui::Slider::new(&mut color[2], 0.0..=1.0).text("B").fixed_decimals(2));
        let col = egui::Color32::from_rgb(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
        );
        let (r, p) = ui.allocate_painter(egui::Vec2::new(28.0, 18.0), egui::Sense::hover());
        p.rect_filled(r.rect, 3.0, col);
    });
}

fn build_crosshair_vao() -> (u32, i32) {
    let aspect = 720.0_f32 / 1280.0;
    let hw = 0.018_f32;
    let ht = 0.003_f32;
    let ht_x = ht / aspect;

    #[rustfmt::skip]
    let verts: [f32; 16] = [
        -hw, -ht,  hw, -ht,  hw, ht,  -hw, ht,
        -ht_x, -hw,  ht_x, -hw,  ht_x, hw,  -ht_x, hw,
    ];
    let indices: [u32; 12] = [0,1,2, 0,2,3, 4,5,6, 4,6,7];

    let (mut vao, mut vbo, mut ebo) = (0u32, 0u32, 0u32);
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER, (verts.len() * 4) as isize, verts.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (indices.len() * 4) as isize, indices.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 8, std::ptr::null());
        gl::EnableVertexAttribArray(0);
        gl::BindVertexArray(0);
    }
    (vao, indices.len() as i32)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

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

    // We need the EventLoopWindowTarget for EguiGlow — capture it on first event.
    // Use a temporary workaround: build App inside the event loop on first iteration.
    let mut app: Option<App> = None;

    event_loop.run(move |event, event_loop_target, control_flow| {
        *control_flow = ControlFlow::Poll;

        // Initialise App on first event so we have EventLoopWindowTarget.
        let app = app.get_or_insert_with(|| App::new(glow_ctx.clone(), event_loop_target));

        match &event {
            Event::WindowEvent { event: we, .. } => {
                if matches!(we, WindowEvent::CloseRequested) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                app.handle_window_event(we, ctx.window());
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (dx, dy) }, ..
            } => {
                app.handle_mouse_motion(*dx, *dy);
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
