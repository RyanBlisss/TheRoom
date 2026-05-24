use nalgebra_glm as glm;
use crate::renderer::{Mesh, build_box, build_quad};

// ─── Constants ───────────────────────────────────────────────────────────────

pub const WALL_H: f32  = 3.0;  // ceiling height
pub const FLOOR_1: f32 = 0.0;
pub const FLOOR_2: f32 = 3.5;  // gap for staircase headroom

// Room IDs
pub const ROOM_MAIN:    usize = 0;
pub const ROOM_BATH:    usize = 1;
pub const ROOM_BED_A:   usize = 2;
pub const ROOM_BED_B:   usize = 3;
pub const ROOM_HALL:    usize = 4;
pub const ROOM_KITCHEN: usize = 5;
pub const ROOM_DINING:  usize = 6;
// Floor 2
pub const ROOM_F2_BATH:  usize = 7;
pub const ROOM_F2_BED_A: usize = 8;
pub const ROOM_F2_BED_B: usize = 9;
pub const ROOM_F2_HALL:  usize = 10;
pub const ROOM_THE_ROOM: usize = 11;

// ─── Wall (AABB slab for collision) ─────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Wall {
    pub min: glm::Vec3,
    pub max: glm::Vec3,
}

impl Wall {
    pub fn new(min: glm::Vec3, max: glm::Vec3) -> Self { Self { min, max } }

    /// Push a point out of this AABB on XZ plane only.
    pub fn push_out(&self, pos: glm::Vec3, radius: f32) -> glm::Vec3 {
        let cx = (self.min.x + self.max.x) * 0.5;
        let cz = (self.min.z + self.max.z) * 0.5;
        let hx = (self.max.x - self.min.x) * 0.5 + radius;
        let hz = (self.max.z - self.min.z) * 0.5 + radius;

        let dx = pos.x - cx;
        let dz = pos.z - cz;

        if dx.abs() < hx && dz.abs() < hz {
            // overlap — find shallowest axis and push out
            let ox = hx - dx.abs();
            let oz = hz - dz.abs();
            if ox < oz {
                return glm::vec3(pos.x + ox * dx.signum(), pos.y, pos.z);
            } else {
                return glm::vec3(pos.x, pos.y, pos.z + oz * dz.signum());
            }
        }
        pos
    }
}

// ─── Door ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Door {
    pub room_a: usize,
    pub room_b: usize,
    /// World position center of door
    pub position: glm::Vec3,
    pub locked: bool,
    pub key_room_id: usize,
    pub open: bool,
}

impl Door {
    pub fn new(room_a: usize, room_b: usize, pos: glm::Vec3, locked: bool, key_room_id: usize) -> Self {
        Self { room_a, room_b, position: pos, locked, key_room_id, open: false }
    }
}

// ─── Stair bounds for floor transitions ─────────────────────────────────────

pub struct StairBounds {
    pub x_min: f32, pub x_max: f32,
    pub z_min: f32, pub z_max: f32,
    pub y_low: f32, pub y_high: f32,
    pub ascend_dir: bool, // true = going +x raises y
}

impl StairBounds {
    pub fn contains_xz(&self, x: f32, z: f32) -> bool {
        x >= self.x_min && x <= self.x_max && z >= self.z_min && z <= self.z_max
    }

    pub fn y_at(&self, x: f32, _z: f32) -> f32 {
        let t = ((x - self.x_min) / (self.x_max - self.x_min)).clamp(0.0, 1.0);
        let t = if self.ascend_dir { t } else { 1.0 - t };
        self.y_low + (self.y_high - self.y_low) * t
    }
}

// ─── Room ───────────────────────────────────────────────────────────────────

pub struct Room {
    pub id: usize,
    pub name: &'static str,
    pub floor: u8,
    pub min: glm::Vec3,
    pub max: glm::Vec3,
    pub color: glm::Vec3,
    pub unlocked: bool,
    pub mesh: Mesh,
    pub walls: Vec<Wall>,
}

impl Room {
    pub fn new(id: usize, name: &'static str, floor: u8,
               min: glm::Vec3, max: glm::Vec3, color: glm::Vec3) -> Self {
        let (v, i) = build_box(min, max);
        let mesh = Mesh::new(&v, &i, color);

        // Four thin wall slabs for collision on XZ
        let thickness = 0.1;
        let walls = vec![
            Wall::new(glm::vec3(min.x - thickness, min.y, min.z - thickness),
                      glm::vec3(min.x + thickness, max.y, max.z + thickness)), // left
            Wall::new(glm::vec3(max.x - thickness, min.y, min.z - thickness),
                      glm::vec3(max.x + thickness, max.y, max.z + thickness)), // right
            Wall::new(glm::vec3(min.x - thickness, min.y, min.z - thickness),
                      glm::vec3(max.x + thickness, max.y, min.z + thickness)), // back
            Wall::new(glm::vec3(min.x - thickness, min.y, max.z - thickness),
                      glm::vec3(max.x + thickness, max.y, max.z + thickness)), // front
        ];

        Self { id, name, floor, min, max, color, unlocked: id == ROOM_MAIN, mesh, walls }
    }

    pub fn center_xz(&self) -> (f32, f32) {
        ((self.min.x + self.max.x) * 0.5, (self.min.z + self.max.z) * 0.5)
    }

    pub fn contains_point(&self, p: &glm::Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x &&
        p.z >= self.min.z && p.z <= self.max.z
    }
}

// ─── Door mesh ──────────────────────────────────────────────────────────────

pub struct DoorMesh {
    pub mesh: Mesh,
    pub door_idx: usize,
}

// ─── World ──────────────────────────────────────────────────────────────────

pub struct World {
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    pub door_meshes: Vec<DoorMesh>,
    pub walls: Vec<Wall>,
    pub stairs: StairBounds,
    pub current_floor: u8,
    pub rooms_unlocked: usize,
}

impl World {
    pub fn new() -> Self {
        let rooms = build_rooms();
        let doors = build_doors();

        // Aggregate all room walls
        let walls: Vec<Wall> = rooms.iter().flat_map(|r| r.walls.iter().cloned()).collect();

        let door_meshes = build_door_meshes(&doors);

        let stairs = StairBounds {
            x_min: 4.0, x_max: 6.0,
            z_min: -1.0, z_max: 3.0,
            y_low: FLOOR_1, y_high: FLOOR_2,
            ascend_dir: true,
        };

        Self {
            rooms,
            doors,
            door_meshes,
            walls,
            stairs,
            current_floor: 1,
            rooms_unlocked: 1,
        }
    }

    pub fn unlock_room(&mut self, room_id: usize) {
        if let Some(r) = self.rooms.iter_mut().find(|r| r.id == room_id) {
            if !r.unlocked {
                r.unlocked = true;
                self.rooms_unlocked += 1;
            }
        }
    }

    pub fn open_door(&mut self, door_idx: usize) {
        if let Some(d) = self.doors.get_mut(door_idx) {
            d.locked = false;
            d.open   = true;
            // unlock the room on the other side
            let target = d.room_b;
            self.unlock_room(target);
        }
        // Rebuild door meshes after state change
        self.door_meshes = build_door_meshes(&self.doors);
    }

    pub fn room_at(&self, pos: &glm::Vec3) -> Option<&Room> {
        self.rooms.iter().find(|r| r.contains_point(pos))
    }

    pub fn room_name_at(&self, pos: &glm::Vec3) -> &'static str {
        self.room_at(pos).map(|r| r.name).unwrap_or("Hallway")
    }

    /// Returns index of nearest interactable door within range.
    pub fn nearest_door(&self, pos: &glm::Vec3, range: f32) -> Option<usize> {
        self.doors.iter().enumerate()
            .filter(|(_, d)| !d.open)
            .filter_map(|(i, d)| {
                let dist = glm::distance(pos, &d.position);
                if dist <= range { Some((i, dist)) } else { None }
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(i, _)| i)
    }
}

// ─── Layout builders ────────────────────────────────────────────────────────

fn build_rooms() -> Vec<Room> {
    let y0 = FLOOR_1;
    let y1 = FLOOR_1 + WALL_H;
    let y2 = FLOOR_2;
    let y3 = FLOOR_2 + WALL_H;

    // Muted, moody colours for each room
    vec![
        // ── Floor 1 ──────────────────────────────────────────────────────────
        Room::new(ROOM_MAIN,    "Main Room",    1,
            glm::vec3(-5.0, y0, -5.0), glm::vec3( 5.0, y1,  5.0),
            glm::vec3(0.20, 0.18, 0.16)),

        Room::new(ROOM_BATH,    "Bathroom",     1,
            glm::vec3(-9.0, y0, -3.0), glm::vec3(-5.0, y1,  3.0),
            glm::vec3(0.14, 0.18, 0.20)),

        Room::new(ROOM_BED_A,   "Bedroom",      1,
            glm::vec3(-5.0, y0,  5.0), glm::vec3( 3.0, y1, 12.0),
            glm::vec3(0.15, 0.13, 0.18)),

        Room::new(ROOM_BED_B,   "Bedroom",      1,
            glm::vec3( 3.0, y0,  5.0), glm::vec3(10.0, y1, 12.0),
            glm::vec3(0.16, 0.13, 0.17)),

        Room::new(ROOM_HALL,    "Hall",         1,
            glm::vec3(-5.0, y0, -12.0), glm::vec3( 5.0, y1, -5.0),
            glm::vec3(0.13, 0.13, 0.13)),

        Room::new(ROOM_KITCHEN, "Kitchen",      1,
            glm::vec3(-10.0, y0, -12.0), glm::vec3(-5.0, y1, -5.0),
            glm::vec3(0.17, 0.17, 0.14)),

        Room::new(ROOM_DINING,  "Dining Room",  1,
            glm::vec3( 5.0, y0, -12.0), glm::vec3(12.0, y1, -5.0),
            glm::vec3(0.16, 0.14, 0.12)),

        // ── Floor 2 ──────────────────────────────────────────────────────────
        Room::new(ROOM_F2_BATH,  "Bathroom",    2,
            glm::vec3(-9.0, y2, -3.0), glm::vec3(-5.0, y3,  3.0),
            glm::vec3(0.12, 0.16, 0.18)),

        Room::new(ROOM_F2_BED_A, "Bedroom",     2,
            glm::vec3(-5.0, y2,  5.0), glm::vec3( 3.0, y3, 12.0),
            glm::vec3(0.13, 0.11, 0.16)),

        Room::new(ROOM_F2_BED_B, "Bedroom",     2,
            glm::vec3( 3.0, y2,  5.0), glm::vec3(10.0, y3, 12.0),
            glm::vec3(0.14, 0.11, 0.15)),

        Room::new(ROOM_F2_HALL,  "Upper Hall",  2,
            glm::vec3(-5.0, y2, -12.0), glm::vec3( 5.0, y3, -5.0),
            glm::vec3(0.11, 0.11, 0.11)),

        Room::new(ROOM_THE_ROOM, "The Room",    2,
            glm::vec3( 5.0, y2, -12.0), glm::vec3(14.0, y3, -3.0),
            glm::vec3(0.08, 0.04, 0.04)),
    ]
}

fn build_doors() -> Vec<Door> {
    let y_mid = FLOOR_1 + WALL_H * 0.5;
    let y2_mid = FLOOR_2 + WALL_H * 0.5;

    vec![
        // Main ↔ Bathroom
        Door::new(ROOM_MAIN, ROOM_BATH,
            glm::vec3(-5.0, y_mid, 0.0), true, ROOM_HALL),
        // Main ↔ Bed A
        Door::new(ROOM_MAIN, ROOM_BED_A,
            glm::vec3(0.0, y_mid, 5.0), true, ROOM_BATH),
        // Main ↔ Bed B
        Door::new(ROOM_MAIN, ROOM_BED_B,
            glm::vec3(5.0, y_mid, 7.0), true, ROOM_BED_A),
        // Main ↔ Hall
        Door::new(ROOM_MAIN, ROOM_HALL,
            glm::vec3(0.0, y_mid, -5.0), false, 0),
        // Hall ↔ Kitchen
        Door::new(ROOM_HALL, ROOM_KITCHEN,
            glm::vec3(-5.0, y_mid, -8.0), true, ROOM_BED_B),
        // Hall ↔ Dining
        Door::new(ROOM_HALL, ROOM_DINING,
            glm::vec3(5.0, y_mid, -8.0), true, ROOM_KITCHEN),
        // Staircase door — Floor 1 to Floor 2 (unlocked after all floor-1 rooms)
        Door::new(ROOM_HALL, ROOM_F2_HALL,
            glm::vec3(5.0, y_mid, -5.0), true, ROOM_DINING),
        // Floor 2 doors
        Door::new(ROOM_F2_HALL, ROOM_F2_BATH,
            glm::vec3(-5.0, y2_mid, 0.0), true, ROOM_F2_BED_B),
        Door::new(ROOM_F2_HALL, ROOM_F2_BED_A,
            glm::vec3(0.0, y2_mid, 5.0), true, ROOM_F2_BATH),
        Door::new(ROOM_F2_HALL, ROOM_F2_BED_B,
            glm::vec3(5.0, y2_mid, 5.0), true, ROOM_F2_BED_A),
        Door::new(ROOM_F2_HALL, ROOM_THE_ROOM,
            glm::vec3(5.0, y2_mid, -3.0), true, ROOM_F2_HALL),
    ]
}

fn build_door_meshes(doors: &[Door]) -> Vec<DoorMesh> {
    doors.iter().enumerate().map(|(i, door)| {
        let color = if door.open {
            glm::vec3(0.05, 0.05, 0.05) // open = invisible/dark
        } else if door.locked {
            glm::vec3(0.4, 0.15, 0.05)  // locked = dark rust
        } else {
            glm::vec3(0.25, 0.18, 0.10) // unlocked = wood
        };

        // Door quad: 1 unit wide, 2.2 units tall, perpendicular to wall normal
        // We approximate orientation from position; for now place flat on XZ
        let p  = door.position;
        let hw = 0.5;
        let hh = 1.1;
        let (v, idx) = build_quad(
            glm::vec3(p.x - hw, p.y - hh, p.z),
            glm::vec3(p.x + hw, p.y - hh, p.z),
            glm::vec3(p.x + hw, p.y + hh, p.z),
            glm::vec3(p.x - hw, p.y + hh, p.z),
            glm::vec3(0.0, 0.0, 1.0),
        );
        DoorMesh { mesh: Mesh::new(&v, &idx, color), door_idx: i }
    }).collect()
}
