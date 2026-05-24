/// Developer/editor overlay — toggle with F2.
///
/// Provides:
///   - Free-fly camera (no collision when active)
///   - Item position editor
///   - Room color editor
///   - Script console (run Lua one-liners)
///   - Save layout to JSON
///   - Reload scripts

use egui;
use serde_json;

#[derive(Default)]
pub struct DevMode {
    pub active:    bool,
    pub fly_cam:   bool,
    pub tab:       DevTab,
    pub console_input: String,
    pub console_log:   Vec<String>,
    pub selected_item: Option<usize>,
    pub selected_room: Option<usize>,
    pending_cmds:      Vec<DevCmd>,
}

impl DevMode {
    pub fn drain_cmds(&mut self) -> Vec<DevCmd> {
        std::mem::take(&mut self.pending_cmds)
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum DevTab {
    #[default]
    Items,
    Rooms,
    Scripts,
    Info,
}

/// Minimal item/room data we need for the editor UI (no OpenGL types).
pub struct ItemInfo {
    pub label: String,
    pub picked_up: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct RoomInfo {
    pub id:   usize,
    pub name: String,
    pub r:    f32,
    pub g:    f32,
    pub b:    f32,
}

/// Commands the DevMode UI wants to send back to main.
pub enum DevCmd {
    MoveItem  { idx: usize, x: f32, y: f32, z: f32 },
    SetRoomColor { idx: usize, r: f32, g: f32, b: f32 },
    SpawnItem { kind: String, x: f32, y: f32, z: f32 },
    DeleteItem(usize),
    RunScript(String),
    ReloadScripts,
    SaveLayout,
    ToggleFly,
}

impl DevMode {
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    /// Render the dev overlay. Commands are stored internally — call drain_cmds() after.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        items: &[ItemInfo],
        rooms: &[RoomInfo],
        scripts: &[String],
        player_pos: (f32, f32, f32),
    ) {
        if !self.active { return; }

        egui::Window::new("☢ DEV MODE")
            .default_pos([10.0, 10.0])
            .default_size([380.0, 520.0])
            .resizable(true)
            .show(ctx, |ui| {
                // ── Tabs ──────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, DevTab::Items,   "Items");
                    ui.selectable_value(&mut self.tab, DevTab::Rooms,   "Rooms");
                    ui.selectable_value(&mut self.tab, DevTab::Scripts, "Scripts");
                    ui.selectable_value(&mut self.tab, DevTab::Info,    "Info");
                    ui.separator();
                    let fly_label = if self.fly_cam { "Fly ON" } else { "Fly OFF" };
                    if ui.button(fly_label).clicked() {
                        self.pending_cmds.push(DevCmd::ToggleFly);
                        self.fly_cam = !self.fly_cam;
                    }
                });
                ui.separator();

                match self.tab {
                    DevTab::Items   => self.tab_items(ui, items),
                    DevTab::Rooms   => self.tab_rooms(ui, rooms),
                    DevTab::Scripts => self.tab_scripts(ui, scripts),
                    DevTab::Info    => self.tab_info(ui, player_pos, items, rooms),
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save Layout").clicked() { self.pending_cmds.push(DevCmd::SaveLayout); }
                    if ui.button("Reload Scripts").clicked() { self.pending_cmds.push(DevCmd::ReloadScripts); }
                });
            });
    }

    fn tab_items(&mut self, ui: &mut egui::Ui, items: &[ItemInfo]) {
        ui.label(egui::RichText::new("Items in world").color(egui::Color32::from_gray(150)).size(11.0));
        ui.add_space(4.0);

        egui::CollapsingHeader::new("Spawn Item").show(ui, |ui| {
            ui.horizontal(|ui| {
                for kind in &["SanityPill", "WindUpToy", "Cd"] {
                    if ui.small_button(*kind).clicked() {
                        self.pending_cmds.push(DevCmd::SpawnItem {
                            kind: kind.to_string(),
                            x: 0.0, y: 1.0, z: 0.0,
                        });
                    }
                }
            });
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                let picked = if item.picked_up { " [bagged]" } else { "" };
                let header = format!("#{} - {}{}", i, item.label, picked);
                let is_sel = self.selected_item == Some(i);
                if ui.selectable_label(is_sel, &header).clicked() {
                    self.selected_item = if is_sel { None } else { Some(i) };
                }
                if self.selected_item == Some(i) {
                    let mut x = item.x;
                    let mut y = item.y;
                    let mut z = item.z;
                    let mut changed = false;
                    ui.indent("item_edit", |ui| {
                        changed |= ui.add(egui::Slider::new(&mut x, -20.0..=20.0).text("X").fixed_decimals(2)).changed();
                        changed |= ui.add(egui::Slider::new(&mut y, -1.0..=10.0).text("Y").fixed_decimals(2)).changed();
                        changed |= ui.add(egui::Slider::new(&mut z, -20.0..=20.0).text("Z").fixed_decimals(2)).changed();
                        if ui.small_button("Delete").clicked() {
                            self.pending_cmds.push(DevCmd::DeleteItem(i));
                            self.selected_item = None;
                        }
                    });
                    if changed {
                        self.pending_cmds.push(DevCmd::MoveItem { idx: i, x, y, z });
                    }
                }
            }
        });
    }

    fn tab_rooms(&mut self, ui: &mut egui::Ui, rooms: &[RoomInfo]) {
        ui.label(egui::RichText::new("Room colors").color(egui::Color32::from_gray(150)).size(11.0));
        ui.add_space(4.0);

        egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
            for (i, room) in rooms.iter().enumerate() {
                let is_sel = self.selected_room == Some(i);
                let swatch = egui::Color32::from_rgb(
                    (room.r * 255.0) as u8,
                    (room.g * 255.0) as u8,
                    (room.b * 255.0) as u8,
                );
                ui.horizontal(|ui| {
                    let (resp, painter) = ui.allocate_painter(egui::Vec2::splat(16.0), egui::Sense::hover());
                    painter.rect_filled(resp.rect, 2.0, swatch);
                    if ui.selectable_label(is_sel, &room.name).clicked() {
                        self.selected_room = if is_sel { None } else { Some(i) };
                    }
                });
                if self.selected_room == Some(i) {
                    let mut r = room.r;
                    let mut g = room.g;
                    let mut b = room.b;
                    let mut changed = false;
                    ui.indent("room_edit", |ui| {
                        changed |= ui.add(egui::Slider::new(&mut r, 0.0..=1.0).text("R").fixed_decimals(2)).changed();
                        changed |= ui.add(egui::Slider::new(&mut g, 0.0..=1.0).text("G").fixed_decimals(2)).changed();
                        changed |= ui.add(egui::Slider::new(&mut b, 0.0..=1.0).text("B").fixed_decimals(2)).changed();
                    });
                    if changed {
                        self.pending_cmds.push(DevCmd::SetRoomColor { idx: room.id, r, g, b });
                    }
                }
            }
        });
    }

    fn tab_scripts(&mut self, ui: &mut egui::Ui, scripts: &[String]) {
        ui.label(egui::RichText::new("Loaded scripts:").color(egui::Color32::from_gray(150)).size(11.0));
        for s in scripts {
            ui.label(egui::RichText::new(s).size(10.0).color(egui::Color32::from_gray(120)).monospace());
        }
        if scripts.is_empty() {
            ui.label(egui::RichText::new("(none - put .lua files in scripts/)").size(10.0).color(egui::Color32::from_gray(70)));
        }
        ui.separator();
        ui.label(egui::RichText::new("Console (Lua):").color(egui::Color32::from_gray(150)).size(11.0));

        egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).show(ui, |ui| {
            for line in &self.console_log {
                ui.label(egui::RichText::new(line).size(10.0).monospace().color(egui::Color32::from_gray(160)));
            }
        });

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.console_input)
                    .desired_width(280.0)
                    .hint_text("Lua expression...")
                    .font(egui::TextStyle::Monospace)
            );
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if submitted || ui.button("Run").clicked() {
                let cmd = self.console_input.trim().to_string();
                if !cmd.is_empty() {
                    self.console_log.push(format!("> {}", cmd));
                    self.pending_cmds.push(DevCmd::RunScript(cmd));
                    self.console_input.clear();
                }
                response.request_focus();
            }
        });
    }

    fn tab_info(&self, ui: &mut egui::Ui, pos: (f32, f32, f32), items: &[ItemInfo], rooms: &[RoomInfo]) {
        ui.label(format!("Player: ({:.2}, {:.2}, {:.2})", pos.0, pos.1, pos.2));
        ui.label(format!("Items: {}  (picked up: {})", items.len(), items.iter().filter(|i| i.picked_up).count()));
        ui.label(format!("Rooms: {}", rooms.len()));
        ui.separator();
        ui.label(egui::RichText::new("Keybindings").strong());
        for (k, v) in &[
            ("F2",        "Toggle dev mode"),
            ("G",         "Grab item"),
            ("R",         "Throw held item"),
            ("E",         "Interact / bag held item"),
            ("F",         "Use selected in hotbar"),
            ("Tab",       "Inventory"),
            ("Space",     "Jump"),
            ("Esc",       "Pause"),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(*k).monospace().color(egui::Color32::from_rgb(200, 180, 100)));
                ui.label(*v);
            });
        }
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        self.console_log.push(s);
        if self.console_log.len() > 200 {
            self.console_log.remove(0);
        }
    }

    /// Serialize item positions and room colors to JSON for saving.
    pub fn layout_to_json(items: &[ItemInfo], rooms: &[RoomInfo]) -> String {
        let items_json: Vec<_> = items.iter().map(|it| {
            serde_json::json!({
                "label": it.label,
                "x": it.x, "y": it.y, "z": it.z,
            })
        }).collect();
        let rooms_json: Vec<_> = rooms.iter().map(|r| {
            serde_json::json!({
                "id": r.id, "name": r.name,
                "r": r.r, "g": r.g, "b": r.b,
            })
        }).collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "items": items_json,
            "rooms": rooms_json,
        })).unwrap_or_default()
    }
}
