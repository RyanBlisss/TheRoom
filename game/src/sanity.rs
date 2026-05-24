/// Sanity is split into two buckets:
///   - `base`: starts at 1.0, reduced permanently when rooms change (−0.2 each time)
///   - `current`: starts at `base`, drains over time, restored by pills up to `base`
///
/// Displayed value = current / 1.0  (shown as a percentage of original max)

pub struct Sanity {
    pub base: f32,       // permanent ceiling, floors at 0
    pub current: f32,    // actual sanity, 0..=base
    drain_rate: f32,     // per second passive drain
    pub catches: u32,    // how many times monster has caught player (affects visuals)
}

impl Sanity {
    pub fn new() -> Self {
        Self {
            base: 1.0,
            current: 1.0,
            drain_rate: 0.005, // ~3.3 minutes to drain fully at normal rate
            catches: 0,
        }
    }

    /// Called every frame. dt in seconds.
    pub fn tick(&mut self, dt: f32) {
        self.current = (self.current - self.drain_rate * dt).max(0.0);
    }

    /// Called when a new room is unlocked — permanent 20% hit.
    pub fn permanent_hit(&mut self) {
        self.base = (self.base - 0.20).max(0.0);
        // current also cannot exceed new base
        self.current = self.current.min(self.base);
    }

    /// Pills restore non-permanent sanity up to the current base ceiling.
    pub fn use_pill(&mut self) -> bool {
        if self.current >= self.base {
            return false; // already full
        }
        self.current = (self.current + 0.25).min(self.base);
        true
    }

    /// Monster catch — rooms change, loses some current sanity too.
    pub fn on_catch(&mut self) {
        self.catches += 1;
        self.current = (self.current - 0.10).max(0.0);
    }

    /// 0.0 = totally insane, 1.0 = fully sane.
    pub fn normalised(&self) -> f32 {
        self.current
    }

    /// How "insane" we are for visual/audio effects. 0..1.
    pub fn insanity(&self) -> f32 {
        1.0 - self.normalised()
    }

    /// Whether we're in a low-sanity danger zone.
    pub fn is_critical(&self) -> bool {
        self.current < 0.25
    }
}
