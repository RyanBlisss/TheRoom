use nalgebra_glm as glm;

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Key { room_id: usize },
    SanityPill,
    WindUpToy,
    Cd,
    CdPlayer,
}

pub const ITEM_HALF_SIZE: f32 = 0.12;
const GRAVITY: f32 = -9.81;

#[derive(Debug, Clone)]
pub struct Item {
    pub kind:      ItemKind,
    pub position:  glm::Vec3,
    pub picked_up: bool,
    pub label:     &'static str,
    pub vel_y:     f32,
    pub landed:    bool,
}

impl Item {
    fn make(kind: ItemKind, pos: glm::Vec3, label: &'static str) -> Self {
        Self { kind, position: pos, picked_up: false, label, vel_y: 0.0, landed: false }
    }
    pub fn key(room_id: usize, pos: glm::Vec3) -> Self {
        Self::make(ItemKind::Key { room_id }, pos, "Key")
    }
    pub fn pill(pos: glm::Vec3) -> Self {
        Self::make(ItemKind::SanityPill, pos, "Sanity Pill")
    }
    pub fn wind_up_toy(pos: glm::Vec3) -> Self {
        Self::make(ItemKind::WindUpToy, pos, "Wind-Up Toy")
    }
    pub fn cd(pos: glm::Vec3) -> Self {
        Self::make(ItemKind::Cd, pos, "CD")
    }
    pub fn cd_player(pos: glm::Vec3) -> Self {
        Self::make(ItemKind::CdPlayer, pos, "CD Player")
    }

    /// Apply gravity each frame until item rests on floor_y.
    pub fn physics_tick(&mut self, dt: f32, floor_y: f32) {
        if self.picked_up || self.landed { return; }
        self.vel_y      += GRAVITY * dt;
        self.position.y += self.vel_y * dt;
        let rest_y = floor_y + ITEM_HALF_SIZE;
        if self.position.y <= rest_y {
            self.position.y = rest_y;
            self.vel_y      = 0.0;
            self.landed     = true;
        }
    }
}

#[derive(Default)]
pub struct Inventory {
    pub items: Vec<ItemKind>,
}

impl Inventory {
    pub fn add(&mut self, kind: ItemKind) {
        self.items.push(kind);
    }

    pub fn has_key_for(&self, room_id: usize) -> bool {
        self.items.iter().any(|k| matches!(k, ItemKind::Key { room_id: id } if *id == room_id))
    }

    pub fn use_key_for(&mut self, room_id: usize) -> bool {
        if let Some(pos) = self.items.iter().position(|k| matches!(k, ItemKind::Key { room_id: id } if *id == room_id)) {
            self.items.remove(pos);
            return true;
        }
        false
    }

    pub fn has_pill(&self) -> bool {
        self.items.contains(&ItemKind::SanityPill)
    }

    pub fn use_pill(&mut self) -> bool {
        if let Some(pos) = self.items.iter().position(|k| k == &ItemKind::SanityPill) {
            self.items.remove(pos);
            return true;
        }
        false
    }

    pub fn has_cd(&self)        -> bool { self.items.contains(&ItemKind::Cd) }
    pub fn has_cd_player(&self) -> bool { self.items.contains(&ItemKind::CdPlayer) }

    pub fn summary(&self) -> Vec<String> {
        let mut counts: std::collections::HashMap<&str, usize> = Default::default();
        for item in &self.items {
            let label = match item {
                ItemKind::Key { .. }  => "Key",
                ItemKind::SanityPill  => "Sanity Pill",
                ItemKind::WindUpToy   => "Wind-Up Toy",
                ItemKind::Cd          => "CD",
                ItemKind::CdPlayer    => "CD Player",
            };
            *counts.entry(label).or_default() += 1;
        }
        let mut out: Vec<String> = counts.iter().map(|(k, v)| format!("{} x{}", k, v)).collect();
        out.sort();
        out
    }
}
