use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;

pub struct AudioManager {
    _stream:       Option<OutputStream>,
    handle:        Option<OutputStreamHandle>,
    ambient_sink:  Option<Sink>,
    heartbeat_sink: Option<Sink>,
    pub master_volume: f32,
    heartbeat_timer: f32,
}

impl AudioManager {
    pub fn new() -> Self {
        let (stream_opt, handle_opt) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(_)     => (None, None),
        };

        let mut mgr = Self {
            _stream:         stream_opt,
            handle:          handle_opt,
            ambient_sink:    None,
            heartbeat_sink:  None,
            master_volume:   0.8,
            heartbeat_timer: 0.0,
        };
        mgr.play_ambient();
        mgr
    }

    fn make_sink(&self) -> Option<Sink> {
        self.handle.as_ref().and_then(|h| Sink::try_new(h).ok())
    }

    fn load(path: &str) -> Option<Decoder<BufReader<File>>> {
        File::open(path).ok()
            .map(BufReader::new)
            .and_then(|r| Decoder::new(r).ok())
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.master_volume = vol;
        if let Some(s) = &self.ambient_sink    { s.set_volume(vol * 0.45); }
        if let Some(s) = &self.heartbeat_sink  { s.set_volume(0.0); }
    }

    pub fn play_ambient(&mut self) {
        if self.ambient_sink.is_some() { return; }
        let Some(sink) = self.make_sink() else { return };
        if let Some(src) = Self::load("assets/sounds/ambient.ogg") {
            sink.set_volume(self.master_volume * 0.45);
            sink.append(src.repeat_infinite());
            sink.play();
            self.ambient_sink = Some(sink);
        }
    }

    /// Call every frame with current insanity (0..1). Heartbeat fades in.
    pub fn tick_heartbeat(&mut self, insanity: f32, dt: f32) {
        if insanity < 0.35 {
            if let Some(s) = &self.heartbeat_sink { s.set_volume(0.0); }
            self.heartbeat_timer = 0.0;
            return;
        }

        let target_vol = self.master_volume * ((insanity - 0.35) / 0.65).powi(2) * 0.9;

        // Start heartbeat sink if not running
        if self.heartbeat_sink.is_none() || self.heartbeat_sink.as_ref().map(|s| s.empty()).unwrap_or(true) {
            let Some(sink) = self.make_sink() else { return };
            if let Some(src) = Self::load("assets/sounds/heartbeat.ogg") {
                sink.set_volume(target_vol);
                sink.append(src.repeat_infinite());
                sink.play();
                self.heartbeat_sink = Some(sink);
            }
        } else if let Some(s) = &self.heartbeat_sink {
            // Smoothly raise volume toward target
            let cur = s.volume();
            s.set_volume(cur + (target_vol - cur) * (dt * 3.0).min(1.0));
        }
    }

    pub fn play_door_unlock(&self) {
        let Some(sink) = self.make_sink() else { return };
        if let Some(src) = Self::load("assets/sounds/door_unlock.ogg") {
            sink.set_volume(self.master_volume * 0.85);
            sink.append(src);
            sink.detach();
        }
    }

    pub fn play_pill_pickup(&self) {
        let Some(sink) = self.make_sink() else { return };
        if let Some(src) = Self::load("assets/sounds/pill_pickup.ogg") {
            sink.set_volume(self.master_volume * 0.70);
            sink.append(src);
            sink.detach();
        }
    }
}
