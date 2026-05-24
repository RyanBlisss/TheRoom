use nalgebra_glm as glm;
use std::collections::HashSet;
use glutin::event::VirtualKeyCode;

pub const PLAYER_HEIGHT: f32    = 1.7;
pub const PLAYER_RADIUS: f32    = 0.3;
pub const MOVE_SPEED: f32       = 4.0;
pub const MOUSE_SENSITIVITY: f32 = 0.0015;
pub const GRAVITY: f32          = -9.81;
pub const JUMP_SPEED: f32       = 4.5;   // gives ~1.03 m apex

pub struct Player {
    pub position: glm::Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub interact_range: f32,
    pub vel_y: f32,
    pub on_ground: bool,
}

impl Player {
    pub fn new(start: glm::Vec3) -> Self {
        Self {
            position: start,
            yaw: 0.0,
            pitch: 0.0,
            interact_range: 2.0,
            vel_y: 0.0,
            on_ground: false,
        }
    }

    /// Forward direction on XZ plane.
    pub fn forward_xz(&self) -> glm::Vec3 {
        glm::normalize(&glm::vec3(self.yaw.sin(), 0.0, self.yaw.cos()))
    }

    /// Right direction on XZ plane.
    pub fn right_xz(&self) -> glm::Vec3 {
        glm::normalize(&glm::vec3(self.yaw.cos(), 0.0, -self.yaw.sin()))
    }

    /// Full 3D forward (for raycasting interact).
    pub fn forward_3d(&self) -> glm::Vec3 {
        glm::normalize(&glm::vec3(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        ))
    }

    pub fn eye_position(&self) -> glm::Vec3 {
        glm::vec3(self.position.x, self.position.y + PLAYER_HEIGHT * 0.85, self.position.z)
    }

    pub fn view_matrix(&self) -> glm::Mat4 {
        let eye    = self.eye_position();
        let target = eye + self.forward_3d();
        let up     = glm::vec3(0.0_f32, 1.0, 0.0);
        glm::look_at(&eye, &target, &up)
    }

    pub fn update(
        &mut self,
        keys: &HashSet<VirtualKeyCode>,
        dt: f32,
        walls: &[crate::world::Wall],
        stairs: Option<&crate::world::StairBounds>,
    ) {
        // ── Horizontal movement ───────────────────────────────────────────
        let mut velocity = glm::vec3(0.0f32, 0.0, 0.0);
        if keys.contains(&VirtualKeyCode::W) { velocity += self.forward_xz() * MOVE_SPEED; }
        if keys.contains(&VirtualKeyCode::S) { velocity -= self.forward_xz() * MOVE_SPEED; }
        if keys.contains(&VirtualKeyCode::A) { velocity += self.right_xz()   * MOVE_SPEED; }
        if keys.contains(&VirtualKeyCode::D) { velocity -= self.right_xz()   * MOVE_SPEED; }

        let desired  = self.position + velocity * dt;
        let resolved = resolve_collision(desired, walls, PLAYER_RADIUS);
        self.position.x = resolved.x;
        self.position.z = resolved.z;

        // ── Stair climbing (overrides gravity while on stairs) ────────────
        let on_stairs = stairs.map_or(false, |s| s.contains_xz(self.position.x, self.position.z));
        if on_stairs {
            if let Some(s) = stairs {
                let target_y = s.y_at(self.position.x, self.position.z);
                self.position.y = lerp(self.position.y, target_y, (dt * 10.0).min(1.0));
                self.vel_y     = 0.0;
                self.on_ground = true;
            }
        } else {
            // ── Gravity & jump ────────────────────────────────────────────
            if keys.contains(&VirtualKeyCode::Space) && self.on_ground {
                self.vel_y     = JUMP_SPEED;
                self.on_ground = false;
            }

            self.vel_y      += GRAVITY * dt;
            self.position.y += self.vel_y * dt;

            // Ground: snap to nearest floor below the player.
            let floor_y = nearest_floor_below(self.position);
            if self.position.y <= floor_y {
                self.position.y = floor_y;
                self.vel_y      = 0.0;
                self.on_ground  = true;
            } else {
                self.on_ground = false;
            }
        }
    }

    pub fn apply_mouse(&mut self, dx: f32, dy: f32) {
        self.yaw   -= dx * MOUSE_SENSITIVITY;
        self.pitch  = (self.pitch - dy * MOUSE_SENSITIVITY).clamp(-1.4, 1.4);
    }
}

fn resolve_collision(mut pos: glm::Vec3, walls: &[crate::world::Wall], radius: f32) -> glm::Vec3 {
    for wall in walls {
        pos = wall.push_out(pos, radius);
    }
    pos
}

fn nearest_floor_below(pos: glm::Vec3) -> f32 {
    let f2 = crate::world::FLOOR_2;
    let f1 = crate::world::FLOOR_1;
    // If above floor-2 threshold, stand on floor 2; otherwise floor 1.
    if pos.y > f2 - 0.5 { f2 } else { f1 }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
