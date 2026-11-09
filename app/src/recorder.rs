//! Frame-accurate recordings of scripted UI performances, for promo videos.
//! Time is simulated (exactly 1/60 s per frame), so every animation renders
//! the same way regardless of how fast frames are captured.
//!
//!     SPARKY_DATA_DIR=$(mktemp -d) cargo run -- --record <dir> <script>
//!
//! Each frame is saved as `<dir>/NNNNN.png`; pointer positions go to
//! `<dir>/pointer.csv` (frame,x,y,pressed) for drawing a cursor later.
use crate::{theme, App};
use eframe::egui::*;
use std::io::Write;

pub const FPS: f64 = 60.;

pub struct Recorder {
    dir: std::path::PathBuf,
    script: String,
    pub frame: u64,
    frames: u64,
    saved: u64,
    pointer: Option<(Pos2, bool)>,
    track: Vec<String>,
}

impl Recorder {
    pub fn new(dir: &str, script: &str) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let frames = match script {
            "nav" => 230,
            "palette" => 220,
            "search" => 200,
            "swipe" => 270,
            "accent" => 200,
            "shortcuts" => 210,
            "reply" => 200,
            _ => 60,
        };
        Self { dir: dir.into(), script: script.into(), frame: 0, frames, saved: 0, pointer: None, track: Vec::new() }
    }
    pub fn done(&self) -> bool {
        self.saved >= self.frames
    }
}

fn key(key: Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }
}
fn tap(events: &mut Vec<Event>, k: Key, modifiers: Modifiers) {
    events.push(key(k, modifiers, true));
    events.push(key(k, modifiers, false));
}
fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
/// Pointer position along eased keyframes (frame, x, y).
fn path(n: u64, keys: &[(u64, f32, f32)]) -> Pos2 {
    let (mut a, mut b) = (keys[0], keys[0]);
    for w in keys.windows(2) {
        if n >= w[0].0 {
            a = w[0];
            b = w[1];
        }
    }
    if n >= b.0 {
        return pos2(b.1, b.2);
    }
    let t = ease((n - a.0) as f32 / (b.0 - a.0).max(1) as f32);
    pos2(a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

impl App {
    /// Input for frame `n` of the active script; may also adjust app state.
    fn script_events(&mut self, n: u64) -> (Vec<Event>, Option<(Pos2, bool)>) {
        let mut ev = Vec::new();
        let script = self.recorder.as_ref().map(|r| r.script.clone()).unwrap_or_default();
        let mut pointer = None;
        let cmd = Modifiers::COMMAND;
        let none = Modifiers::NONE;
        let text = |ev: &mut Vec<Event>, n: u64, start: u64, every: u64, s: &str| {
            for (i, c) in s.chars().enumerate() {
                if n == start + i as u64 * every {
                    ev.push(Event::Text(c.to_string()));
                }
            }
        };
        match script.as_str() {
            // J/J/J/K moves the selection; E archives with the exit animation.
            "nav" => {
                for f in [24, 46, 68] { if n == f { tap(&mut ev, Key::J, none); } }
                if n == 96 { tap(&mut ev, Key::K, none); }
                if n == 130 || n == 175 { tap(&mut ev, Key::E, none); }
            }
            "palette" => {
                if n == 20 { tap(&mut ev, Key::K, cmd); }
                text(&mut ev, n, 62, 11, "gti");
                if n == 125 { self.command_search.clear(); self.command_index = 0; }
                text(&mut ev, n, 135, 9, "arch");
                if n == 185 || n == 200 { tap(&mut ev, Key::ArrowDown, none); }
            }
            "search" => {
                if n == 20 { tap(&mut ev, Key::Slash, none); }
                text(&mut ev, n, 55, 8, "coffee");
                if n == 130 || n == 160 { tap(&mut ev, Key::ArrowDown, none); }
            }
            // Row 2 of the Today section: y 294..382 at 1440 x 900.
            "swipe" => {
                let p = path(n, &[(0, 980., 620.), (28, 430., 338.), (60, 430., 338.), (90, 505., 336.),
                    (140, 505., 336.), (160, 470., 338.), (198, 330., 338.), (212, 330., 338.), (236, 592., 336.)]);
                let mut pressed = false;
                if (30..48).contains(&n) {
                    ev.push(Event::MouseWheel { unit: MouseWheelUnit::Point, delta: vec2(-10.5, 0.), modifiers: none });
                }
                if n == 100 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: none }); }
                if (100..105).contains(&n) { pressed = true; }
                if n == 105 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: none }); }
                // Drag the next row open with the mouse.
                if n == 160 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: none }); }
                if (160..199).contains(&n) { pressed = true; }
                if n == 199 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: none }); }
                if n == 244 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: none }); }
                if (244..249).contains(&n) { pressed = true; }
                if n == 249 { ev.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: none }); }
                ev.insert(0, Event::PointerMoved(p));
                pointer = Some((p, pressed));
            }
            // A new accent every half second (one beat at 120 BPM).
            "accent" => {
                self.theme = theme::Accent::ALL[((n / 30) as usize) % theme::Accent::ALL.len()];
            }
            "shortcuts" => {
                if n == 16 { tap(&mut ev, Key::Slash, cmd); }
                text(&mut ev, n, 50, 9, "arch");
                if n == 110 { self.keyboard.query.clear(); self.keyboard.recording = true; }
                if n == 150 { tap(&mut ev, Key::E, none); }
            }
            "reply" => {
                if n == 18 { tap(&mut ev, Key::R, none); }
                text(&mut ev, n, 50, 3, "Love it. Let’s ship it on Friday!");
            }
            _ => {}
        }
        (ev, pointer)
    }

    pub(crate) fn record_input(&mut self, ctx: &Context, raw: &mut RawInput) {
        let Some(n) = self.recorder.as_ref().map(|r| r.frame) else { return };
        raw.time = Some(n as f64 / FPS);
        raw.predicted_dt = (1. / FPS) as f32;
        let (events, pointer) = self.script_events(n);
        raw.events.extend(events);
        let recorder = self.recorder.as_mut().unwrap();
        if pointer.is_some() {
            recorder.pointer = pointer;
        }
        if n < recorder.frames {
            let (p, down) = recorder.pointer.unwrap_or((pos2(-1., -1.), false));
            recorder.track.push(format!("{n},{:.1},{:.1},{}", p.x, p.y, down as u8));
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::new(n)));
        }
        recorder.frame += 1;
        ctx.request_repaint();
    }

    pub(crate) fn record_output(&mut self, ctx: &Context) {
        let Some(recorder) = self.recorder.as_mut() else { return };
        for ev in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, user_data, .. } = ev {
                let Some(n) = user_data.data.as_ref().and_then(|d| d.downcast_ref::<u64>()).copied() else { continue };
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let path = recorder.dir.join(format!("{n:05}.png"));
                image::save_buffer(&path, &bytes, image.size[0] as u32, image.size[1] as u32, image::ColorType::Rgba8).unwrap();
                recorder.saved += 1;
            }
        }
        if recorder.done() {
            if let Ok(mut f) = std::fs::File::create(recorder.dir.join("pointer.csv")) {
                let _ = writeln!(f, "frame,x,y,pressed\n{}", recorder.track.join("\n"));
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}
