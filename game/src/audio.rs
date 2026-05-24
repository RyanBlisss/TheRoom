/// Audio manager. Currently stubs out — real OGG files are placeholders.
/// When real assets are placed in assets/sounds/, replace the load calls.
pub struct AudioManager {
    pub enabled: bool,
}

impl AudioManager {
    pub fn new() -> Self {
        // rodio::OutputStream::try_default() would go here
        // keeping it stub until real sound files exist
        Self { enabled: false }
    }

    pub fn play_ambient(&self) {
        // TODO: loop assets/sounds/ambient.ogg
    }

    pub fn play_heartbeat(&self, _intensity: f32) {
        // TODO: play assets/sounds/heartbeat.ogg, pitch scaled by intensity
    }

    pub fn play_door_unlock(&self) {
        // TODO: play assets/sounds/door_unlock.ogg
    }

    pub fn play_pill_pickup(&self) {
        // TODO: play assets/sounds/pill_pickup.ogg
    }
}
