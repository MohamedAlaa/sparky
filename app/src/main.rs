mod classifier;
mod credentials;
mod date_groups;
mod editor;
mod icons;
mod oauth;
mod recorder;
mod shortcuts;
mod storage;
mod sync;
mod threading;
mod theme;
use eframe::egui::*;
use serde::{Deserialize, Serialize};
const INK: Color32 = Color32::from_rgb(37, 39, 48);
// 4.9:1 on white and 4.6:1 on the sidebar: readable for small secondary text.
const MUTED: Color32 = Color32::from_rgb(98, 103, 118);
// Decorative only (keycap edges, dividers); never used for text.
const SUBTLE: Color32 = Color32::from_rgb(196, 200, 210);
const SIDEBAR_BG: Color32 = Color32::from_rgb(247, 247, 250);
// Type scale.
const T_CAPTION: f32 = 11.5;
const T_SMALL: f32 = 12.5;
const T_BODY: f32 = 14.;
const T_TITLE: f32 = 22.;
// Mail list geometry: section header height, row height, row pitch.
const SECTION_H: f32 = 34.;
const ROW_H: f32 = 88.;
const ROW_STRIDE: f32 = 92.;
const SWIPE_OPEN: f32 = 168.;
// Swipe action fills. Archive is a true orange (white 3.6:1, enough for the
// icon and bold label); Delete keeps white above 4.5:1.
const SWIPE_ARCHIVE: Color32 = Color32::from_rgb(234, 88, 12);
const SWIPE_DELETE: Color32 = Color32::from_rgb(208, 36, 36);
const PURPLE: Color32 = Color32::from_rgb(113, 91, 211);
const LINE: Color32 = Color32::from_rgb(225, 228, 235);
const CORNER_RADIUS: u8 = 6;

#[derive(Default)]
struct Toast {
    message: String,
    started: Option<std::time::Instant>,
}
impl Toast {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), started: Some(std::time::Instant::now()) }
    }
    fn opacity(&self, elapsed: f32) -> f32 {
        ((5.0 - elapsed) / 0.4).clamp(0., 1.)
    }
    fn show(&mut self, ctx: &Context) {
        let Some(started) = self.started else { return };
        if self.message.is_empty() { return; }
        let elapsed = started.elapsed().as_secs_f32();
        let opacity = self.opacity(elapsed);
        if opacity <= 0. {
            *self = Self::default();
            return;
        }
        if elapsed >= 4.6 || elapsed < icons::DUR_MED {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(4.6 - elapsed));
        }
        let is_error = ["could not", "unavailable", "failed", "error", "denied", "stopped"]
            .iter().any(|word| self.message.to_lowercase().contains(word));
        let is_info = self.message.contains('…') || self.message.starts_with("Local demo");
        let accent = if is_error { Color32::from_rgb(185, 62, 66) }
            else if is_info { ctx.style().visuals.selection.stroke.color } else { Color32::from_rgb(39, 133, 99) };
        let mut dismiss = false;
        let rise = icons::ease(elapsed / icons::DUR_MED);
        Area::new(Id::new("notification-toast"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, vec2(0., -52. + 10. * (1. - rise)))
            .show(ctx, |ui| {
                ui.set_opacity(opacity.min(rise));
                ui.set_max_width((ctx.screen_rect().width() - 64.).min(520.));
                Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1., LINE))
                    .corner_radius(12)
                    .inner_margin(Margin::symmetric(16, 12))
                    .shadow(Shadow { offset: [0, 4], blur: 20, spread: 0,
                        color: Color32::from_black_alpha(24) })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.;
                            ui.add(Image::new(if is_error { icons::Icon::Alert } else if is_info { icons::Icon::Info } else { icons::Icon::Check }.source())
                                .fit_to_exact_size(vec2(18., 18.)).tint(accent));
                            let message = ui.painter().layout(
                                self.message.clone(), FontId::proportional(13.), INK,
                                (ctx.screen_rect().width() - 164.).min(420.),
                            );
                            ui.add(Label::new(message).selectable(false));
                            dismiss = icons::icon_button(ui, icons::Icon::X, 26., MUTED, Color32::from_black_alpha(14))
                                .on_hover_text("Dismiss notification").clicked();
                        });
                    });
            });
        if dismiss { *self = Self::default(); }
    }
}

fn app_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    // Use the installed Apple font, without redistributing Apple's font files.
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/SFNS.ttf") {
        fonts.font_data.insert(
            "Apple San Francisco".into(),
            FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "Apple San Francisco".into());
    }
    let bold = std::fs::read("/System/Library/Fonts/Supplemental/Arial Bold.ttf").ok();
    let family = if let Some(bytes) = bold {
        fonts
            .font_data
            .insert("writer-bold".into(), FontData::from_owned(bytes).into());
        vec!["writer-bold".into()]
    } else {
        fonts.families[&FontFamily::Proportional].clone()
    };
    fonts
        .families
        .insert(FontFamily::Name("writer-bold".into()), family);
    let mut medium_family = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read("/System/Library/Fonts/HelveticaNeue.ttc") {
        let mut medium = FontData::from_owned(bytes);
        medium.index = 10; // Installed Helvetica Neue Medium face.
        fonts.font_data.insert("ui-medium".into(), medium.into());
        medium_family.insert(0, "ui-medium".into());
    }
    fonts
        .families
        .insert(FontFamily::Name("ui-medium".into()), medium_family);
    fonts
}
fn medium_label(label: &str) -> RichText {
    RichText::new(label)
        .size(12.)
        .family(FontFamily::Name("ui-medium".into()))
}
// A translucent pearl rim, bright edge and broad ambient shadow surround paper surfaces.
fn glass_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgba_unmultiplied(239, 242, 250, 238))
        .stroke(Stroke::new(1.5, Color32::from_white_alpha(245)))
        .corner_radius(CORNER_RADIUS)
        .inner_margin(10.)
        .shadow(Shadow {
            offset: [0, 16],
            blur: 52,
            spread: 2,
            color: Color32::from_black_alpha(38),
        })
}
fn paper_frame() -> Frame {
    Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1., LINE))
        .corner_radius(CORNER_RADIUS)
        .inner_margin(20.)
}

// Keep form inputs comfortably padded, including masked password fields.
fn form_field(value: &mut String) -> TextEdit<'_> {
    TextEdit::singleline(value)
        .font(FontId::proportional(14.))
        .margin(vec2(12., 10.))
        .desired_width(f32::INFINITY)
}

fn compose_address_row(ui: &mut Ui, label: &str, value: &mut String, bcc: Option<&mut bool>) {
    // Use the form's full width for every recipient underline, including the
    // To row whose trailing Bcc toggle otherwise shortens its measured bounds.
    let line_left = ui.cursor().left();
    let line_right = line_left + ui.available_width();
    let row = ui
        .horizontal(|ui| {
            ui.spacing_mut().button_padding = vec2(5., 2.);
            ui.spacing_mut().interact_size.y = 24.;
            let field_id = ui.make_persistent_id(("compose-address", label));
            if ui
                .add_sized(
                    vec2(34., 24.),
                    Button::new(medium_label(label).color(INK))
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1., Color32::from_gray(210)))
                        .corner_radius(5.),
                )
                .clicked()
            {
                ui.memory_mut(|m| m.request_focus(field_id));
            }
            ui.add_space(4.);
            let width = ui.available_width();
            ui.allocate_ui_with_layout(
                vec2(width, 32.),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let reserve = if bcc.is_some() {
                        42. + ui.spacing().item_spacing.x
                    } else {
                        0.
                    };
                    ui.add_sized(
                        vec2((width - reserve).max(80.), 30.),
                        TextEdit::singleline(value)
                            .id(field_id)
                            .frame(false)
                            .font(FontId::proportional(16.))
                            .margin(vec2(8., 4.)),
                    );
                    if let Some(show) = bcc {
                        if ui
                            .add_sized(
                                vec2(42., 24.),
                                Button::new(medium_label("Bcc").color(Color32::from_gray(75)))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            *show = !*show;
                        }
                    }
                },
            );
        })
        .response
        .rect;
    ui.painter().hline(
        line_left..=line_right,
        row.bottom(),
        Stroke::new(1., Color32::from_gray(160)),
    );
}

// Provider marks are drawn locally so onboarding also works offline.
fn provider_logo(p: &Painter, at: Pos2, provider: &str) {
    if provider == "gmail" {
        for (points, color) in [
            (
                vec![at + vec2(2., 22.), at + vec2(2., 2.)],
                Color32::from_rgb(66, 133, 244),
            ),
            (
                vec![at + vec2(2., 2.), at + vec2(15., 12.), at + vec2(28., 2.)],
                Color32::from_rgb(234, 67, 53),
            ),
            (
                vec![at + vec2(28., 2.), at + vec2(28., 10.)],
                Color32::from_rgb(251, 188, 4),
            ),
            (
                vec![at + vec2(28., 10.), at + vec2(28., 22.)],
                Color32::from_rgb(52, 168, 83),
            ),
        ] {
            p.add(Shape::line(points, Stroke::new(5., color)));
        }
    } else {
        let blue = Color32::from_rgb(0, 120, 212);
        p.rect_filled(
            Rect::from_min_size(at + vec2(6., 0.), vec2(27., 24.)),
            3.,
            if provider == "graph" { blue } else { MUTED },
        );
        p.add(Shape::line(
            vec![at + vec2(7., 3.), at + vec2(20., 13.), at + vec2(32., 3.)],
            Stroke::new(1.5, Color32::WHITE),
        ));
        if provider == "graph" {
            p.rect_filled(
                Rect::from_min_size(at + vec2(-2., 5.), vec2(20., 22.)),
                2.,
                Color32::from_rgb(0, 83, 157),
            );
            p.text(
                at + vec2(8., 16.),
                Align2::CENTER_CENTER,
                "O",
                FontId::proportional(17.),
                Color32::WHITE,
            );
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Mail {
    from: String,
    initials: String,
    #[serde(default)]
    cc: String,
    #[serde(default)]
    bcc: String,
    subject: String,
    preview: String,
    body: String,
    #[serde(default)]
    marks: Vec<u8>,
    time: String,
    category: String,
    #[serde(default)]
    labels: Vec<String>,
    folder: String,
    unread: bool,
    star: bool,
    color: [u8; 3],
    #[serde(default)]
    snoozed_until: Option<u64>,
    #[serde(default)]
    account: String,
    #[serde(default)]
    remote_id: String,
    #[serde(default)]
    remote_thread: String,
    #[serde(default)]
    message_id: String,
    #[serde(default)]
    references: Vec<String>,
}
#[derive(Serialize, Deserialize)]
struct Stored {
    mails: Vec<Mail>,
}
impl Mail {
    fn is_sample(&self) -> bool {
        self.account.trim().is_empty() && self.remote_id.is_empty()
            && !matches!(self.folder.as_str(), "Sent" | "Drafts")
    }

    fn mailbox_label(&self) -> String {
        let address = self.account.trim().split_once(':')
            .map_or(self.account.trim(), |(_, address)| address).trim();
        if !address.is_empty() {
            let direction = if matches!(self.folder.as_str(), "Sent" | "Drafts") { "From" } else { "To" };
            format!("{direction}: {address}")
        } else if matches!(self.folder.as_str(), "Sent" | "Drafts") {
            "Saved on this device".into()
        } else if self.remote_id.is_empty() {
            "Sample message".into()
        } else {
            "Receiving account unavailable".into()
        }
    }
}
#[derive(Default)]
struct SwipeMotion {
    offset: f32,
    velocity: f32,
}
impl SwipeMotion {
    fn follow(&mut self, offset: f32) {
        self.offset = offset;
        self.velocity = 0.;
    }

    fn settle(&mut self, target: f32, dt: f32) -> bool {
        // Exact critically damped spring: consistent at every display refresh rate,
        // with continuous position and velocity when a closing swipe is reopened.
        let displacement = self.offset - target;
        let decay = (-28. * dt).exp();
        let impulse = self.velocity + 28. * displacement;
        self.offset = target + (displacement + impulse * dt) * decay;
        self.velocity = (self.velocity - 28. * impulse * dt) * decay;
        if (self.offset - target).abs() < 0.1 && self.velocity.abs() < 1. {
            self.follow(target);
            false
        } else {
            true
        }
    }
}
/// A row leaving the list after Archive/Delete/Snooze: it slides out, then its
/// height collapses. The move itself is applied only when the animation ends.
struct Leaving {
    idx: usize,
    message_id: String,
    action: &'static str,
    started: f64,
    /// Swipe offset when the action fired, so the slide continues from there.
    from: f32,
}
const LEAVE_SLIDE: f64 = 0.18;
const LEAVE_COLLAPSE: f64 = 0.2;
impl Leaving {
    fn slide(&self, now: f64) -> f32 {
        icons::ease(((now - self.started) / LEAVE_SLIDE) as f32)
    }
    fn collapse(&self, now: f64) -> f32 {
        icons::ease(((now - self.started - LEAVE_SLIDE) / LEAVE_COLLAPSE) as f32)
    }
    fn done(&self, now: f64) -> bool {
        now - self.started >= LEAVE_SLIDE + LEAVE_COLLAPSE
    }
}

/// "Product Hunt <hello@producthunt.com>" → "Product Hunt"; bare addresses stay intact.
fn sender_name(from: &str) -> &str {
    let name = from.split('<').next().unwrap_or(from).trim().trim_matches('"').trim();
    if name.is_empty() { from.trim() } else { name }
}

/// Paints one keycap with its left edge at `left_center.x`, vertically centered on it.
/// Returns its width.
fn paint_keycap(p: &Painter, left_center: Pos2, key: &str) -> f32 {
    // System fonts lack the return glyph, so it is drawn as a Lucide icon.
    let enter = key == "↵";
    let galley = p.layout_no_wrap(if enter { String::new() } else { key.to_owned() }, FontId::new(11., FontFamily::Name("ui-medium".into())), MUTED);
    let size = vec2((galley.size().x + 12.).max(20.), 20.);
    let rect = Rect::from_min_size(left_center - vec2(0., size.y / 2.), size);
    p.rect_filled(rect.translate(vec2(0., 1.5)), 5., SUBTLE);
    p.rect_filled(rect, 5., Color32::WHITE);
    p.rect_stroke(rect, 5., Stroke::new(1., LINE), StrokeKind::Inside);
    if enter {
        icons::paint(p, p.ctx(), icons::Icon::Enter, Rect::from_center_size(rect.center(), vec2(12., 12.)), MUTED);
    } else {
        p.galley(rect.center() - galley.size() / 2., galley, MUTED);
    }
    size.x
}
/// Paints a hint such as "⌘ ⇧ P" or "G then I" as keycaps. With `right_align`,
/// `at` is the right edge. Returns the total width.
fn keycaps_width(p: &Painter, hint: &str) -> f32 {
    let caps = shortcuts::keycaps(hint);
    caps.iter().map(|k| match k {
        Some(k) => (p.layout_no_wrap(k.clone(), FontId::new(11., FontFamily::Name("ui-medium".into())), MUTED).size().x + 12.).max(20.),
        None => 14.,
    }).sum::<f32>() + 4. * caps.len().saturating_sub(1) as f32
}
fn paint_keycaps(p: &Painter, at: Pos2, hint: &str, right_align: bool) -> f32 {
    let caps = shortcuts::keycaps(hint);
    let total = keycaps_width(p, hint);
    let mut x = if right_align { at.x - total } else { at.x };
    for k in &caps {
        match k {
            Some(k) => x += paint_keycap(p, pos2(x, at.y), k) + 4.,
            None => {
                p.text(pos2(x + 7., at.y), Align2::CENTER_CENTER, "→", FontId::proportional(11.), MUTED);
                x += 18.;
            }
        }
    }
    total
}

/// Char indices covered by search tokens matching at word starts (prefix
/// matches, like the full-text index), case-insensitively.
fn token_hits(text: &str, tokens: &[String]) -> Vec<usize> {
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let mut hits = Vec::new();
    for token in tokens {
        let t: Vec<char> = token.chars().collect();
        if t.is_empty() { continue; }
        for i in 0..lower.len() {
            let word_start = i == 0 || !chars[i - 1].is_alphanumeric();
            if word_start && lower[i..].starts_with(&t) {
                hits.extend(i..i + t.len());
            }
        }
    }
    hits
}
/// A one-line excerpt of `body` around the first token match ("…before match after").
fn snippet(body: &str, tokens: &[String]) -> String {
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = flat.chars().collect();
    let first = token_hits(&flat, tokens).into_iter().min();
    let Some(at) = first else {
        return chars.iter().take(120).collect();
    };
    let start = at.saturating_sub(30);
    let excerpt: String = chars[start..].iter().take(140).collect();
    if start > 0 { format!("…{excerpt}") } else { excerpt }
}

/// Text with fuzzy-matched characters drawn in the accent color and medium weight.
fn highlighted(text: &str, matches: &[usize], size: f32, base: Color32, accent: Color32) -> text::LayoutJob {
    let mut job = text::LayoutJob::default();
    let regular = FontId::proportional(size);
    let strong = FontId::new(size, FontFamily::Name("ui-medium".into()));
    for (i, ch) in text.chars().enumerate() {
        let hit = matches.contains(&i);
        let mut buf = [0u8; 4];
        job.append(
            ch.encode_utf8(&mut buf),
            0.,
            text::TextFormat::simple(if hit { strong.clone() } else { regular.clone() }, if hit { accent } else { base }),
        );
    }
    job
}

/// ↑/↓ wrap, Page Up/Down jump by 8. Consumes the keys; returns whether a key was pressed.
fn list_navigation(ctx: &Context, index: &mut usize, len: usize) -> bool {
    if len == 0 {
        return false;
    }
    let mut step = |d: isize, wrap: bool| {
        let n = *index as isize + d;
        *index = if wrap { n.rem_euclid(len as isize) as usize } else { n.clamp(0, len as isize - 1) as usize };
    };
    let mut pressed = false;
    for (key, d, wrap) in [(Key::ArrowDown, 1, true), (Key::ArrowUp, -1, true), (Key::PageDown, 8, false), (Key::PageUp, -8, false)] {
        while ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
            step(d, wrap);
            pressed = true;
        }
    }
    *index = (*index).min(len - 1);
    pressed
}

/// Adds a widget at `rect` without moving the parent's layout cursor
/// (`Ui::put` would pull the cursor back to the widget's bottom edge).
fn place(ui: &mut Ui, rect: Rect, widget: impl Widget) -> Response {
    ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)))
        .add_sized(rect.size(), widget)
}

/// Shared shell for the command palette and the shortcut guide: a dimmed
/// backdrop (click to close) and a floating panel that fades and rises in.
/// `t` is the eased 0→1 entry progress.
fn modal<R>(ctx: &Context, id: &str, width: f32, top: f32, t: f32, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let screen = ctx.screen_rect();
    Area::new(Id::new((id, "backdrop")))
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let r = ui.allocate_rect(screen, Sense::click());
            ui.painter().rect_filled(screen, 0., Color32::from_black_alpha((70. * t) as u8));
            if r.clicked() {
                *open = false;
            }
        });
    let panel = Id::new((id, "panel"));
    ctx.move_to_top(LayerId::new(Order::Foreground, panel));
    Area::new(panel)
        .order(Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2., top + 10. * (1. - t)))
        .show(ctx, |ui| {
            ui.set_opacity(t);
            Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1., LINE))
                .corner_radius(12)
                .shadow(Shadow { offset: [0, 18], blur: 48, spread: 0, color: Color32::from_black_alpha(46) })
                .show(ui, |ui| {
                    ui.set_width(width);
                    add(ui)
                })
                .inner
        })
        .inner
}

struct App {
    theme: theme::Accent,
    classifier: classifier::State,
    mails: Vec<Mail>,
    selected: usize,
    folder: String,
    category: String,
    search: String,
    compose: bool,
    to: String,
    cc: String,
    bcc: String,
    show_bcc: bool,
    draft_signature: String,
    draft_dirty_since: Option<std::time::Instant>,
    draft_saved: bool,
    subject: String,
    body: String,
    palette: bool,
    command_search: String,
    command_index: usize,
    command_focus: bool,
    keyboard: shortcuts::State,
    editor: editor::Editor,
    writer_focus: bool,
    inline_reply: bool,
    accounts: Vec<sync::Account>,
    account_filter: Option<String>,
    pending_account: Option<sync::Account>,
    google_signin: Option<(
        std::sync::mpsc::Receiver<Result<sync::Account, String>>,
        std::sync::Arc<std::sync::atomic::AtomicBool>,
    )>,
    google_setup_json: String,
    connect_provider: String,
    connect_address: String,
    connect_secret: String,
    connect_imap: String,
    connect_smtp: String,
    connect_error: String,
    toast: Toast,
    frames: u32,
    screenshot: Option<String>,
    saved_shot: bool,
    settings: bool,
    welcome: bool,
    scroll: f32,
    collapsed_dates: [bool; 3],
    swipe: Option<(usize, f32)>,
    swipe_motion: std::collections::HashMap<usize, SwipeMotion>,
    swipe_last_input: f64,
    leaving: Option<Leaving>,
    now: f64,
    recent_commands: Vec<String>,
    recorder: Option<recorder::Recorder>,
    /// Why saved sign-ins couldn't be read from Keychain at startup, if they couldn't.
    credentials_error: Option<String>,
    search_open: bool,
    search_query: String,
    search_index: usize,
    search_focus: bool,
    recent_searches: Vec<String>,
    editing_draft: Option<usize>,
    replying_to: Option<Mail>,
    store: Option<storage::Store>,
    account: Option<sync::Account>,
    syncing: Option<std::sync::mpsc::Receiver<Result<sync::SyncBatch, String>>>,
    sending: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
}
fn sample() -> Vec<Mail> {
    let rows=[
 ("Alex Morgan","AM","A little more space for big ideas","The new workspace is ready. Take a look around.","Hi Jamie,\n\nGood things happen when we make room for them.\n\nWe’ve put together the first look at our new workspace, and I’d love to hear what you think. A calmer place to focus, share ideas, and do our best work together.\n\nA few things to explore:\n\n  •  A fresh direction for the studio\n  •  Spaces designed for deep work\n  •  Little details that make a big difference\n\nTake a look when you have a moment. Let’s catch up over coffee this week and talk through your ideas.\n\nAlex","10:42 AM","People",[210,225,216]),
 ("Sofia Chen","SC","Design review · Friday","A few thoughts ahead of our next conversation.","Hi Jamie,\n\nI’ve gathered our latest designs for Friday’s review. The direction is feeling good!\n\nCould you bring your thoughts on the onboarding flow? I’d love to make the first few moments feel a little more personal.\n\nSee you Friday,\nSofia","10:18 AM","People",[232,217,239]),
 ("Oliver at Layers","OL","Your weekly dose of inspiration","Five things worth making time for this week.","A small collection of things we loved this week.\n\nThoughtful design, new perspectives, and a reminder to slow down.\n\nEnjoy your weekend!\nThe Layers team","9:56 AM","Newsletters",[243,226,201]),
 ("Maya Patel","MP","Coffee next week?","There’s a new place on the corner I think you’d love.","Hey Jamie,\n\nIt’s been too long! There’s a lovely new coffee shop around the corner. Are you free Tuesday morning?\n\nWould love to hear what you’ve been working on.\n\nMaya","9:30 AM","People",[212,222,245]),
 ("North Studio","NS","Your project is looking good","The latest updates to your workspace are here.","Your workspace is up to date.\n\n3 new comments and 2 completed tasks are waiting for you.\n\nHave a great day!","Yesterday","Notifications",[220,226,235]),
 ("Daniel Brooks","DB","Re: A fresh start","Love this direction. Let’s make it happen.","Jamie,\n\nLove this direction. Let’s make it happen. I’ve added some thoughts to the shared document.\n\nThanks for bringing this together.\nDaniel","Yesterday","People",[236,219,210]),
 ("The Sunday Edit","SE","Less, but better","A slower read for your Sunday morning.","This week, we’re exploring the art of doing less, but doing it better.\n\nPour something warm and settle in.\n\nThe Sunday Edit","Yesterday","Newsletters",[226,226,200]),
 ("Emma Wilson","EW","Summer plans are coming together","A small update from our side of the world.","Hi Jamie,\n\nThe summer plans are finally coming together! Let me know if you’d like to join us for the weekend.\n\nEmma","Tuesday","People",[218,230,228])];
    rows.iter()
        .enumerate()
        .map(|(i, r)| Mail {
            from: r.0.into(),
            initials: r.1.into(),
            cc: String::new(),
            bcc: String::new(),
            subject: r.2.into(),
            preview: r.3.into(),
            body: r.4.into(),
            marks: Vec::new(),
            time: r.5.into(),
            category: r.6.into(),
            labels: Vec::new(),
            folder: "Inbox".into(),
            unread: i < 4,
            star: i == 1,
            color: r.7,
            snoozed_until: None,
            account: String::new(),
            remote_id: String::new(),
            remote_thread: String::new(),
            message_id: String::new(),
            references: Vec::new(),
        })
        .collect()
}
fn txt(p: &Painter, pos: Pos2, s: &str, size: f32, c: Color32) {
    p.text(pos, Align2::LEFT_TOP, s, FontId::proportional(size), c);
}
// Mail metadata can contain newlines. Keep each list field in its own
// measured line so previews never spill into the next message or the date.
fn mail_list_line(p: &Painter, rect: Rect, text: &str, size: f32, color: Color32) {
    mail_list_line_font(p, rect, text, FontId::proportional(size), color)
}
fn mail_list_line_font(p: &Painter, rect: Rect, text: &str, font: FontId, color: Color32) {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut job = text::LayoutJob::simple(text, font, color, rect.width());
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.sections[0].format.line_height = Some(rect.height());
    let galley = p.layout_job(job);
    p.with_clip_rect(p.clip_rect().intersect(rect))
        .galley(rect.min, galley, color);
}
fn line(p: &Painter, a: Pos2, b: Pos2) {
    p.line_segment([a, b], Stroke::new(1., LINE));
}
fn button(ui: &mut Ui, rect: Rect, label: &str, fill: Color32, c: Color32) -> Response {
    button_icon(ui, rect, None, label, fill, c)
}
/// A painted button with an optional leading Lucide icon. Transparent fills are
/// borderless ghost buttons; hover fades in rather than snapping.
fn button_icon(ui: &mut Ui, rect: Rect, icon: Option<icons::Icon>, label: &str, fill: Color32, c: Color32) -> Response {
    let r = ui.allocate_rect(rect, Sense::click());
    let hover = icons::fade(ui.ctx(), r.id.with("hover"), r.hovered(), icons::DUR_FAST);
    let p = ui.painter();
    if fill == Color32::TRANSPARENT {
        p.rect_filled(rect, CORNER_RADIUS, Color32::from_black_alpha((10. * hover) as u8));
    } else {
        p.rect_filled(rect, CORNER_RADIUS, fill.lerp_to_gamma(Color32::BLACK, 0.05 * hover));
        let accent = fill == ui.visuals().selection.stroke.color;
        p.rect_stroke(
            rect,
            CORNER_RADIUS,
            Stroke::new(1., if accent { fill.gamma_multiply(0.85) } else { LINE }),
            StrokeKind::Inside,
        );
    }
    let font = FontId::new(13., FontFamily::Name("ui-medium".into()));
    let galley = p.layout_no_wrap(label.to_owned(), font, c);
    let icon_w = if icon.is_some() { if label.is_empty() { 16. } else { 24. } } else { 0. };
    let start = rect.center().x - (icon_w + galley.size().x) / 2.;
    if let Some(icon) = icon {
        icons::paint(p, ui.ctx(), icon, Rect::from_center_size(pos2(start + 8., rect.center().y), vec2(16., 16.)), c);
    }
    p.galley(pos2(start + icon_w, rect.center().y - galley.size().y / 2.), galley, c);
    r.on_hover_cursor(CursorIcon::PointingHand)
}
fn avatar(p: &Painter, pos: Pos2, initials: &str, col: [u8; 3], radius: f32) {
    p.circle_filled(pos, radius, Color32::from_rgb(col[0], col[1], col[2]));
    p.text(
        pos,
        Align2::CENTER_CENTER,
        initials,
        FontId::proportional(12.),
        INK,
    );
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<String>) -> Self {
        cc.egui_ctx.set_theme(Theme::Light);
        cc.egui_ctx.set_visuals(Visuals::light());
        cc.egui_ctx.set_fonts(app_fonts());
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut st = (*cc.egui_ctx.style()).clone();
        st.spacing.item_spacing = vec2(10., 12.);
        st.spacing.button_padding = vec2(14., 9.);
        st.spacing.interact_size.y = 34.;
        st.animation_time = 0.16;
        st.visuals.override_text_color = Some(INK);
        st.visuals.interact_cursor = Some(CursorIcon::PointingHand);
        st.visuals.extreme_bg_color = Color32::WHITE;
        for widget in [
            &mut st.visuals.widgets.inactive,
            &mut st.visuals.widgets.hovered,
            &mut st.visuals.widgets.active,
            &mut st.visuals.widgets.noninteractive,
            &mut st.visuals.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(CORNER_RADIUS);
            widget.fg_stroke = Stroke::new(1., INK);
        }
        st.visuals.widgets.hovered.bg_stroke = Stroke::new(1., Color32::from_rgb(197, 190, 223));
        st.visuals.widgets.active.bg_stroke = Stroke::new(1., PURPLE);
        st.visuals.window_fill = Color32::WHITE;
        st.visuals.window_stroke = Stroke::new(1., LINE);
        st.visuals.window_corner_radius = CornerRadius::same(CORNER_RADIUS);
        st.visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
        st.visuals.widgets.noninteractive.weak_bg_fill = Color32::WHITE;
        st.visuals.widgets.inactive.bg_fill = Color32::from_rgb(249, 248, 252);
        st.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(249, 248, 252);
        st.visuals.widgets.inactive.bg_stroke = Stroke::new(1., LINE);
        st.visuals.widgets.hovered.bg_fill = Color32::from_rgb(240, 236, 251);
        st.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(240, 236, 251);
        st.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(234, 228, 252);
        st.visuals.widgets.active.fg_stroke = Stroke::new(1., PURPLE);
        st.visuals.widgets.active.bg_fill = Color32::from_rgb(234, 228, 252);
        st.visuals.selection.bg_fill = Color32::from_rgb(229, 222, 253);
        cc.egui_ctx.set_style_of(Theme::Light, st.clone());
        cc.egui_ctx.set_style_of(Theme::Dark, st);
        let opened = data_directory().and_then(|dir| {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("Could not create mail folder: {e}"))?;
            storage::Store::open(&dir.join("sparky.sqlite3"), &dir.join("mailbox.json"))
        });
        let (store, mails, mut error) = match opened {
            Ok(store) => match store.load() {
                Ok(mut mails) => {
                    let mut changed = false;
                    for (index, mail) in mails.iter_mut().enumerate() {
                        if mail.message_id.is_empty() {
                            mail.message_id = format!("<sparky-local-{index}@local>");
                            changed = true;
                        }
                        // Older syncs cut the Date header at 16 characters
                        // ("07 Nov 2026 21:2"); store one consistent label.
                        let time = date_groups::normalize(&mail.time);
                        if time != mail.time {
                            mail.time = time;
                            changed = true;
                        }
                    }
                    let error = if changed {
                        store.save(&mails).err().unwrap_or_default()
                    } else {
                        String::new()
                    };
                    (Some(store), mails, error)
                }
                Err(error) => (
                    None,
                    Vec::new(),
                    format!("Mailbox could not be loaded: {error}"),
                ),
            },
            Err(error) => (
                None,
                Vec::new(),
                format!("Mailbox could not be opened: {error}"),
            ),
        };
        let mut credentials_error = None;
        let mut accounts = match if screenshot.is_some() { Ok(Vec::new()) } else { credentials::load() } {
            Ok(accounts) => accounts,
            Err(message) => {
                error = format!("Couldn’t load your connected accounts: {message} Sync will ask again.");
                credentials_error = Some(message);
                Vec::new()
            }
        };
        if let Some(account) = sync::Account::configured() {
            accounts.retain(|saved| saved.scope() != account.scope());
            accounts.push(account);
        }
        let account = accounts.first().cloned();
        let returning = account.is_some()
            || mails.iter().any(|mail| !mail.account.is_empty());
        let completed = store.as_ref().and_then(|store| store.onboarding_complete().ok()).unwrap_or(false);
        if returning && !completed {
            if let Some(store) = &store { let _ = store.complete_onboarding(); }
        }
        let welcome = !returning && !completed;
        let classification = store.as_ref().map(|store| store.classifier_settings()).transpose();
        let classifier_settings = match classification {
            Ok(settings) => settings.unwrap_or_default(),
            Err(message) => { error = format!("Could not load local AI settings: {message}"); Default::default() }
        };
        let theme = match store.as_ref().map(|store| store.accent()).transpose() {
            Ok(value) => value.unwrap_or_default(),
            Err(message) => { error = format!("Could not load main color: {message}"); Default::default() }
        };
        theme.apply(&cc.egui_ctx);
        let mut app = Self {
            theme,
            classifier: classifier::State { settings: classifier_settings, ..Default::default() },
            mails,
            selected: 0,
            folder: "Inbox".into(),
            category: "All".into(),
            search: String::new(),
            compose: false,
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            show_bcc: false,
            draft_signature: String::new(),
            draft_dirty_since: None,
            draft_saved: false,
            subject: String::new(),
            body: String::new(),
            palette: false,
            command_search: String::new(),
            command_index: 0,
            command_focus: false,
            keyboard: shortcuts::State::default(),
            editor: editor::Editor::default(),
            writer_focus: false,
            inline_reply: false,
            accounts,
            account_filter: None,
            pending_account: None,
            google_signin: None,
            google_setup_json: String::new(),
            connect_provider: "gmail".into(),
            connect_address: String::new(),
            connect_secret: String::new(),
            connect_imap: String::new(),
            connect_smtp: String::new(),
            connect_error: String::new(),
            toast: Toast::new(error),
            frames: 0,
            screenshot,
            saved_shot: false,
            settings: false,
            welcome,
            scroll: 0.,
            collapsed_dates: [false; 3],
            swipe: None,
            swipe_motion: Default::default(),
            swipe_last_input: 0.,
            leaving: None,
            now: 0.,
            recent_commands: Vec::new(),
            recorder: None,
            credentials_error: None,
            search_open: false,
            search_query: String::new(),
            search_index: 0,
            search_focus: false,
            recent_searches: Vec::new(),
            editing_draft: None,
            replying_to: None,
            store,
            account,
            syncing: None,
            sending: None,
        };
        app.credentials_error = credentials_error;
        // Sort mail already on this device by sender: earlier syncs filed everything
        // under People, and hand corrections are kept.
        let mut sorted = false;
        for mail in &mut app.mails {
            let before = (mail.category.clone(), mail.labels.clone());
            classifier::apply_sender_rule(&app.classifier.settings, mail);
            sorted |= before != (mail.category.clone(), mail.labels.clone());
        }
        if sorted {
            app.persist();
        }
        app.recent_commands = app.store.as_ref().and_then(|s| s.recent_commands().ok()).unwrap_or_default();
        app.recent_searches = app.store.as_ref().and_then(|s| s.recent_searches().ok()).unwrap_or_default();
        app.remove_sample_messages();
        app
    }
    fn open_search(&mut self) {
        self.palette = false;
        self.keyboard.help = false;
        self.settings = false;
        self.connect_secret.clear();
        self.search_open = true;
        // Reopening shows the active list filter so it can be refined.
        self.search_query = self.search.clone();
        self.search_index = 0;
        self.search_focus = true;
    }
    fn remember_search(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }
        self.recent_searches.retain(|q| !q.eq_ignore_ascii_case(query));
        self.recent_searches.insert(0, query.to_owned());
        self.recent_searches.truncate(6);
        if let Some(store) = &self.store {
            let _ = store.save_recent_searches(&self.recent_searches);
        }
    }
    /// Full-text hits across every folder, grouped by folder in sidebar order.
    fn search_results(&self, query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let hits: Vec<usize> = match &self.store {
            Some(store) => store.search(query).unwrap_or_default(),
            None => {
                let q = query.to_lowercase();
                (0..self.mails.len())
                    .filter(|&i| format!("{} {} {}", self.mails[i].from, self.mails[i].subject, self.mails[i].body).to_lowercase().contains(&q))
                    .collect()
            }
        };
        let order = ["Inbox", "Starred", "Snoozed", "Sent", "Drafts", "Archive", "Trash"];
        let rank = |f: &str| order.iter().position(|o| *o == f).unwrap_or(order.len());
        let mut hits: Vec<usize> = hits.into_iter().filter(|&i| i < self.mails.len() && self.account_matches(&self.mails[i])).collect();
        hits.sort_by_key(|&i| rank(&self.mails[i].folder));
        hits.truncate(60);
        hits
    }
    /// Jumps to a mail found by search: its folder, its row, and the reader.
    fn open_search_result(&mut self, idx: usize) {
        let folder = self.mails[idx].folder.clone();
        self.go_to_folder(&folder);
        self.category = "All".into();
        self.selected = idx;
        let today = date_groups::local_today();
        self.collapsed_dates[date_groups::group(&self.mails[idx].time, today)] = false;
        if let Some((_, y)) = self.expanded_rows().into_iter().find(|(i, _)| *i == idx) {
            self.scroll = (y - SECTION_H).max(0.);
        }
        self.open_selected();
    }
    fn search_modal(&mut self, ctx: &Context, t: f32) {
        let mut open = true;
        let accent = self.theme.color();
        let query = self.search_query.trim().to_owned();
        let tokens: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect();
        let results = self.search_results(&query);
        // Selectable items: results then "filter the list" when searching;
        // recent searches when the box is empty.
        let items = if query.is_empty() { self.recent_searches.len() } else { results.len() + 1 };
        let navigated = list_navigation(ctx, &mut self.search_index, items);
        let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let pointer_moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
        let top = ctx.screen_rect().height() * 0.1;
        let list_height = (ctx.screen_rect().height() - top - 200.).clamp(220., 480.);
        let mut chosen: Option<usize> = None;
        let mut filter = false;
        let mut recall: Option<String> = None;
        let mut forget: Option<String> = None;
        let medium = |size: f32| FontId::new(size, FontFamily::Name("ui-medium".into()));
        modal(ctx, "mail-search", 680., top, t, &mut open, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            // Input row: icon and text share one vertical center.
            let (row, _) = ui.allocate_exact_size(vec2(680., 60.), Sense::hover());
            icons::paint(ui.painter(), ctx, icons::Icon::Search, Rect::from_center_size(row.left_center() + vec2(28., 0.), vec2(19., 19.)), accent);
            let font = FontId::proportional(17.);
            let line = ui.fonts(|f| f.row_height(&font));
            let input = place(ui, 
                Rect::from_center_size(pos2(row.left() + 52. + (row.width() - 140.) / 2., row.center().y), vec2(row.width() - 140., line + 4.)),
                TextEdit::singleline(&mut self.search_query)
                    .id(Id::new("mail-search-modal"))
                    .font(font)
                    .margin(vec2(0., 2.))
                    .frame(false)
                    .hint_text(RichText::new("Search sender, subject or message").color(MUTED)),
            );
            if self.search_focus {
                input.request_focus();
                self.search_focus = false;
            }
            if input.changed() {
                self.search_index = 0;
            }
            paint_keycaps(ui.painter(), row.right_center() - vec2(20., 0.), "Esc", true);
            ui.painter().hline(row.x_range(), row.bottom(), Stroke::new(1., LINE));
            let section = |ui: &mut Ui, label: &str| -> Rect {
                let (h, _) = ui.allocate_exact_size(vec2(680., 32.), Sense::hover());
                ui.painter().text(pos2(h.left() + 24., h.bottom() - 10.), Align2::LEFT_CENTER, label.to_uppercase(), medium(10.5), accent);
                h
            };
            ScrollArea::vertical().max_height(list_height).auto_shrink([false, true]).show(ui, |ui| {
                if query.is_empty() {
                    if self.recent_searches.is_empty() {
                        let (r, _) = ui.allocate_exact_size(vec2(680., 150.), Sense::hover());
                        let c = r.center();
                        ui.painter().circle_filled(c - vec2(0., 26.), 22., self.theme.tint(0.08));
                        icons::paint(ui.painter(), ctx, icons::Icon::Mail, Rect::from_center_size(c - vec2(0., 26.), vec2(20., 20.)), accent);
                        ui.painter().text(c + vec2(0., 14.), Align2::CENTER_CENTER, "Search every folder at once", medium(T_BODY), INK);
                        ui.painter().text(c + vec2(0., 36.), Align2::CENTER_CENTER, "Matches sender, subject and message text. Words can be partial: “inv” finds “invoice”.", FontId::proportional(T_SMALL), MUTED);
                        return;
                    }
                    let header = section(ui, "Recent searches");
                    for (index, recent) in self.recent_searches.clone().iter().enumerate() {
                        let (slot, response) = ui.allocate_exact_size(vec2(680., 42.), Sense::click());
                        if response.hovered() && pointer_moved { self.search_index = index; }
                        let active = index == self.search_index;
                        if active && navigated {
                            ui.scroll_to_rect(if index == 0 { header.union(slot) } else { slot }, None);
                        }
                        let rect = slot.shrink2(vec2(10., 2.));
                        let p = ui.painter();
                        let active_t = icons::fade(ctx, Id::new(("recent-search", index)), active, icons::DUR_FAST);
                        p.rect_filled(rect, 8., self.theme.tint(0.09).gamma_multiply(active_t));
                        icons::paint(p, ctx, icons::Icon::Clock, Rect::from_center_size(rect.left_center() + vec2(22., 0.), vec2(15., 15.)), if active { accent } else { MUTED });
                        p.text(rect.left_center() + vec2(44., 0.), Align2::LEFT_CENTER, recent, FontId::proportional(T_BODY), INK);
                        let remove = Rect::from_center_size(rect.right_center() - vec2(22., 0.), vec2(24., 24.));
                        let x = ui.interact(remove, Id::new(("forget-search", index)), Sense::click()).on_hover_text("Remove from recent searches");
                        if active || x.hovered() {
                            icons::paint(ui.painter(), ctx, icons::Icon::X, remove.shrink(5.), if x.hovered() { INK } else { MUTED });
                        }
                        if x.clicked() {
                            forget = Some(recent.clone());
                        } else if response.clicked() || (enter && active) {
                            recall = Some(recent.clone());
                        }
                    }
                    ui.add_space(6.);
                    return;
                }
                let mut index = 0;
                let mut last_folder = "";
                for &idx in &results {
                    let m = &self.mails[idx];
                    let header = (m.folder != last_folder).then(|| section(ui, &m.folder));
                    last_folder = &self.mails[idx].folder;
                    let (slot, response) = ui.allocate_exact_size(vec2(680., 62.), Sense::click());
                    if response.hovered() && pointer_moved { self.search_index = index; }
                    let active = index == self.search_index;
                    if active && (navigated || input.changed()) {
                        ui.scroll_to_rect(header.map_or(slot, |h| h.union(slot)), None);
                    }
                    let rect = slot.shrink2(vec2(10., 2.));
                    let p = ui.painter();
                    let active_t = icons::fade(ctx, Id::new(("search-hit", idx)), active, icons::DUR_FAST);
                    p.rect_filled(rect, 8., self.theme.tint(0.09).gamma_multiply(active_t));
                    if active {
                        p.rect_filled(Rect::from_min_size(rect.left_top() + vec2(0., 14.), vec2(2.5, rect.height() - 28.)), 2., accent);
                    }
                    avatar(p, rect.left_center() + vec2(28., 0.), &m.initials, m.color, 15.);
                    let text_x = rect.left() + 56.;
                    let date_w = p.layout_no_wrap(m.time.clone(), FontId::proportional(T_CAPTION), MUTED).size().x;
                    let right = rect.right() - 14.;
                    let enter_w = if active { 30. } else { 0. };
                    p.text(pos2(right - enter_w, rect.top() + 18.), Align2::RIGHT_CENTER, &m.time, FontId::proportional(T_CAPTION), MUTED);
                    if active {
                        paint_keycaps(p, pos2(right, rect.center().y), "↵", true);
                    }
                    // Subject on top, then sender and the matching part of the body.
                    let clip = |p: &Painter, r: Rect| p.with_clip_rect(p.clip_rect().intersect(r));
                    let subject = if m.subject.trim().is_empty() { "(no subject)" } else { m.subject.as_str() };
                    let subject_rect = Rect::from_min_max(pos2(text_x, rect.top() + 8.), pos2(right - enter_w - date_w - 14., rect.top() + 28.));
                    let mut job = highlighted(subject, &token_hits(subject, &tokens), T_BODY, INK, accent);
                    job.wrap.max_rows = 1;
                    job.wrap.max_width = subject_rect.width();
                    job.wrap.break_anywhere = true;
                    let g = p.layout_job(job);
                    clip(p, subject_rect).galley(subject_rect.min, g, INK);
                    let line_rect = Rect::from_min_max(pos2(text_x, rect.top() + 33.), pos2(right - enter_w - 8., rect.top() + 53.));
                    let sender = sender_name(&m.from).to_owned();
                    let snippet = snippet(&m.body, &tokens);
                    let mut job = highlighted(&sender, &token_hits(&sender, &tokens), T_SMALL, INK, accent);
                    if !snippet.is_empty() {
                        job.append("  ·  ", 0., text::TextFormat::simple(FontId::proportional(T_SMALL), MUTED));
                        let hits = token_hits(&snippet, &tokens);
                        let more = highlighted(&snippet, &hits, T_SMALL, MUTED, accent);
                        for section in more.sections {
                            job.append(&more.text[section.byte_range.clone()], 0., section.format);
                        }
                    }
                    job.wrap.max_rows = 1;
                    job.wrap.max_width = line_rect.width();
                    job.wrap.break_anywhere = true;
                    let g = p.layout_job(job);
                    clip(p, line_rect).galley(line_rect.min, g, INK);
                    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() || (enter && active) {
                        chosen = Some(idx);
                    }
                    index += 1;
                }
                if results.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(680., 110.), Sense::hover());
                    icons::paint(ui.painter(), ctx, icons::Icon::Search, Rect::from_center_size(r.center() - vec2(0., 20.), vec2(22., 22.)), SUBTLE);
                    ui.painter().text(r.center() + vec2(0., 12.), Align2::CENTER_CENTER, format!("No mail matches “{query}”"), medium(T_BODY), INK);
                    ui.painter().text(r.center() + vec2(0., 34.), Align2::CENTER_CENTER, "Check the spelling or try fewer words.", FontId::proportional(T_SMALL), MUTED);
                }
                // Last item: keep the query as a filter on the current list.
                ui.add_space(4.);
                let (slot, response) = ui.allocate_exact_size(vec2(680., 42.), Sense::click());
                if response.hovered() && pointer_moved { self.search_index = index; }
                let active = index == self.search_index;
                if active && navigated {
                    ui.scroll_to_rect(slot, None);
                }
                let rect = slot.shrink2(vec2(10., 2.));
                let p = ui.painter();
                p.rect_filled(rect, 8., if active { self.theme.tint(0.09) } else { Color32::from_rgb(248, 248, 251) });
                icons::paint(p, ctx, icons::Icon::Search, Rect::from_center_size(rect.left_center() + vec2(22., 0.), vec2(15., 15.)), accent);
                p.text(rect.left_center() + vec2(44., 0.), Align2::LEFT_CENTER,
                    format!("Filter {} by “{query}”", self.folder), FontId::proportional(T_BODY), INK);
                if active {
                    paint_keycaps(p, pos2(rect.right() - 14., rect.center().y), "↵", true);
                }
                if response.on_hover_cursor(CursorIcon::PointingHand).clicked() || (enter && active) {
                    filter = true;
                }
                ui.add_space(6.);
            });
            // Footer legend.
            let (foot, _) = ui.allocate_exact_size(vec2(680., 40.), Sense::hover());
            let p = ui.painter();
            p.rect_filled(foot, CornerRadius { sw: 12, se: 12, ..Default::default() }, Color32::from_rgb(250, 250, 252));
            p.hline(foot.x_range(), foot.top(), Stroke::new(1., LINE));
            let mut fx = foot.left() + 18.;
            for (keys, label) in [("↑ ↓", "Navigate"), ("↵", "Open"), ("Esc", "Close")] {
                fx += paint_keycaps(p, pos2(fx, foot.center().y), keys, false) + 6.;
                let g = p.layout_no_wrap(label.into(), FontId::proportional(T_CAPTION), MUTED);
                let w = g.size().x;
                p.galley(pos2(fx, foot.center().y - g.size().y / 2.), g, MUTED);
                fx += w + 18.;
            }
            if !query.is_empty() {
                let count = match results.len() { 0 => "No results".to_owned(), 1 => "1 result".to_owned(), n if n >= 60 => "60+ results".to_owned(), n => format!("{n} results") };
                p.text(foot.right_center() - vec2(18., 0.), Align2::RIGHT_CENTER, count, FontId::proportional(T_CAPTION), MUTED);
            }
        });
        if let Some(gone) = forget {
            self.recent_searches.retain(|q| *q != gone);
            if let Some(store) = &self.store {
                let _ = store.save_recent_searches(&self.recent_searches);
            }
            self.search_index = self.search_index.min(self.recent_searches.len().saturating_sub(1));
        } else if let Some(recent) = recall {
            self.search_query = recent;
            self.search_index = 0;
            self.search_focus = true;
        } else if let Some(idx) = chosen {
            self.remember_search(&query);
            self.search_open = false;
            self.open_search_result(idx);
        } else if filter {
            self.remember_search(&query);
            self.search_open = false;
            self.search = query.clone();
            self.scroll = 0.;
        }
        if !open {
            self.search_open = false;
        }
    }
    fn open_commands(&mut self) {
        self.search_open = false;
        self.keyboard.help = false;
        self.settings = false;
        self.connect_secret.clear();
        self.palette = !self.palette;
        self.command_search.clear();
        self.command_index = 0;
        self.command_focus = true;
    }
    fn command_enabled(&self, name: &str) -> bool {
        if self.sending.is_some() {
            return matches!(
                name,
                "Commands" | "Keyboard shortcuts" | "Settings & shortcuts" | "Close / save draft"
            );
        }
        if shortcuts::COMMANDS
            .iter()
            .any(|c| c.name == name && c.scope == shortcuts::Scope::Message)
        {
            return !self.compose && self.visible().contains(&self.selected);
        }
        !self.compose
            || matches!(
                name,
                "Commands"
                    | "Keyboard shortcuts"
                    | "Settings & shortcuts"
                    | "Close / save draft"
                    | "Connect email account"
                    | "Sync all accounts"
            )
    }

    fn execute_command(&mut self, name: &str, ctx: &Context) {
        if !self.command_enabled(name) {
            return;
        }
        match name {
            "Commands" => self.open_commands(),
            "Keyboard shortcuts" => {
                self.search_open = false;
                self.keyboard.help = !self.keyboard.help;
                self.keyboard.focus_help = self.keyboard.help;
                self.keyboard.query.clear();
                self.palette = false;
                self.settings = false;
                self.connect_secret.clear();
            }
            "Settings & shortcuts" | "Connect email account" => {
                self.palette = false;
                self.keyboard.help = false;
                self.settings = true;
            }
            "Compose" => self.begin_compose(),
            "Sync all accounts" => self.start_sync(),
            "Search mail" => self.open_search(),
            "Open conversation / draft" => self.open_selected(),
            "Next conversation" | "Previous conversation" => {
                let rows = self.expanded_rows();
                let visible: Vec<_> = rows.iter().map(|(idx, _)| *idx).collect();
                if !visible.is_empty() {
                    let old = visible
                        .iter()
                        .position(|&i| i == self.selected)
                        .unwrap_or(0);
                    let next = if name == "Next conversation" {
                        (old + 1).min(visible.len() - 1)
                    } else {
                        old.saturating_sub(1)
                    };
                    self.selected = visible[next];
                    let height = (ctx.screen_rect().height() - 203.).max(ROW_STRIDE);
                    let row_y = rows[next].1;
                    // Keep the row clear of a pinned date header.
                    self.scroll = self
                        .scroll
                        .min(row_y - SECTION_H)
                        .max(row_y + ROW_STRIDE - height)
                        .max(0.);
                }
            }
            "Close / save draft" => {
                if self.search_open {
                    self.search_open = false;
                } else if self.palette {
                    self.palette = false;
                } else if self.keyboard.help {
                    self.keyboard.help = false;
                } else if self.settings {
                    self.settings = false;
                    self.connect_secret.clear();
                } else if self.keyboard.pending_g.take().is_some() {
                } else if self.compose && self.sending.is_none() {
                    self.store_composed("Drafts");
                    self.toast = Toast::new("Draft saved");
                } else {
                    self.search.clear();
                }
                if let Some(id) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
                if self.compose {
                    self.writer_focus = true;
                }
            }
            _ if name.starts_with("Go to ") => self.go_to_folder(&name[6..]),
            _ if name.starts_with("Show ") => {
                self.category = name[5..].into();
                self.scroll = 0.;
            }
            "Archive" => self.dismiss("Archive", 0.),
            "Delete" => self.dismiss("Delete", 0.),
            "Snooze" => self.dismiss("Snooze", 0.),
            "Move to Inbox" => self.dismiss("Move to Inbox", 0.),
            _ => self.action(name),
        }
    }

    fn go_to_folder(&mut self, folder: &str) {
        if self.compose && self.inline_reply && self.sending.is_none() {
            self.store_composed("Drafts");
        }
        self.folder = folder.into();
        self.category = "All".into();
        self.search.clear();
        self.scroll = 0.;
        if let Some(&first) = self.visible().first() {
            self.selected = first;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if self.welcome {
            self.keyboard.pending_g = None;
            return;
        }
        let now = ctx.input(|i| i.time);
        if self.keyboard.pending_g.is_some_and(|t| now - t > 1.5) {
            self.keyboard.pending_g = None;
        }
        if self.keyboard.help && self.keyboard.recording {
            // "Record keys": the next press only filters the guide; it never runs.
            let pressed = ctx.input(|i| i.events.iter().find_map(|e| match e {
                Event::Key { key, modifiers, pressed: true, repeat: false, .. } => Some((*key, *modifiers)),
                _ => None,
            }));
            if let Some((key, modifiers)) = pressed {
                self.keyboard.recording = false;
                if key != Key::Escape {
                    self.keyboard.recorded = Some((key, modifiers));
                    self.keyboard.query.clear();
                }
            }
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Key { .. } | Event::Text(_))));
            return;
        }
        let typing = ctx.wants_keyboard_input();
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            if let Event::Key {
                key,
                modifiers,
                pressed: true,
                repeat,
                ..
            } = event
            {
                let blocked = self.compose || self.palette || self.search_open || self.settings || self.keyboard.help;
                let pending = self.keyboard.pending_g.is_some();
                let command = self
                    .keyboard
                    .resolve(key, modifiers, repeat, now, typing, blocked);
                if let Some(command) = command {
                    ctx.input_mut(|i| {
                        i.consume_key(modifiers, key);
                        if !typing {
                            i.events.retain(|e| !matches!(e, Event::Text(_)));
                        }
                    });
                    self.execute_command(command.name, ctx);
                    // Scope changes must settle before processing another command.
                    break;
                } else if pending || self.keyboard.pending_g.is_some() {
                    ctx.input_mut(|i| {
                        i.consume_key(modifiers, key);
                        i.events.retain(|e| !matches!(e, Event::Text(_)));
                    });
                }
            }
        }
        if self.keyboard.pending_g.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(1500));
        }
    }

    fn shortcut_guide(&mut self, ctx: &Context, t: f32) {
        let mut open = true;
        let accent = self.theme.color();
        let top = (ctx.screen_rect().height() * 0.08).max(40.);
        let body_height = (ctx.screen_rect().height() - top - 230.).clamp(240., 560.);
        modal(ctx, "shortcut-guide", 780., top, t, &mut open, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            Frame::new().inner_margin(Margin { left: 24, right: 16, top: 18, bottom: 14 }).show(ui, |ui| {
                ui.set_width(740.);
                // Header.
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Keyboard shortcuts").size(18.).family(FontFamily::Name("ui-medium".into())).color(INK));
                        ui.add_space(4.);
                        ui.label(RichText::new("Letter keys work outside text fields. For “Go to”, press G, release, then the folder key.").size(T_SMALL).color(MUTED));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if icons::icon_button(ui, icons::Icon::X, 30., MUTED, Color32::from_black_alpha(12)).on_hover_text("Close · Esc").clicked() {
                            self.keyboard.help = false;
                        }
                    });
                });
                ui.add_space(14.);
                // Search + Record keys.
                let (bar, _) = ui.allocate_exact_size(vec2(740., 38.), Sense::hover());
                let search = Rect::from_min_max(bar.min, pos2(bar.right() - 148., bar.bottom()));
                let focused = ctx.memory(|m| m.has_focus(Id::new("guide-search")));
                let p = ui.painter();
                p.rect_filled(search, 8., if focused { Color32::WHITE } else { Color32::from_rgb(244, 245, 248) });
                p.rect_stroke(search, 8., Stroke::new(1., if focused { accent } else { LINE }), StrokeKind::Inside);
                icons::paint(p, ctx, icons::Icon::Search, Rect::from_center_size(search.left_center() + vec2(18., 0.), vec2(15., 15.)), MUTED);
                if let Some((key, mods)) = self.keyboard.recorded {
                    // A recorded key replaces the text query with a removable chip.
                    let desc = shortcuts::describe(key, mods);
                    let x = search.left() + 36.;
                    p.text(pos2(x, search.center().y), Align2::LEFT_CENTER, "Bound to", FontId::proportional(T_SMALL), MUTED);
                    let w = paint_keycaps(p, pos2(x + 62., search.center().y), &desc, false);
                    let clear = Rect::from_center_size(pos2(x + 62. + w + 18., search.center().y), vec2(22., 22.));
                    let r = ui.interact(clear, Id::new("guide-clear-recorded"), Sense::click()).on_hover_text("Clear key filter").on_hover_cursor(CursorIcon::PointingHand);
                    icons::paint(ui.painter(), ctx, icons::Icon::X, clear.shrink(5.), if r.hovered() { INK } else { MUTED });
                    if r.clicked() {
                        self.keyboard.recorded = None;
                        self.keyboard.focus_help = true;
                    }
                } else {
                    let response = place(ui, 
                        Rect::from_min_max(search.min + vec2(34., 4.), search.max - vec2(10., 4.)),
                        TextEdit::singleline(&mut self.keyboard.query)
                            .id(Id::new("guide-search"))
                            .frame(false)
                            .font(FontId::proportional(T_BODY))
                            .hint_text(RichText::new("Search shortcuts").color(MUTED)),
                    );
                    if self.keyboard.focus_help {
                        response.request_focus();
                        self.keyboard.focus_help = false;
                    }
                }
                let recording = self.keyboard.recording;
                let record_rect = Rect::from_min_max(pos2(bar.right() - 140., bar.top()), bar.max);
                let pulse = if recording { 0.5 + 0.5 * (ctx.input(|i| i.time) as f32 * 5.).sin() } else { 0. };
                if recording { ctx.request_repaint(); }
                if button_icon(ui, record_rect, Some(icons::Icon::Keyboard), if recording { "Press a key…" } else { "Record keys" },
                    if recording { self.theme.tint(0.12 + 0.06 * pulse) } else { Color32::WHITE }, if recording { accent } else { MUTED })
                    .on_hover_text("Press any key combination to see what it does")
                    .clicked()
                {
                    self.keyboard.recording = !recording;
                    if self.keyboard.recording {
                        ctx.memory_mut(|m| m.surrender_focus(Id::new("guide-search")));
                    }
                }
                ui.add_space(12.);
                // Group chips.
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6., 6.);
                    let chip = |ui: &mut Ui, label: &str, on: bool| -> bool {
                        let font = FontId::new(T_SMALL, FontFamily::Name("ui-medium".into()));
                        let g = ui.painter().layout_no_wrap(label.to_owned(), font, if on { accent } else { MUTED });
                        let (r, resp) = ui.allocate_exact_size(g.size() + vec2(22., 12.), Sense::click());
                        let hover = icons::fade(ui.ctx(), resp.id.with("h"), resp.hovered(), icons::DUR_FAST);
                        let fill = if on { self.theme.tint(0.12) } else { Color32::from_rgb(244, 245, 248).lerp_to_gamma(Color32::from_rgb(234, 236, 241), hover) };
                        ui.painter().rect_filled(r, r.height() / 2., fill);
                        ui.painter().galley(r.center() - g.size() / 2., g, MUTED);
                        resp.on_hover_cursor(CursorIcon::PointingHand).clicked()
                    };
                    if chip(ui, "All", self.keyboard.group.is_none()) {
                        self.keyboard.group = None;
                    }
                    for group in shortcuts::groups() {
                        if chip(ui, shortcuts::group_label(group), self.keyboard.group == Some(group)) {
                            self.keyboard.group = if self.keyboard.group == Some(group) { None } else { Some(group) };
                        }
                    }
                });
                ui.add_space(14.);
                // Filtered catalog, grouped into cards.
                let query = self.keyboard.query.trim().to_owned();
                let recorded = self.keyboard.recorded.map(|(k, m)| shortcuts::bound_to(k, m));
                let mut seen = std::collections::HashSet::new();
                let mut cards: Vec<(&'static str, Vec<(&'static str, Vec<usize>)>)> = Vec::new();
                for c in shortcuts::COMMANDS {
                    if !seen.insert(c.name) || self.keyboard.group.is_some_and(|g| g != c.group) {
                        continue;
                    }
                    if let Some(names) = &recorded {
                        if !names.contains(&c.name) { continue; }
                    }
                    let matches = if recorded.is_some() || query.is_empty() {
                        Some(vec![])
                    } else {
                        shortcuts::fuzzy(&query, c.name).map(|(_, m)| m)
                            .or_else(|| shortcuts::hints(c.name).iter().any(|h| h.to_lowercase() == query.to_lowercase()).then(Vec::new))
                    };
                    let Some(matches) = matches else { continue };
                    match cards.iter_mut().find(|(g, _)| *g == c.group) {
                        Some((_, rows)) => rows.push((c.name, matches)),
                        None => cards.push((c.group, vec![(c.name, matches)])),
                    }
                }
                ScrollArea::vertical().max_height(body_height).auto_shrink([false, true]).show(ui, |ui| {
                    if cards.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(40.);
                            ui.add(Image::new(icons::Icon::Keyboard.source()).fit_to_exact_size(vec2(28., 28.)).tint(SUBTLE));
                            ui.add_space(10.);
                            ui.label(RichText::new(if recorded.is_some() { "That key isn’t bound to anything" } else { "No shortcuts match" }).size(T_BODY).color(INK));
                            ui.add_space(4.);
                            ui.label(RichText::new("Try “draft”, “folder” or “reply”, or record a key.").size(T_SMALL).color(MUTED));
                            ui.add_space(40.);
                        });
                        return;
                    }
                    // Balance two columns by row count.
                    let mut columns: [Vec<usize>; 2] = Default::default();
                    let mut weight = [0usize; 2];
                    for (i, (_, rows)) in cards.iter().enumerate() {
                        let c = if weight[0] <= weight[1] { 0 } else { 1 };
                        columns[c].push(i);
                        weight[c] += rows.len() + 2;
                    }
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing = vec2(14., 14.);
                        for column in &columns {
                            ui.vertical(|ui| {
                                ui.set_width(358.);
                                for &i in column {
                                    let (group, rows) = &cards[i];
                                    Frame::new()
                                        .fill(Color32::from_rgb(250, 250, 252))
                                        .stroke(Stroke::new(1., LINE))
                                        .corner_radius(10)
                                        .inner_margin(Margin { left: 14, right: 12, top: 12, bottom: 8 })
                                        .show(ui, |ui| {
                                            ui.set_width(332.);
                                            ui.label(RichText::new(group.to_uppercase()).size(10.5).family(FontFamily::Name("ui-medium".into())).color(accent));
                                            ui.add_space(6.);
                                            for (name, matches) in rows {
                                                let (r, _) = ui.allocate_exact_size(vec2(332., 30.), Sense::hover());
                                                let p = ui.painter();
                                                if let Some(icon) = icons::Icon::for_name(name) {
                                                    icons::paint(p, ctx, icon, Rect::from_center_size(r.left_center() + vec2(7., 0.), vec2(14., 14.)), MUTED);
                                                }
                                                let label = p.layout_job(highlighted(name, matches, T_SMALL + 0.5, INK, accent));
                                                p.galley(pos2(r.left() + 24., r.center().y - label.size().y / 2.), label, INK);
                                                // Alternatives separated by "or", right-aligned.
                                                let mut x = r.right();
                                                for (n, hint) in shortcuts::hints(name).iter().rev().enumerate() {
                                                    if n > 0 {
                                                        p.text(pos2(x - 4., r.center().y), Align2::RIGHT_CENTER, "or", FontId::proportional(T_CAPTION), MUTED);
                                                        x -= 22.;
                                                    }
                                                    x -= paint_keycaps(p, pos2(x, r.center().y - 1.), hint, true);
                                                }
                                            }
                                        });
                                    ui.add_space(14.);
                                }
                            });
                        }
                    });
                });
            });
            // Footer.
            let (foot, _) = ui.allocate_exact_size(vec2(780., 40.), Sense::hover());
            let p = ui.painter();
            p.rect_filled(foot, CornerRadius { sw: 12, se: 12, ..Default::default() }, Color32::from_rgb(250, 250, 252));
            p.hline(foot.x_range(), foot.top(), Stroke::new(1., LINE));
            let mut fx = foot.left() + 22.;
            for (keys, label) in [("Tab", "Next control"), ("⇧ Tab", "Previous"), ("Esc", "Close")] {
                fx += paint_keycaps(p, pos2(fx, foot.center().y), keys, false) + 6.;
                let g = p.layout_no_wrap(label.into(), FontId::proportional(T_CAPTION), MUTED);
                let w = g.size().x;
                p.galley(pos2(fx, foot.center().y - g.size().y / 2.), g, MUTED);
                fx += w + 18.;
            }
            let palette = shortcuts::hint("Commands");
            p.text(foot.right_center() - vec2(22., 0.), Align2::RIGHT_CENTER, "Command palette", FontId::proportional(T_CAPTION), MUTED);
            let w = p.layout_no_wrap("Command palette".into(), FontId::proportional(T_CAPTION), MUTED).size().x;
            paint_keycaps(p, foot.right_center() - vec2(28. + w, 0.), &palette, true);
        });
        if !open {
            self.keyboard.help = false;
        }
        if !self.keyboard.help {
            self.keyboard.recording = false;
            self.keyboard.recorded = None;
        }
    }

    fn writer(&mut self, ui: &mut Ui, height: f32) {
        let shortcuts_enabled =
            self.sending.is_none() && !self.palette && !self.settings && !self.keyboard.help;
        let submit = shortcuts_enabled && shortcuts::consume(ui.ctx(), "Send / save locally");
        let discard = shortcuts_enabled && shortcuts::consume(ui.ctx(), "Discard draft");
        let command = if cfg!(target_os = "macos") {
            "⌘"
        } else {
            "Ctrl"
        };
        ui.add_enabled_ui(self.sending.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("From").small().color(MUTED));
                let mut chosen = self.account.as_ref().map(|a| a.scope()).unwrap_or_default();
                ComboBox::from_id_salt("send-account")
                    .selected_text(
                        self.account
                            .as_ref()
                            .map(|a| a.address.as_str())
                            .unwrap_or("Local mailbox"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut chosen,
                            String::new(),
                            "Local mailbox (save on this device)",
                        );
                        for a in &self.accounts {
                            ui.selectable_value(&mut chosen, a.scope(), &a.address);
                        }
                    });
                self.account = self.accounts.iter().find(|a| a.scope() == chosen).cloned();
            });
            ui.add_space(8.);
            self.editor
                .show(ui, &mut self.body, &mut self.writer_focus, height);
            ui.separator();
            ui.horizontal(|ui| {
                let can_submit = !self.to.trim().is_empty() && !self.body.trim().is_empty();
                if ui
                    .add_enabled(
                        can_submit,
                        Button::new(
                            medium_label(if self.account.is_some() {
                                "Send"
                            } else {
                                "Save locally"
                            })
                            .color(Color32::WHITE),
                        )
                        .shortcut_text(medium_label(&format!("{command} Return")).color(Color32::WHITE))
                        .fill(self.theme.color()),
                    )
                    .clicked()
                    || (submit && can_submit)
                {
                    if let Some(account) = self.account.clone() {
                        let (to, cc, bcc, subject, body, html) = (
                            self.to.clone(),
                            self.cc.clone(),
                            self.bcc.clone(),
                            self.subject.clone(),
                            self.body.clone(),
                            editor::html(&self.body, &self.editor.marks),
                        );
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.sending = Some(rx);
                        self.toast = Toast::new("Sending…");
                        std::thread::spawn(move || {
                            let _ = tx.send(account.send(&to, &cc, &bcc, &subject, &body, &html));
                        });
                    } else {
                        self.store_composed("Sent");
                        self.toast = Toast::new("Message saved locally to Sent");
                    }
                }
                if ui
                    .add(Button::new(medium_label("Save draft")).shortcut_text(medium_label("Esc")))
                    .clicked()
                {
                    self.store_composed("Drafts");
                    self.toast = Toast::new("Draft saved");
                }
                if ui
                    .add(
                        Button::new(medium_label("Discard"))
                            .shortcut_text(medium_label(&format!("{command} ⇧ ⌫"))),
                    )
                    .clicked()
                    || discard
                {
                    if let Some(index) = self.editing_draft.take() {
                        self.mails.remove(index);
                        self.selected = self.selected.min(self.mails.len().saturating_sub(1));
                        self.persist();
                    }
                    self.compose = false;
                    self.inline_reply = false;
                    self.body.clear();
                }
                if self.sending.is_some() {
                    ui.spinner();
                    ui.label("Sending…");
                }
            });
        });
    }
    /// Why a command can't run right now, shown in the palette instead of a dead row.
    fn command_disabled_reason(&self, name: &str) -> &'static str {
        if self.command_enabled(name) {
            ""
        } else if self.sending.is_some() {
            "Sending…"
        } else if self.compose {
            "Close the writer first"
        } else {
            "Select a message first"
        }
    }
    /// Palette entries `(name, group)` in catalog order; writer-only commands are omitted.
    fn palette_commands() -> Vec<(&'static str, &'static str)> {
        let mut all = vec![("Connect email account", "Accounts")];
        for c in shortcuts::COMMANDS {
            if matches!(c.scope, shortcuts::Scope::Writer | shortcuts::Scope::Editor) || c.name == "Close / save draft" {
                continue;
            }
            if !all.iter().any(|(name, _)| *name == c.name) {
                all.push((c.name, shortcuts::group_label(c.group)));
            }
        }
        all
    }
    fn remember_command(&mut self, name: &str) {
        self.recent_commands.retain(|n| n != name);
        self.recent_commands.insert(0, name.to_owned());
        self.recent_commands.truncate(5);
        if let Some(store) = &self.store {
            let _ = store.save_recent_commands(&self.recent_commands);
        }
    }
    fn command_palette(&mut self, ctx: &Context, t: f32) {
        let mut open = true;
        let accent = self.theme.color();
        let query = self.command_search.trim().to_owned();
        // Rows: (section header, (name, group, matched char indices)).
        struct Row { header: Option<&'static str>, name: &'static str, group: &'static str, matches: Vec<usize> }
        let commands = Self::palette_commands();
        let mut rows: Vec<Row> = Vec::new();
        if query.is_empty() {
            let recent: Vec<_> = self.recent_commands.iter()
                .filter_map(|r| commands.iter().find(|(n, _)| n == r).copied())
                .collect();
            for (i, (name, group)) in recent.iter().enumerate() {
                rows.push(Row { header: (i == 0).then_some("Recently used"), name, group, matches: vec![] });
            }
            let mut last = "";
            for &(name, group) in &commands {
                if recent.iter().any(|(n, _)| *n == name) { continue; }
                rows.push(Row { header: (group != last).then_some(group), name, group, matches: vec![] });
                last = group;
            }
        } else {
            let mut scored: Vec<(i32, Row)> = commands.iter().filter_map(|&(name, group)| {
                shortcuts::fuzzy(&query, name).map(|(score, matches)| (score, Row { header: None, name, group, matches }))
                    .or_else(|| shortcuts::fuzzy(&query, group).map(|(score, _)| (score - 20, Row { header: None, name, group, matches: vec![] })))
            }).collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            rows = scored.into_iter().map(|(_, r)| r).collect();
        }
        // Keys are consumed here, so remember that the selection moved by keyboard
        // to scroll it into view below.
        let navigated = !rows.is_empty() && list_navigation(ctx, &mut self.command_index, rows.len());
        let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let pointer_moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
        let top = ctx.screen_rect().height() * 0.12;
        let mut execute = None;
        let mut open_guide = false;
        modal(ctx, "command-palette", 640., top, t, &mut open, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            // Input row.
            let (row, _) = ui.allocate_exact_size(vec2(640., 58.), Sense::hover());
            icons::paint(ui.painter(), ctx, icons::Icon::Search, Rect::from_center_size(row.left_center() + vec2(26., 0.), vec2(18., 18.)), MUTED);
            let input = place(ui, 
                Rect::from_min_max(row.min + vec2(48., 14.), row.max - vec2(70., 14.)),
                TextEdit::singleline(&mut self.command_search)
                    .font(FontId::proportional(17.))
                    .frame(false)
                    .hint_text(RichText::new("Type a command").color(MUTED)),
            );
            if self.command_focus {
                input.request_focus();
                self.command_focus = false;
            }
            if input.changed() {
                self.command_index = 0;
            }
            paint_keycaps(ui.painter(), row.right_center() - vec2(20., 0.), "Esc", true);
            ui.painter().hline(row.x_range(), row.bottom(), Stroke::new(1., LINE));
            // Results.
            if rows.is_empty() {
                let (r, _) = ui.allocate_exact_size(vec2(640., 120.), Sense::hover());
                icons::paint(ui.painter(), ctx, icons::Icon::Search, Rect::from_center_size(r.center() - vec2(0., 18.), vec2(22., 22.)), SUBTLE);
                ui.painter().text(r.center() + vec2(0., 14.), Align2::CENTER_CENTER, format!("No commands match “{query}”"), FontId::proportional(T_BODY), INK);
                ui.painter().text(r.center() + vec2(0., 36.), Align2::CENTER_CENTER, "Try “compose”, “archive” or “go to”", FontId::proportional(T_SMALL), MUTED);
            } else {
                ScrollArea::vertical().max_height(400.).auto_shrink([false, true]).show(ui, |ui| {
                    ui.add_space(6.);
                    for (index, item) in rows.iter().enumerate() {
                        let header_rect = item.header.map(|header| {
                            let (h, _) = ui.allocate_exact_size(vec2(640., if index == 0 { 26. } else { 32. }), Sense::hover());
                            ui.painter().text(pos2(h.left() + 22., h.bottom() - 9.), Align2::LEFT_CENTER, header.to_uppercase(),
                                FontId::new(10.5, FontFamily::Name("ui-medium".into())), MUTED);
                            h
                        });
                        let (slot, response) = ui.allocate_exact_size(vec2(640., 38.), Sense::click());
                        let disabled = !self.command_enabled(item.name);
                        if response.hovered() && pointer_moved {
                            self.command_index = index;
                        }
                        let active = index == self.command_index;
                        if active && (navigated || input.changed()) {
                            // Bring the row, and its section header if it starts one, into view.
                            ui.scroll_to_rect(header_rect.map_or(slot, |h| h.union(slot)), None);
                        }
                        let rect = slot.shrink2(vec2(8., 1.));
                        let p = ui.painter();
                        let active_t = icons::fade(ctx, Id::new(("palette-row", item.name)), active, icons::DUR_FAST);
                        p.rect_filled(rect, 8., self.theme.tint(0.09).gamma_multiply(active_t));
                        if active {
                            p.rect_filled(Rect::from_min_size(rect.left_top() + vec2(0., 9.), vec2(2.5, 20.)), 2., accent);
                        }
                        let dim = if disabled { 0.5 } else { 1. };
                        let tile = Rect::from_center_size(rect.left_center() + vec2(24., 0.), vec2(26., 26.));
                        p.rect_filled(tile, 7., if active { self.theme.tint(0.16) } else { Color32::from_rgb(243, 244, 247) });
                        if let Some(icon) = icons::Icon::for_name(item.name) {
                            icons::paint(p, ctx, icon, Rect::from_center_size(tile.center(), vec2(15., 15.)),
                                (if active { accent } else { MUTED }).gamma_multiply(dim));
                        }
                        let label = p.layout_job(highlighted(item.name, &item.matches, T_BODY, INK.gamma_multiply(dim), accent));
                        let label_w = label.size().x;
                        p.galley(pos2(tile.right() + 12., rect.center().y - label.size().y / 2.), label, INK);
                        // The group is context; skip it when the name already says it ("Go to Inbox").
                        let side = if disabled { self.command_disabled_reason(item.name) } else if item.name.starts_with(item.group) { "" } else { item.group };
                        p.text(pos2(tile.right() + 24. + label_w, rect.center().y), Align2::LEFT_CENTER, side, FontId::proportional(T_SMALL), MUTED.gamma_multiply(if disabled { 1. } else { 0.85 }));
                        let mut kx = rect.right() - 12.;
                        if active && !disabled {
                            kx -= paint_keycaps(p, pos2(kx, rect.center().y), "↵", true) + 10.;
                        }
                        let hint = shortcuts::hint(item.name);
                        if !hint.is_empty() {
                            paint_keycaps(p, pos2(kx, rect.center().y), &hint, true);
                        }
                        let response = response.on_hover_cursor(if disabled { CursorIcon::NotAllowed } else { CursorIcon::PointingHand });
                        if !disabled && (response.clicked() || (enter && active)) {
                            execute = Some(item.name);
                        }
                    }
                    ui.add_space(6.);
                });
            }
            // Footer legend.
            let (foot, _) = ui.allocate_exact_size(vec2(640., 40.), Sense::hover());
            let p = ui.painter();
            p.rect_filled(foot, CornerRadius { sw: 12, se: 12, ..Default::default() }, Color32::from_rgb(250, 250, 252));
            p.hline(foot.x_range(), foot.top(), Stroke::new(1., LINE));
            let mut fx = foot.left() + 18.;
            for (keys, label) in [("↑ ↓", "Navigate"), ("↵", "Run"), ("Esc", "Close")] {
                fx += paint_keycaps(p, pos2(fx, foot.center().y), keys, false) + 6.;
                let g = p.layout_no_wrap(label.into(), FontId::proportional(T_CAPTION), MUTED);
                let w = g.size().x;
                p.galley(pos2(fx, foot.center().y - g.size().y / 2.), g, MUTED);
                fx += w + 18.;
            }
            let guide_hint = shortcuts::hint("Keyboard shortcuts");
            let guide_rect = Rect::from_min_max(pos2(foot.right() - 170., foot.top() + 6.), foot.max - vec2(10., 6.));
            let guide = ui.interact(guide_rect, Id::new("palette-guide"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let p = ui.painter();
            if guide.hovered() {
                p.rect_filled(guide_rect, 6., Color32::from_black_alpha(8));
            }
            p.text(guide_rect.right_center() - vec2(8., 0.), Align2::RIGHT_CENTER, "All shortcuts", FontId::proportional(T_CAPTION), MUTED);
            let w = p.layout_no_wrap("All shortcuts".into(), FontId::proportional(T_CAPTION), MUTED).size().x;
            paint_keycaps(p, guide_rect.right_center() - vec2(14. + w, 0.), &guide_hint, true);
            open_guide = guide.clicked();
        });
        if let Some(name) = execute {
            self.palette = false;
            self.remember_command(name);
            self.execute_command(name, ctx);
        } else if open_guide {
            self.execute_command("Keyboard shortcuts", ctx);
        }
        if !open {
            self.palette = false;
        }
    }
    fn welcome_modal(&mut self, ctx: &Context) {
        Modal::new(Id::new("connect-welcome"))
            .frame(glass_frame())
            .show(ctx, |ui| {
                paper_frame().show(ui, |ui| {
                    ui.set_width(450.);
                    ui.add_space(8.);
                    ui.label(RichText::new("Welcome to Sparky").color(self.theme.color()).strong());
                    ui.add_space(8.);
                    ui.heading("Connect your mail");
                    ui.label("All your email. One calm place.");
                    ui.add_space(22.);
                    for (provider, label) in [
                        ("gmail", "Gmail"),
                        ("graph", "Outlook / Microsoft 365"),
                        ("imap", "Other email · IMAP"),
                    ] {
                        let (rect, response) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 60.), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(
                            rect,
                            CORNER_RADIUS,
                            if response.hovered() {
                                self.theme.tint(0.07)
                            } else {
                                Color32::from_rgb(249, 249, 252)
                            },
                        );
                        p.rect_stroke(
                            rect,
                            CORNER_RADIUS,
                            Stroke::new(1., LINE),
                            StrokeKind::Inside,
                        );
                        let at = rect.min + vec2(18., 18.);
                        provider_logo(p, at, provider);
                        p.text(
                            rect.min + vec2(64., 30.),
                            Align2::LEFT_CENTER,
                            label,
                            FontId::proportional(16.),
                            INK,
                        );
                        p.text(
                            rect.right_center() - vec2(22., 0.),
                            Align2::CENTER_CENTER,
                            "→",
                            FontId::proportional(20.),
                            self.theme.color(),
                        );
                        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            self.connect_provider = provider.into();
                            self.connect_secret.clear();
                            self.connect_error.clear();
                            self.welcome = false;
                            self.settings = true;
                        }
                        ui.add_space(8.);
                    }
                    ui.add_space(8.);
                    ui.separator();
                    ui.add_space(10.);
                    if ui
                        .add_sized(
                            [ui.available_width(), 44.],
                            Button::new(RichText::new("Check local demo →").color(self.theme.color())),
                        )
                        .clicked()
                    {
                        if self.mails.is_empty() {
                            self.mails = sample();
                            self.persist();
                        }
                        self.welcome = false;
                        self.toast = Toast::new("Local demo · Explore without connecting an account");
                    }
                    ui.label(
                        RichText::new(
                            "Try the inbox and reply editor with mail stored on this device.",
                        )
                        .small()
                        .color(MUTED),
                    );
                    ui.add_space(8.);
                });
            });
    }
    fn account_settings(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Accounts & settings").vscroll(true).default_height(700.).max_height((ctx.screen_rect().height() - 100.).max(300.)).frame(glass_frame()).open(&mut open).collapsible(false).resizable(false).default_width(560.).anchor(Align2::CENTER_CENTER, vec2(0.,0.)).show(ctx, |ui| { paper_frame().show(ui, |ui| {
            ui.label(RichText::new("Appearance").strong());
            ui.label("Main color");
            ui.label("Choose a readable color for buttons and highlights.");
            ui.horizontal_wrapped(|ui| {
                for preset in theme::Accent::ALL {
                    let selected = self.theme == preset;
                    let label = if selected { format!("✓ {}", preset.name()) } else { preset.name().into() };
                    let response = ui.add(Button::new(RichText::new(label).color(Color32::WHITE))
                        .fill(preset.color()).stroke(Stroke::new(if selected { 2. } else { 1. }, if selected { INK } else { preset.color() })));
                    if response.clicked() && !selected {
                        let saved = self.store.as_ref().ok_or_else(|| "Mailbox storage is unavailable".to_owned()).and_then(|store| store.save_accent(preset));
                        match saved {
                            Ok(()) => { self.theme = preset; preset.apply(ctx); self.toast = Toast::new(format!("Main color changed to {}", preset.name())); }
                            Err(error) => self.toast = Toast::new(format!("Could not save main color: {error}")),
                        }
                    }
                }
            });
            ui.add_space(4.);
            ui.separator();
            ui.heading("All your email. One calm place.");
            ui.label("Connect multiple accounts to your unified inbox.");
            ui.add_space(8.);
            let mut remove = None;
            for (index, account) in self.accounts.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(self.theme.color()));
                    ui.label(format!("{} · {} · {}", account.display_name(), account.address, account.provider));
                    if ui.add_enabled(self.syncing.is_none() && self.sending.is_none(), Button::new("Disconnect")).clicked() { remove = Some(index); }
                });
            }
            if let Some(index) = remove {
                let mut remaining = self.accounts.clone();
                let removed = remaining.remove(index);
                if let Err(error) = credentials::save(&remaining) {
                    self.toast = Toast::new(error);
                    return;
                }
                self.accounts = remaining;
                if self.account.as_ref().is_some_and(|a| a.scope() == removed.scope()) { self.account = self.accounts.first().cloned(); }
                self.toast = Toast::new("Account disconnected. Downloaded mail stays on this device.");
            }
            if self.accounts.is_empty() { ui.label(RichText::new("No accounts connected yet").color(MUTED)); }
            if ui.add_enabled(!self.accounts.is_empty() && self.syncing.is_none(), Button::new("Sync all accounts")).clicked() { self.start_sync(); }
            ui.separator();
            self.classifier_settings_ui(ui);
            ui.separator();
            ui.label(RichText::new("Add an email account").strong());
            ui.add_enabled_ui(self.syncing.is_none() && self.sending.is_none() && self.google_signin.is_none(), |ui| {
                ui.horizontal(|ui| {
                    for (id,label) in [("gmail","Gmail"),("graph","Outlook / Microsoft 365"),("imap","Other · IMAP")] {
                        if ui.selectable_value(&mut self.connect_provider, id.into(), label).changed() { self.connect_secret.clear(); self.connect_error.clear(); }
                    }
                });
                if self.connect_provider == "gmail" {
                    ui.add_space(10.);
                    ui.label("Sign in securely in your browser and choose your Google account.");
                    ui.label(RichText::new("Use your Google profile name, read your inbox and send mail when you click Send.").small().color(MUTED));
                    let google_ready = oauth::configured();
                    if ui.add_enabled(google_ready, Button::new(RichText::new("Continue with Google →").color(self.theme.color()))).clicked() {
                        self.connect_error.clear();
                        let (tx, rx) = std::sync::mpsc::channel();
                        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                        self.google_signin = Some((rx, cancel.clone()));
                        let sign_in_ctx = ctx.clone();
                        std::thread::spawn(move || {
                            let result = oauth::connect(cancel, || {
                                sign_in_ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                                sign_in_ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                                sign_in_ctx.send_viewport_cmd(ViewportCommand::Focus);
                                sign_in_ctx.request_repaint();
                            });
                            let _ = tx.send(result);
                            sign_in_ctx.request_repaint();
                        });
                    }
                    if !google_ready {
                        ui.label(RichText::new("Google sign-in is not configured for this copy of Sparky.").small().color(MUTED));
                        ui.collapsing("Set up Google sign-in · app owner", |ui| {
                            ui.label("Mail apps register with Google before people can sign in. Configure Sparky once, then package it to share browser sign-in with everyone.");
                            ui.hyperlink_to("Open Google Cloud setup", "https://console.cloud.google.com/apis/credentials");
                            ui.label("Enable the Gmail API, configure the consent screen for Gmail reading and sending, and create a Desktop app OAuth client. Add test users while testing; public distribution requires Google's verification.");
                            ui.label("Download the client's JSON file, open it in TextEdit, and paste its contents here.");
                            ui.add(TextEdit::multiline(&mut self.google_setup_json).desired_rows(4).desired_width(f32::INFINITY).hint_text("Paste Google Desktop app JSON"));
                            if ui.button("Save Google setup").clicked() {
                                match oauth::save_configuration(&self.google_setup_json) {
                                    Ok(()) => {
                                        self.google_setup_json.clear();
                                        self.connect_error.clear();
                                        self.toast = Toast::new("Google setup saved. Continue with Google, or package the app to share it.");
                                    }
                                    Err(error) => self.connect_error = error,
                                }
                            }
                        });
                    }
                } else {
                ui.label("Email address");
                ui.add(form_field(&mut self.connect_address).hint_text("you@example.com"));
                ui.label(if self.connect_provider == "imap" { "App password" } else { "OAuth access token" });
                ui.add(form_field(&mut self.connect_secret).password(true));
                if self.connect_provider == "imap" {
                    ui.label("IMAP host · TLS");
                    ui.add(form_field(&mut self.connect_imap).hint_text("imap.example.com:993"));
                    ui.label("SMTP host · TLS");
                    ui.add(form_field(&mut self.connect_smtp).hint_text("smtp.example.com:465"));
                    ui.label(RichText::new("Use an app password from your provider’s security settings.").small().color(MUTED));
                } else {
                    ui.label(RichText::new("Use a Microsoft Graph delegated token with Mail.Read and Mail.Send permissions.").small().color(MUTED));
                    ui.label(RichText::new("Browser sign-in and automatic token renewal aren’t available yet.").small().color(MUTED));
                }
                if ui.button(RichText::new("Connect & check inbox →").color(self.theme.color())).clicked() {
                    match sync::Account::new(self.connect_provider.clone(), self.connect_address.clone(), self.connect_secret.clone(), self.connect_imap.trim().into(), self.connect_smtp.trim().into()) {
                        Ok(account) => {
                            self.connect_error.clear();
                            self.pending_account = Some(account.clone());
                            let (tx, rx) = std::sync::mpsc::channel();
                            self.syncing = Some(rx);
                            std::thread::spawn(move || { let _ = tx.send(account.fetch().map(sync::SyncBatch::from)); });
                        },
                        Err(error) => self.connect_error = error,
                    }
                }
                }
            });
            if self.google_signin.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Waiting for Google sign-in in your browser…");
                    if ui.button("Cancel sign-in").clicked() { self.cancel_google_signin(); }
                });
            }
            if self.syncing.is_some() { ui.horizontal(|ui| {ui.spinner(); ui.label(if self.pending_account.is_some() { "Checking connection and importing inbox…" } else { "Connected · Downloading inbox in the background…" });}); }
            if !self.connect_error.is_empty() { ui.colored_label(Color32::from_rgb(180,65,65), &self.connect_error); }
            ui.label(RichText::new("Connected accounts are saved securely in macOS Keychain.").small().color(MUTED));
            ui.add_space(16.);
            ui.separator();
            ui.add_space(12.);
            ui.label(RichText::new("Keyboard shortcuts").size(13.).strong().color(INK));
            ui.add_space(10.);
            ui.label("Press ? for the complete guide. Use G, then a letter to switch folders.");
            if ui.button(format!("Open keyboard guide   {}", shortcuts::hint("Keyboard shortcuts"))).clicked() {
                self.settings = false;
                self.connect_secret.clear();
                self.keyboard.help = true;
                self.keyboard.focus_help = true;
            }

        }); });
        if !open {
            self.cancel_google_signin();
            self.settings = false;
            self.connect_secret.clear();
        }
    }
    fn begin_compose(&mut self) {
        if self.sending.is_some() {
            return;
        }
        if self.compose {
            self.store_composed("Drafts");
        }
        self.compose = true;
        self.inline_reply = false;
        if let Some(account) = self.account_filter.as_ref()
            .and_then(|scope| self.accounts.iter().find(|a| a.scope() == *scope)) {
            self.account = Some(account.clone());
        } else if self.account.is_none() {
            self.account = self.accounts.first().cloned();
        }
        self.writer_focus = true;
        self.editor.reset(Vec::new());
        self.to.clear();
        self.cc.clear();
        self.bcc.clear();
        self.show_bcc = false;
        self.draft_signature.clear();
        self.draft_dirty_since = None;
        self.draft_saved = false;
        self.subject.clear();
        self.body.clear();
        self.editing_draft = None;
        self.replying_to = None;
    }
    fn store_composed(&mut self, folder: &str) {
        self.save_composed(folder);
        self.editing_draft = None;
        self.compose = false;
        self.inline_reply = false;
    }
    fn save_composed(&mut self, folder: &str) {
        let mail = Mail {
            from: if folder == "Sent" {
                format!("Me → {}", self.to)
            } else {
                self.to.clone()
            },
            initials: "JL".into(),
            cc: self.cc.clone(),
            bcc: self.bcc.clone(),
            subject: self.subject.clone(),
            preview: self.body.chars().take(65).collect(),
            body: self.body.clone(),
            marks: self.editor.marks.clone(),
            time: "Now".into(),
            category: "People".into(),
            labels: Vec::new(),
            folder: folder.into(),
            unread: false,
            star: false,
            color: [225, 219, 244],
            snoozed_until: None,
            account: self
                .account
                .as_ref()
                .map(|a| a.scope())
                .or_else(|| self.replying_to.as_ref().map(|m| m.account.clone()))
                .unwrap_or_default(),
            remote_id: String::new(),
            remote_thread: self
                .replying_to
                .as_ref()
                .map(|m| m.remote_thread.clone())
                .unwrap_or_default(),
            message_id: format!(
                "<sparky-{}-{}@local>",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            references: self
                .replying_to
                .as_ref()
                .map(|m| {
                    let mut refs = m.references.clone();
                    if !m.message_id.is_empty() {
                        refs.push(m.message_id.clone());
                    }
                    refs
                })
                .unwrap_or_default(),
        };
        if let Some(idx) = self.editing_draft {
            self.mails[idx] = mail;
        } else {
            self.editing_draft = Some(self.mails.len());
            self.mails.push(mail);
        }
        self.draft_saved = match &self.store {
            Some(store) => match store.save(&self.mails) {
                Ok(()) => true,
                Err(error) => {
                    self.toast = Toast::new(format!("Changes could not be saved: {error}"));
                    false
                }
            },
            None => {
                self.toast = Toast::new("Storage unavailable. Changes cannot be saved.");
                false
            }
        };
    }
    fn wake_snoozed(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut changed = false;
        for m in &mut self.mails {
            if m.folder == "Snoozed" && m.snoozed_until.is_some_and(|t| t <= now) {
                m.folder = "Inbox".into();
                m.snoozed_until = None;
                m.unread = true;
                changed = true;
            }
        }
        if changed {
            self.persist();
        }
    }
    fn persist(&mut self) {
        match &self.store {
            Some(store) => {
                if let Err(error) = store.save(&self.mails) {
                    self.toast = Toast::new(format!("Changes could not be saved: {error}"));
                }
            }
            None => self.toast = Toast::new("Storage unavailable. Changes cannot be saved."),
        }
    }
    fn learn_classification(&mut self, index: usize) {
        self.classifier.settings.learn(&self.mails[index]);
        self.persist();
        if let Some(store) = &self.store {
            if let Err(error) = store.save_classifier_settings(&self.classifier.settings) {
                self.toast = Toast::new(format!("Could not save learning from this correction: {error}"));
                return;
            }
        }
        self.toast = Toast::new("Correction remembered. Similar emails will use it as an example.");
    }
    fn classifier_settings_ui(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("AI model").strong());
        ui.label("Apply Work, Personal, Ideas, Social Media, Subscriptions and Banking labels and organize inbox categories.");
        ui.collapsing("How does our classifier work?", |ui| {
            ui.label(RichText::new("1. Read one email").strong());
            ui.add(Label::new("Pinterest emails always receive Social Media and go into Notifications. For other emails, saved manual corrections are applied first. Clear account suspension, cancellation and renewal alerts from recognized subscription services (such as Disney+) receive Subscriptions directly. For other emails, Sparky gives your selected AI model the sender, subject and a shortened email body, plus relevant examples from your corrections for this account when space allows. Long text and quoted replies are trimmed. Saved examples contain sender, subject and your chosen category and labels, without email bodies.").wrap());
            ui.label(RichText::new("2. Choose a category and labels").strong());
            ui.add(Label::new("The model judges the content using fixed instructions: People for human messages, Newsletters for bulk mail and promotions, or Notifications for alerts and receipts. It can also add Work, Personal, Ideas, Social Media, Subscriptions and Banking labels, or leave labels empty when unclear.").wrap());
            ui.add(Label::new("For example, every Pinterest digest receives Social Media and goes into Notifications. A colleague's project email may receive Work and go into People. Model predictions for other senders can be wrong.").wrap());
            ui.label(RichText::new("3. Check and save the answer").strong());
            ui.add(Label::new("Sparky first tries existing categories and approved labels. Results at 75% confidence or above are applied; lower confidence results show editable criteria below for your approval. Invalid answers leave the email unchanged. Accepted results are saved in Sparky; nothing is written back to Gmail, Outlook or IMAP. Manual changes made while the model is working are respected.").wrap());
            ui.label(RichText::new("Where does the email go?").strong());
            ui.add(Label::new("Apple Foundation Model runs on this Mac. Ollama also connects to this Mac. My own server sends the email text to the address you configure, which may be another computer or a cloud service. Sparky does not automatically switch to another provider.").wrap());
            ui.label(RichText::new("How to use it").strong());
            ui.add(Label::new("Choose a model below and use Test model connection. Turn on automatic labeling for emails newly downloaded during sync. For existing mail, use Auto-classify inbox for the selected account, or Auto-classify on one message. The inbox action includes all categories and ignores the search filter.").wrap());
            ui.add(Label::new("Correct mistakes with the message's category and label menu. Sparky automatically remembers your corrections across restarts, keeps your choices when classifying the same email again (except the fixed Pinterest rule), and uses relevant examples to improve similar emails. This teaches your preferences without retraining the model. Examples go to your selected provider with the email. Cancel keeps finished results and ignores the current and queued work. A connection or model error stops the batch; retry after fixing it.").wrap());
        });
        ui.collapsing("Built-in label criteria", |ui| {
            ui.add(Label::new(format!("Subscriptions — {}", classifier::SUBSCRIPTIONS_CRITERIA)).wrap());
            ui.add(Label::new(format!("Banking — {}", classifier::BANKING_CRITERIA)).wrap());
        });
        let before = serde_json::to_string(&self.classifier.settings).unwrap_or_default();
        ui.label(format!("Learning from {} corrected emails", self.classifier.settings.corrections.len()));
        self.classifier_reviews_ui(ui);
        ui.add_enabled_ui(!self.classifier.busy() && self.classifier.connection.is_none(), |ui| {
            let previous = self.classifier.settings.provider;
            ComboBox::from_id_salt("ai_provider")
                .selected_text(self.classifier.settings.provider.name())
                .show_ui(ui, |ui| {
                    for provider in [classifier::Provider::Apple, classifier::Provider::Ollama, classifier::Provider::Custom] {
                        ui.selectable_value(&mut self.classifier.settings.provider, provider, provider.name());
                    }
                });
            if previous != self.classifier.settings.provider { self.classifier.status.clear(); }
            match self.classifier.settings.provider {
                classifier::Provider::Apple => {
                    ui.label("Default · on-device Apple Foundation Model. No server or model download to set up in Sparky.");
                    ui.label(RichText::new("Requires macOS 26 or later, a supported Mac and Apple Intelligence enabled in System Settings. Email stays on this Mac.").small().color(MUTED));
                }
                classifier::Provider::Ollama => {
                    ui.label("Open Ollama on this Mac and download a model, then enter its name below. Email stays on this Mac.");
                    ui.collapsing("Set up Ollama", |ui| {
                        ui.hyperlink_to("Download Ollama for Mac", "https://ollama.com/download/mac");
                        ui.label("After installing and opening Ollama, run this in Terminal:");
                        ui.monospace("ollama pull llama3.2");
                        if ui.button("Copy Ollama command").clicked() { ui.ctx().copy_text("ollama pull llama3.2".into()); }
                    });
                    ui.horizontal(|ui| { ui.label("Ollama address"); ui.text_edit_singleline(&mut self.classifier.settings.ollama_endpoint); });
                    ui.horizontal(|ui| { ui.label("Model"); ui.text_edit_singleline(&mut self.classifier.settings.ollama_model); });
                }
                classifier::Provider::Custom => {
                    ui.label("Connect an OpenAI-compatible server, including MLX. Use its base URL ending in /v1 and its model name.");
                    ui.label(RichText::new("Email text is sent to the server you choose. Use HTTPS for a remote server.").small().color(MUTED));
                    ui.horizontal(|ui| { ui.label("Server address"); ui.text_edit_singleline(&mut self.classifier.settings.endpoint); });
                    ui.horizontal(|ui| { ui.label("Model"); ui.text_edit_singleline(&mut self.classifier.settings.model); });
                    ui.horizontal(|ui| { ui.label("API key (optional)"); ui.add(TextEdit::singleline(&mut self.classifier.settings.api_key).password(true)); });
                    ui.label(RichText::new("API keys are kept only until Sparky closes. Enter the key again after restarting. MLX can use default_model.").small().color(MUTED));
                }
            }
            ui.checkbox(&mut self.classifier.settings.automatic, "Automatically label newly downloaded emails");
            {
                ui.horizontal(|ui| {
                    ui.label("Model context (tokens)");
                    ui.add(DragValue::new(&mut self.classifier.settings.context).range(1024..=8192).speed(128));
                });
            }
            ui.label(RichText::new("One email per request. Long bodies and quoted replies are trimmed; uncertain results keep their current label. Labels are saved locally.").small().color(MUTED));
            if ui.button("Test model connection").clicked() {
                let settings = self.classifier.settings.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.classifier.connection = Some(rx);
                self.classifier.status = "Checking selected model…".into();
                std::thread::spawn(move || { let _ = tx.send(classifier::test(&settings)); });
            }
        });
        if before != serde_json::to_string(&self.classifier.settings).unwrap_or_default() {
            if let Some(store) = &self.store {
                if let Err(error) = store.save_classifier_settings(&self.classifier.settings) { self.classifier.status = format!("Settings could not be saved: {error}"); }
            }
        }
        if self.classifier.busy() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Classifying {} of {}…", self.classifier.done + 1, self.classifier.total));
                if ui.button("Cancel classification").clicked() { self.classifier.cancel(); }
            });
        }
        if !self.classifier.status.is_empty() { ui.add(Label::new(&self.classifier.status).wrap()); }
    }
    fn classifier_reviews_ui(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label(RichText::new(format!("Suggested criteria · {} to review", self.classifier.settings.reviews.len())).strong());
        ui.add(Label::new("Existing choices are tried first. Below 75% confidence, nothing changes until you review. Edit a suggested label and its matching criteria, then accept it for this email and future classifications. Using an existing label name reuses its saved criteria.").wrap());
        let mut action = None;
        for (i, review) in self.classifier.settings.reviews.iter_mut().enumerate() {
            ui.push_id(i, |ui| { ui.group(|ui| {
                ui.add(Label::new(RichText::new(&review.subject).strong()).wrap());
                ui.label(format!("Best existing category: {} · confidence {:.0}%", review.decision.category, review.decision.confidence * 100.0));
                ComboBox::from_id_salt("review_category").selected_text(&review.decision.category).show_ui(ui, |ui| {
                    for c in classifier::CATEGORIES { ui.selectable_value(&mut review.decision.category, c.into(), c); }
                });
                for (j, criterion) in review.decision.suggestions.iter_mut().enumerate() {
                    ui.push_id(j, |ui| {
                        ui.label("Label name"); ui.add(TextEdit::singleline(&mut criterion.name).char_limit(48));
                        ui.label("Use this label when…"); ui.add(TextEdit::multiline(&mut criterion.description).desired_rows(2).char_limit(240));
                        let valid = classifier::valid_criterion(criterion) && classifier::CATEGORIES.contains(&review.decision.category.as_str());
                        if ui.add_enabled(valid, Button::new("Accept criteria & label email")).clicked() { action = Some((i, Some(j))); }
                        if !valid { ui.label("Enter a label name, matching criteria, and an existing category. Folder and category names are reserved."); }
                    });
                }
                if ui.button("Dismiss · keep current labels").clicked() { action = Some((i, None)); }
            }); });
        }
        if let Some((i, choice)) = action {
            if let Some(j) = choice { self.accept_classifier_criterion(i, j); }
            else { self.classifier.settings.reviews.remove(i); }
        }
        if !self.classifier.settings.criteria.is_empty() {
            ui.collapsing("Accepted custom criteria", |ui| {
                for c in &self.classifier.settings.criteria { ui.add(Label::new(format!("{} — {}", c.name, c.description)).wrap()); }
            });
            ComboBox::from_id_salt("custom_label_folder").selected_text("Browse custom labels").show_ui(ui, |ui| {
                for c in self.classifier.settings.criteria.clone() {
                    if ui.selectable_label(self.folder == c.name, &c.name).clicked() { self.category = "All".into(); self.go_to_folder(&c.name); self.settings = false; }
                }
            });
        }
    }
    fn accept_classifier_criterion(&mut self, i: usize, j: usize) -> bool {
        let Some(review) = self.classifier.settings.reviews.get(i).cloned() else { return false; };
        let Some(criterion) = review.decision.suggestions.get(j) else { return false; };
        if !classifier::valid_criterion(criterion) || !classifier::CATEGORIES.contains(&review.decision.category.as_str()) { return false; }
                let mut criterion = review.decision.suggestions[j].clone();
                criterion.name = criterion.name.trim().into(); criterion.description = criterion.description.trim().into();
                if let Some(mail) = self.mails.iter_mut().find(|m| classifier::identity(m) == review.key) {
                    if mail.category != review.original_category || mail.labels != review.original_labels || matches!(mail.folder.as_str(), "Trash" | "Sent" | "Drafts") {
                        self.toast = Toast::new("Email changed since classification. Dismiss this suggestion and classify again.");
                        return false;
                    }
                    let existing = self.classifier.settings.labels().into_iter().find(|l| l.eq_ignore_ascii_case(&criterion.name));
                    let name = existing.unwrap_or_else(|| {
                        self.classifier.settings.criteria.push(criterion.clone()); criterion.name.clone()
                    });
                    mail.category = review.decision.category;
                    if !mail.labels.contains(&name) { mail.labels.push(name); }
                    self.classifier.settings.learn(mail);
                    self.persist();
                    self.toast = Toast::new("Criteria accepted and email labeled. Future classifications will use this choice.");
                } else {
                    self.toast = Toast::new("This email no longer exists. Dismiss its suggestion."); return false;
                }
        true
    }
    fn poll_classifier(&mut self) {
        if let Some(result) = self.classifier.connection.as_ref().and_then(classifier::receive) {
            self.classifier.connection = None;
            self.classifier.status = result.unwrap_or_else(|error| error);
        }
        if let Some(result) = self.classifier.result.as_ref().and_then(classifier::receive) {
            self.classifier.result = None;
            let active = self.classifier.active.take().unwrap();
            if !self.classifier.cancelled {
                match result {
                    Ok(label) => {
                        self.classifier.done += 1;
                        let mut decision = classifier::pinterest_rule(&active)
                            .or_else(|| self.classifier.settings.corrected(&active))
                            .or(label).unwrap_or_else(|| "Uncertain".into());
                        if let Some(mail) = self.mails.iter_mut().find(|m| classifier::identity(m) == classifier::identity(&active)) {
                            // Respect manual corrections and folder moves made during inference.
                            if mail.category == active.category && mail.labels == active.labels && !matches!(mail.folder.as_str(), "Trash" | "Sent" | "Drafts") {
                                self.classifier.settings.reviews.retain(|r| r.key != classifier::identity(&active));
                                if decision.confidence >= 0.75 && decision.category != "Uncertain" {
                                    mail.category = decision.category; mail.labels = decision.labels; self.persist();
                                } else {
                                    self.classifier.uncertain += 1;
                                    if decision.suggestions.is_empty() {
                                        decision.suggestions.push(classifier::Criterion { name: "Needs review".into(), description: "Emails that do not clearly fit my existing label criteria; review these manually.".into() });
                                    }
                                    self.classifier.settings.reviews.push(classifier::Review {
                                        key: classifier::identity(&active), subject: active.subject.clone(),
                                        original_category: active.category.clone(), original_labels: active.labels.clone(), decision,
                                    });
                                    self.settings = true;
                                }
                                if let Some(store) = &self.store {
                                    if let Err(error) = store.save_classifier_settings(&self.classifier.settings) { self.toast = Toast::new(format!("Could not save classification review: {error}")); }
                                }
                            }
                        }
                        self.classifier.status = format!("Processed {} of {} · {} need review in AI settings", self.classifier.done, self.classifier.total, self.classifier.settings.reviews.len());
                        if self.classifier.queue.is_empty() { self.toast = Toast::new(self.classifier.status.clone()); }
                    }
                    Err(error) => {
                        self.classifier.queue.clear();
                        self.classifier.status = format!("Stopped after {} of {}. {error}", self.classifier.done, self.classifier.total);
                        self.toast = Toast::new(self.classifier.status.clone());
                        self.settings = true;
                    }
                }
            }
        }
        if self.classifier.active.is_none() {
            if let Some(mail) = self.classifier.queue.pop_front() {
                let settings = self.classifier.settings.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.classifier.active = Some(mail.clone());
                self.classifier.result = Some(rx);
                self.classifier.status = format!("Classifying {} of {}…", self.classifier.done + 1, self.classifier.total);
                std::thread::spawn(move || { let _ = tx.send(classifier::classify(&settings, &mail)); });
            }
        }
    }
    fn start_sync(&mut self) {
        if self.syncing.is_some() || self.sending.is_some() || self.google_signin.is_some() {
            return;
        }
        if self.accounts.is_empty() && self.screenshot.is_none() && !cfg!(test) {
            // Keychain may have been locked, or its access prompt dismissed, at
            // startup (macOS asks again after every rebuild of an unsigned app).
            match credentials::load() {
                Ok(saved) => {
                    self.credentials_error = None;
                    if !saved.is_empty() {
                        self.account = self.account.take().or_else(|| saved.first().cloned());
                        self.accounts = saved;
                    }
                }
                Err(message) => self.credentials_error = Some(message),
            }
        }
        if self.accounts.is_empty() {
            if let Some(message) = &self.credentials_error {
                self.toast = Toast::new(format!(
                    "Sparky couldn’t read your saved sign-ins. {message} If macOS asks for access, choose Always Allow, then sync again."
                ));
                return;
            }
            let downloaded = self.mails.iter().any(|mail| !mail.account.is_empty());
            self.toast = Toast::new(if downloaded {
                "Your previous sign-ins were not saved. Reconnect your accounts in Settings once to enable sync."
            } else {
                "No accounts are connected. Connect an account in Settings to sync."
            });
            self.settings = true;
            return;
        }
        if self.store.is_none() {
            self.toast = Toast::new("Sync unavailable: mailbox storage could not be opened.");
            return;
        }
        let accounts = self.accounts.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.syncing = Some(rx);
        self.toast = Toast::new(format!("Syncing {} connected accounts…", accounts.len()));
        std::thread::spawn(move || {
            let _ = tx.send(Ok(sync::fetch_all(&accounts, sync::Account::fetch)));
        });
    }
    fn cancel_google_signin(&mut self) {
        if let Some((_, cancel)) = self.google_signin.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    fn remove_sample_messages(&mut self) {
        // Downloaded mail also identifies returning users whose older sign-ins
        // were not saved. Never mix the demo inbox into a real mailbox.
        if self.accounts.is_empty() && !self.mails.iter().any(|mail| !mail.account.is_empty()) {
            return;
        }
        let keep: Vec<bool> = self.mails.iter().map(|mail| !mail.is_sample()).collect();
        if keep.iter().all(|keep| *keep) { return; }
        let remap = |index: usize| {
            keep.get(index).copied().unwrap_or(false)
                .then(|| keep[..index].iter().filter(|keep| **keep).count())
        };
        let selected = remap(self.selected);
        self.editing_draft = self.editing_draft.and_then(remap);
        self.mails.retain(|mail| !mail.is_sample());
        self.selected = selected.unwrap_or(0);
        if self.replying_to.as_ref().is_some_and(Mail::is_sample) {
            self.replying_to = None;
            self.inline_reply = false;
        }
        self.swipe = None;
        self.swipe_motion = Default::default();
        if selected.is_none() && !self.compose { self.select_received_mail(); }
        self.persist();
    }
    fn register_connected_account(&mut self, account: sync::Account) {
        self.accounts.retain(|a| a.scope() != account.scope());
        self.accounts.push(account.clone());
        self.account = Some(account);
        self.remove_sample_messages();
        self.welcome = false;
        if let Some(store) = &self.store {
            if let Err(error) = store.complete_onboarding() {
                self.toast = Toast::new(format!("Welcome preference could not be saved: {error}"));
            }
        }
        self.connect_secret.clear();
        self.connect_error.clear();
    }
    fn poll_network(&mut self) {
        let result = self
            .google_signin
            .as_ref()
            .and_then(|(rx, _)| match rx.try_recv() {
                Ok(value) => Some(value),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Google sign-in stopped. Please try again.".into()))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = result {
            self.google_signin = None;
            match result {
                Ok(account) => {
                    // OAuth and Gmail's profile endpoint have already verified access.
                    // Inbox download can take minutes and must not gate connection.
                    self.register_connected_account(account.clone());
                    self.toast = Toast::new(format!("{} connected · Downloading inbox…", account.address));
                    if let Err(error) = credentials::save(&self.accounts) {
                        self.connect_error = format!("Connected, but the account could not be remembered: {error}");
                    }
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.syncing = Some(rx);
                    std::thread::spawn(move || {
                        let _ = tx.send(account.fetch().map(sync::SyncBatch::from));
                    });
                }
                Err(error) => {
                    self.connect_error = error.clone();
                    self.toast = Toast::new(error);
                }
            }
        }

        if let Some(result) = self.syncing.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.syncing = None;
            match result {
                Ok(batch) => {
                    let needs_received_selection = !self.compose && self.mails.get(self.selected)
                        .is_none_or(Mail::is_sample);
                    let mails = batch.mails;
                    if let Some(account) = self.pending_account.take() {
                        self.register_connected_account(account);
                    }
                    let count = mails.len();
                    let mut to_classify = Vec::new();
                    for mut mail in mails.into_iter().rev() {
                        if let Some(existing) = self
                            .mails
                            .iter_mut()
                            .find(|m| m.account == mail.account && m.remote_id == mail.remote_id)
                        {
                            // Retain local moves, stars and read state; refresh server content and threading.
                            let (folder, star, unread, snoozed) = (
                                existing.folder.clone(),
                                existing.star,
                                existing.unread,
                                existing.snoozed_until,
                            );
                            let category = existing.category.clone();
                            let labels = existing.labels.clone();
                            *existing = mail;
                            existing.category = category;
                            existing.labels = labels;
                            existing.folder = folder;
                            existing.star = star;
                            existing.unread = unread;
                            existing.snoozed_until = snoozed;
                        } else {
                            // Obvious senders (LinkedIn, noreply@…) are sorted without the model.
                            let decided = classifier::apply_sender_rule(&self.classifier.settings, &mut mail);
                            if self.classifier.settings.automatic && !decided { to_classify.push(mail.clone()); }
                            self.mails.insert(0, mail);
                            self.selected += 1;
                            if let Some(index) = &mut self.editing_draft {
                                *index += 1;
                            }
                        }
                    }
                    self.classifier.enqueue(to_classify);
                    // Open received mail after connecting cleared the sample inbox.
                    if needs_received_selection {
                        self.select_received_mail();
                    }
                    self.toast = Toast::new(if batch.failures.is_empty() {
                        format!("Synced {count} recent inbox messages")
                    } else {
                        format!("Synced {count} recent inbox messages. Could not sync: {}", batch.failures.join("; "))
                    });
                    self.persist();
                    if let Err(error) = credentials::save(&self.accounts) {
                        self.toast = Toast::new(format!("Mail synced, but accounts could not be remembered: {error}"));
                    }
                }
                Err(error) => {
                    if self.pending_account.take().is_some() {
                        self.connect_error = error.clone();
                    }
                    self.toast = Toast::new(error);
                }
            }
        }
        if let Some(result) = self.sending.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.sending = None;
            match result {
                Ok(()) => {
                    self.store_composed("Sent");
                    self.toast = Toast::new("Message sent");
                }
                Err(error) => self.toast = Toast::new(error),
            }
        }
    }
    fn action(&mut self, a: &str) {
        if self.sending.is_some() || self.selected >= self.mails.len() {
            return;
        }
        match a {
            "Archive" => {
                self.mails[self.selected].folder = "Archive".into();
                self.toast = Toast::new("Moved to Archive");
            }
            "Snooze" => {
                self.mails[self.selected].folder = "Snoozed".into();
                self.mails[self.selected].snoozed_until = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        + 86400,
                );
                self.toast = Toast::new("Snoozed for 24 hours");
            }
            "Delete" => {
                self.mails[self.selected].folder = "Trash".into();
                self.toast = Toast::new("Moved to Trash");
            }
            "Move to Inbox" => {
                self.mails[self.selected].folder = "Inbox".into();
                self.mails[self.selected].snoozed_until = None;
                self.toast = Toast::new("Moved to Inbox");
            }
            "Star" => self.mails[self.selected].star = !self.mails[self.selected].star,
            "Unread" => {
                self.mails[self.selected].unread = !self.mails[self.selected].unread;
            }
            "Reply" => {
                self.replying_to = Some(self.mails[self.selected].clone());
                self.compose = true;
                self.inline_reply = true;
                self.writer_focus = true;
                self.editor.reset(Vec::new());
                if !self.mails[self.selected].account.is_empty() {
                    self.account = self
                        .accounts
                        .iter()
                        .find(|a| a.scope() == self.mails[self.selected].account)
                        .cloned();
                }
                self.editing_draft = None;
                self.cc.clear();
                self.bcc.clear();
                self.to = self.mails[self.selected].from.clone();
                self.subject = format!("Re: {}", self.mails[self.selected].subject);
                self.body = String::new();
            }
            _ => {}
        }
        self.persist();
    }
    fn open_selected(&mut self) {
        if !self.visible().contains(&self.selected) || self.sending.is_some() {
            return;
        }
        let m = self.mails[self.selected].clone();
        if m.folder == "Drafts" && self.sending.is_none() {
            self.compose = true;
            self.editing_draft = Some(self.selected);
            self.replying_to = Some(m.clone());
            self.inline_reply = false;
            self.writer_focus = true;
            self.editor.reset(m.marks.clone());
            self.account = self
                .accounts
                .iter()
                .find(|a| a.scope() == m.account)
                .cloned();
            self.cc = m.cc.clone();
            self.bcc = m.bcc.clone();
            self.show_bcc = !m.bcc.is_empty();
            self.draft_signature.clear();
            self.draft_dirty_since = None;
            self.draft_saved = true;
            self.to = m.from.clone();
            self.subject = m.subject.clone();
            self.body = m.body.clone();
        }
        self.mails[self.selected].unread = false;
        self.persist();
    }
    fn account_matches(&self, mail: &Mail) -> bool {
        self.account_filter.as_ref().is_none_or(|scope| scope == &mail.account)
    }
    fn select_received_mail(&mut self) {
        if let Some(index) = self.visible().into_iter().find(|&index| {
            let mail = &self.mails[index];
            !mail.account.is_empty() && !mail.remote_id.is_empty()
        }) {
            self.selected = index;
            self.scroll = 0.;
        }
    }
    fn account_filter_menu(&mut self, ui: &mut Ui) {
        let previous = self.account_filter.clone();
        let mut options: Vec<(String, String)> = self.accounts.iter()
            .map(|a| (a.scope(), a.address.clone())).collect();
        // Downloaded mail remains filterable after disconnecting or restarting.
        for mail in &self.mails {
            if !mail.account.is_empty() && !options.iter().any(|(scope, _)| scope == &mail.account) {
                options.push((mail.account.clone(), mail.account.split_once(':')
                    .map_or(mail.account.clone(), |(_, address)| address.to_owned())));
            }
        }
        let label = self.account_filter.as_ref().and_then(|scope| options.iter()
            .find(|(key, _)| key == scope).map(|(_, label)| label.as_str()))
            .unwrap_or("All accounts");
        ComboBox::from_id_salt("inbox-account-filter").width(195.)
            .selected_text(label).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.account_filter, None, "All accounts");
                for (scope, address) in options {
                    ui.selectable_value(&mut self.account_filter, Some(scope), address);
                }
            }).response.on_hover_text("Show mail from one account or all accounts");
        if previous != self.account_filter {
            if self.compose && self.inline_reply && self.sending.is_none() {
                self.store_composed("Drafts");
            }
            self.scroll = 0.;
            self.swipe = None;
            self.swipe_motion.clear();
            if let Some(first) = self.visible().first() { self.selected = *first; }
        }
    }
    fn dated_sections(&self, visible: &[usize]) -> [Vec<usize>; 3] {
        let today = date_groups::local_today();
        let mut sections: [Vec<usize>; 3] = Default::default();
        for &idx in visible {
            sections[date_groups::group(&self.mails[idx].time, today)].push(idx);
        }
        sections
    }
    /// Rows `(idx, y, height_factor)` and section headers `(group, y)` in list
    /// space, plus the total height. A leaving row shrinks toward zero height.
    fn list_layout(&self) -> (Vec<(usize, f32, f32)>, Vec<(usize, f32)>, f32) {
        let mut y = 0.;
        let mut rows = Vec::new();
        let mut heads = Vec::new();
        for (group, mails) in self.dated_sections(&self.visible()).iter().enumerate() {
            if mails.is_empty() { continue; }
            heads.push((group, y));
            y += SECTION_H;
            if !self.collapsed_dates[group] {
                for &idx in mails {
                    let k = self.leaving.as_ref().filter(|l| l.idx == idx).map_or(1., |l| 1. - l.collapse(self.now));
                    rows.push((idx, y, k));
                    y += ROW_STRIDE * k;
                }
            }
        }
        (rows, heads, y)
    }
    fn expanded_rows(&self) -> Vec<(usize, f32)> {
        self.list_layout().0.into_iter().map(|(idx, y, _)| (idx, y)).collect()
    }
    /// Archive/Delete/Snooze/Move to Inbox. When the mail would leave the
    /// current list, the row animates out first and the move lands afterwards.
    fn dismiss(&mut self, action: &'static str, from: f32) {
        self.finish_leaving();
        if self.sending.is_some() || self.selected >= self.mails.len() {
            return;
        }
        let target = match action { "Archive" => "Archive", "Delete" => "Trash", "Snooze" => "Snoozed", _ => "Inbox" };
        let leaves = self.folder != "Starred"
            && self.folder != target
            && self.screenshot.is_none()
            && self.expanded_rows().iter().any(|(i, _)| *i == self.selected);
        if leaves {
            self.leaving = Some(Leaving {
                idx: self.selected,
                message_id: self.mails[self.selected].message_id.clone(),
                action,
                started: self.now,
                from,
            });
        } else {
            self.action(action);
        }
        self.swipe = None;
    }
    /// Applies a pending row exit and selects the row that slid into its place.
    fn finish_leaving(&mut self) {
        let Some(l) = self.leaving.take() else { return };
        // Mail may have been re-synced meanwhile; find the message again.
        let idx = if self.mails.get(l.idx).is_some_and(|m| m.message_id == l.message_id) {
            l.idx
        } else if let Some(i) = self.mails.iter().position(|m| m.message_id == l.message_id) {
            i
        } else {
            return;
        };
        let before = self.expanded_rows();
        let position = before.iter().position(|(i, _)| *i == idx);
        let previous = self.selected;
        self.selected = idx;
        self.action(l.action);
        self.swipe_motion.remove(&idx);
        let after = self.expanded_rows();
        if after.iter().any(|(i, _)| *i == idx) {
            return;
        }
        self.selected = if previous != idx && after.iter().any(|(i, _)| *i == previous) {
            previous
        } else {
            position.and_then(|p| after.get(p).or(after.last())).map_or(self.selected, |(i, _)| *i)
        };
    }
    fn visible(&self) -> Vec<usize> {
        let hits = if self.search.trim().is_empty() {
            None
        } else {
            Some(match &self.store {
                Some(store) => store.search(&self.search).unwrap_or_default(),
                None => self
                    .mails
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| {
                        format!("{} {} {}", m.from, m.subject, m.body)
                            .to_lowercase()
                            .contains(&self.search.to_lowercase())
                    })
                    .map(|(i, _)| i)
                    .collect(),
            })
        };
        let groups = threading::threads(&self.mails);
        let mut seen = std::collections::HashSet::new();
        self.mails
            .iter()
            .enumerate()
            .filter(|(i, m)| {
                (if self.classifier.settings.labels().contains(&self.folder) {
                    m.labels.contains(&self.folder) && !matches!(m.folder.as_str(), "Trash" | "Drafts" | "Sent")
                } else if self.folder == "Starred" {
                    m.star
                } else {
                    m.folder == self.folder
                }) && (self.category == "All" || m.category == self.category)
                    && self.account_matches(m)
                    && hits.as_ref().is_none_or(|hits| hits.contains(i))
                    && seen.insert(groups[*i])
            })
            .map(|(i, _)| i)
            .collect()
    }
}
impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &Context, raw_input: &mut RawInput) {
        self.record_input(ctx, raw_input);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Never drop a move whose exit animation was still running at quit.
        self.finish_leaving();
    }
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
        self.frames += 1;
        if !self.settings {
            self.cancel_google_signin();
        }
        self.poll_network();
        self.remove_sample_messages();
        self.poll_classifier();
        if !self.accounts.is_empty() {
            self.welcome = false;
        }
        if self.syncing.is_some() || self.sending.is_some() || self.google_signin.is_some() || self.classifier.busy() || self.classifier.connection.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.frames % 120 == 1 {
            self.wake_snoozed();
        }
        if self.screenshot.is_some() {
            ctx.request_repaint();
        }
        if self.frames == 120 && self.screenshot.is_some() {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
        }
        for ev in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, .. } = ev {
                if let Some(path) = &self.screenshot {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    image::save_buffer(
                        path,
                        &bytes,
                        image.size[0] as u32,
                        image.size[1] as u32,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                    self.saved_shot = true;
                }
            }
        }
        if self.saved_shot {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        self.record_output(ctx);
        self.now = ctx.input(|i| i.time);
        self.handle_shortcuts(ctx);
        CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::WHITE))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                let w = full.width();
                let h = full.height();
                let sx = 212.;
                // The list takes about 30% of the window; the reader gets the rest.
                let lx = sx + (w * 0.3).clamp(398., 460.);
                let p = ui.painter().clone();
                p.rect_filled(Rect::from_min_max(pos2(0., 0.), pos2(sx, h)), 0., SIDEBAR_BG);
                line(&p, pos2(sx, 0.), pos2(sx, h));
                line(&p, pos2(lx, 66.), pos2(lx, h - 34.));
                line(&p, pos2(sx, 66.), pos2(w, 66.));
                p.rect_filled(
                    Rect::from_min_size(pos2(24., 26.), vec2(29., 29.)),
                    CORNER_RADIUS,
                    self.theme.color(),
                );
                p.text(
                    pos2(38., 40.),
                    Align2::CENTER_CENTER,
                    "S",
                    FontId::proportional(23.),
                    Color32::WHITE,
                );
                txt(&p, pos2(64., 28.), "Sparky", 24., INK);
                // Use the connected account list, even when a local draft has
                // temporarily cleared the writer's From account.
                let identity = self.account_filter.as_ref()
                    .and_then(|scope| self.accounts.iter().find(|a| a.scope() == *scope))
                    .or_else(|| self.accounts.first());
                let display_name = identity.map(|a| a.display_name()).unwrap_or("Local mailbox");
                let initials = identity
                    .map(|a| a.initials())
                    .unwrap_or_else(|| "S".into());
                let status = match self.accounts.len() {
                    0 => "Connect email account".to_owned(),
                    1 => "1 account connected".to_owned(),
                    count => format!("{count} accounts connected"),
                };
                txt(&p, pos2(24., 76.), if identity.is_some() { "YOUR ACCOUNTS" } else { "LOCAL MAILBOX" }, 10., MUTED);
                avatar(&p, pos2(38., 113.), &initials, [225, 219, 244], 16.);
                mail_list_line(&p, Rect::from_min_size(pos2(63., 98.), vec2(135., 18.)), display_name, 13., INK);
                mail_list_line(&p, Rect::from_min_size(pos2(63., 118.), vec2(135., 16.)), &status, 11., MUTED);
                let account_hint = if self.accounts.is_empty() {
                    "Connect your email account".to_owned()
                } else {
                    format!("{}\nClick to manage accounts", self.accounts.iter().map(|a| a.address.as_str()).collect::<Vec<_>>().join("\n"))
                };
                if ui.allocate_rect(Rect::from_min_size(pos2(20., 94.), vec2(180., 44.)), Sense::click())
                    .on_hover_text(account_hint)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked() {
                    self.settings = true;
                }
                if button_icon(
                    ui,
                    Rect::from_min_size(pos2(20., 148.), vec2(172., 36.)),
                    Some(icons::Icon::Compose),
                    "Compose",
                    self.theme.color(),
                    Color32::WHITE,
                )
                .clicked()
                {
                    self.begin_compose();
                }
                let nav = [
                    "Inbox", "Starred", "Snoozed", "Sent", "Drafts", "Archive", "Trash",
                ];
                for (n, name) in nav.iter().enumerate() {
                    let y = 200. + n as f32 * 34.;
                    let rect = Rect::from_min_size(pos2(12., y), vec2(188., 32.));
                    let r = ui.allocate_rect(rect, Sense::click());
                    let active = self.folder == *name;
                    if active || r.hovered() {
                        p.rect_filled(
                            rect,
                            CORNER_RADIUS,
                            if active {
                                self.theme.tint(0.13)
                            } else {
                                Color32::from_rgb(239, 239, 245)
                            },
                        );
                    }
                    let ic = if active { self.theme.color() } else { MUTED };
                    let center = pos2(32., rect.center().y);
                    if let Some(icon) = icons::Icon::for_name(name) {
                        icons::paint(&p, ctx, icon, Rect::from_center_size(center, vec2(17., 17.)), ic);
                    }
                    p.text(
                        pos2(52., center.y),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(13.),
                        if active { self.theme.color() } else { INK },
                    );
                    let count = self
                        .mails
                        .iter()
                        .filter(|m| m.folder == *name && m.unread && self.account_matches(m))
                        .count();
                    if count > 0 {
                        p.text(
                            pos2(180., center.y),
                            Align2::RIGHT_CENTER,
                            count.to_string(),
                            FontId::proportional(12.),
                            self.theme.color(),
                        );
                    }
                    if r.clicked() {
                        if self.compose && self.inline_reply && self.sending.is_none() {
                            self.store_composed("Drafts");
                        }
                        self.go_to_folder(name);
                    }
                }
                if button_icon(
                    ui,
                    Rect::from_min_size(pos2(19., h - 127.), vec2(175., 30.)),
                    Some(icons::Icon::UserPlus),
                    "Connect email account",
                    Color32::TRANSPARENT,
                    self.theme.color(),
                )
                .clicked()
                {
                    self.settings = true;
                }
                txt(&p, pos2(24., 456.), "LABELS", 10., MUTED);
                let label_colors = [
                    [157, 145, 211],
                    [124, 178, 156],
                    [218, 171, 111],
                    [112, 166, 218],
                    [194, 138, 181],
                    [100, 173, 169],
                ];
                let labels = self.classifier.settings.labels();
                let labels_height = (h - 615.).max(0.);
                ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(pos2(19., 475.), vec2(181., labels_height))), |ui| {
                    ScrollArea::vertical()
                        .id_salt("sidebar-labels")
                        .max_height(labels_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.;
                            for (i, name) in labels.iter().enumerate() {
                                let (row, _) = ui.allocate_exact_size(vec2(175., 28.), Sense::hover());
                                let label_rect = Rect::from_min_size(row.min, vec2(175., 26.));
                                let response = ui.interact(label_rect, Id::new(("sidebar-label", name)), Sense::click());
                                let painter = ui.painter();
                                if self.folder == *name || response.hovered() {
                                    painter.rect_filled(label_rect, CORNER_RADIUS, self.theme.tint(0.10));
                                }
                                if response.clicked() { self.category = "All".into(); self.go_to_folder(name); }
                                response.on_hover_text(format!("Show emails labeled {name}"));
                                let col = label_colors[i % label_colors.len()];
                                let y = label_rect.center().y;
                                painter.circle_stroke(
                                    pos2(row.min.x + 13., y),
                                    4.,
                                    Stroke::new(1.6, Color32::from_rgb(col[0], col[1], col[2])),
                                );
                                painter.text(
                                    pos2(row.min.x + 33., y),
                                    Align2::LEFT_CENTER,
                                    name,
                                    FontId::proportional(12.),
                                    MUTED,
                                );
                            }
                        });
                });
                if button_icon(
                    ui,
                    Rect::from_min_size(pos2(19., h - 88.), vec2(175., 30.)),
                    Some(icons::Icon::Settings),
                    "Settings & shortcuts",
                    Color32::TRANSPARENT,
                    MUTED,
                )
                .clicked()
                {
                    self.settings = true;
                }
                txt(
                    &p,
                    pos2(26., h - 39.),
                    if self.account.is_some() {
                        "•  Account configured"
                    } else {
                        "•  Local mailbox"
                    },
                    11.,
                    MUTED,
                );
                p.text(pos2(sx + 26., 36.), Align2::LEFT_CENTER, &self.folder, FontId::new(T_TITLE, FontFamily::Name("ui-medium".into())), INK);
                // Search trigger: opens the search modal. An active list filter shows
                // here as the query with a clear button.
                let search_rect = Rect::from_min_size(pos2(w - 344., 18.), vec2(320., 34.));
                let filtering = !self.search.is_empty();
                let clear_rect = Rect::from_center_size(search_rect.right_center() - vec2(18., 0.), vec2(22., 22.));
                let clear = filtering.then(|| ui.interact(clear_rect, Id::new("clear-mail-filter"), Sense::click())
                    .on_hover_text("Clear filter").on_hover_cursor(CursorIcon::PointingHand));
                let trigger = ui.interact(search_rect, Id::new("mail-search-trigger"), Sense::click())
                    .on_hover_text(format!("Search mail · {}", shortcuts::hint("Search mail")))
                    .on_hover_cursor(CursorIcon::PointingHand);
                let hover_t = icons::fade(ctx, Id::new("mail-search-hover"), trigger.hovered(), icons::DUR_FAST);
                p.rect_filled(search_rect, 8., Color32::from_rgb(244, 245, 248).lerp_to_gamma(Color32::from_rgb(237, 238, 243), hover_t));
                p.rect_stroke(search_rect, 8., Stroke::new(1., if filtering { self.theme.color() } else { LINE }), StrokeKind::Inside);
                icons::paint(&p, ctx, icons::Icon::Search, Rect::from_center_size(search_rect.left_center() + vec2(18., 0.), vec2(15., 15.)), if filtering { self.theme.color() } else { MUTED });
                let label_rect = Rect::from_min_max(search_rect.min + vec2(34., 0.), search_rect.max - vec2(40., 0.));
                let label = if filtering { format!("“{}”", self.search) } else { "Search mail".to_owned() };
                let mut job = text::LayoutJob::simple_singleline(label, FontId::proportional(T_BODY), if filtering { INK } else { MUTED });
                job.wrap.max_width = label_rect.width();
                job.wrap.max_rows = 1;
                let g = p.layout_job(job);
                p.with_clip_rect(label_rect).galley(pos2(label_rect.left(), search_rect.center().y - g.size().y / 2.), g, MUTED);
                if let Some(clear) = &clear {
                    icons::paint(&p, ctx, icons::Icon::X, clear_rect.shrink(5.), if clear.hovered() { INK } else { MUTED });
                } else {
                    paint_keycaps(&p, search_rect.right_center() - vec2(8., 0.), &shortcuts::hint("Search mail"), true);
                }
                if clear.is_some_and(|c| c.clicked()) {
                    self.search.clear();
                    self.scroll = 0.;
                } else if trigger.clicked() {
                    self.open_search();
                }
                ui.scope_builder(UiBuilder::new().max_rect(
                    Rect::from_min_size(pos2(sx + 26., 76.), vec2(200., 28.))),
                    |ui| self.account_filter_menu(ui));
                ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(pos2(sx + 236., 76.), vec2(155., 30.))), |ui| {
                    if self.classifier.busy() {
                        if ui.button(format!("{} / {} · Cancel", self.classifier.done, self.classifier.total)).clicked() { self.classifier.cancel(); }
                    } else if ui.button("Auto-classify inbox").on_hover_text("Label existing inbox mail for the selected account with your selected AI model. Learn how in Settings → AI model.").clicked() {
                        let mails = self.mails.iter().filter(|m| m.folder == "Inbox" && self.account_matches(m)).cloned().collect();
                        self.classifier.enqueue(mails);
                    }
                });
                let tabs = ["All", "People", "Newsletters", "Notifications"];
                for (i, t) in tabs.iter().enumerate() {
                    let x = sx + 23. + [0., 52., 132., 247.][i];
                    let tw = [43., 70., 105., 112.][i];
                    let r = button(
                        ui,
                        Rect::from_min_size(pos2(x, 116.), vec2(tw, 29.)),
                        t,
                        if self.category == *t {
                            self.theme.tint(0.10)
                        } else {
                            Color32::WHITE
                        },
                        if self.category == *t { self.theme.color() } else { MUTED },
                    );
                    if r.clicked() {
                        self.category = t.to_string();
                        self.scroll = 0.;
                    }
                }
                line(&p, pos2(sx, 158.), pos2(lx, 158.));

                let visible = self.visible();
                if !visible.contains(&self.selected) {
                    if let Some(&first) = visible.first() {
                        self.selected = first;
                    }
                }
                let list_clip = Rect::from_min_max(pos2(sx, 168.), pos2(lx, h - 35.));
                if self.swipe.is_some_and(|(idx, _)| !visible.contains(&idx)) {
                    self.swipe = None;
                }
                let now = ctx.input(|i| i.time);
                if self.leaving.as_ref().is_some_and(|l| l.done(now)) {
                    self.finish_leaving();
                } else if self.leaving.is_some() {
                    ctx.request_repaint();
                }
                let wheel = ctx.input(|i| i.raw_scroll_delta);
                let horizontal_swipe = wheel.x.abs() > wheel.y.abs() && wheel.x.abs() > 0.;
                if self.swipe.is_some() && ctx.input(|i| i.key_pressed(Key::Escape)) {
                    self.swipe = None;
                }
                if let Some((idx, offset)) = self.swipe {
                    if now - self.swipe_last_input > 0.15 && !ctx.input(|i| i.pointer.any_down()) {
                        self.swipe = if offset < -40. { Some((idx, -SWIPE_OPEN)) } else { None };
                    }
                }
                let (rows, heads, content_height) = self.list_layout();
                let max_scroll = (content_height - list_clip.height()).max(0.);
                if ui.rect_contains_pointer(list_clip) && !horizontal_swipe {
                    self.scroll -= ctx.input(|i| i.smooth_scroll_delta.y);
                }
                self.scroll = self.scroll.clamp(0., max_scroll);
                let groups = threading::threads(&self.mails);
                let mut thread_counts = std::collections::HashMap::new();
                for group in &groups {
                    *thread_counts.entry(*group).or_insert(0usize) += 1;
                }
                let list_painter = p.with_clip_rect(list_clip);
                let mut swipe_action = None;
                let mut pointer_in_swipe = false;
                self.swipe_motion.retain(|idx, _| visible.contains(idx));
                let medium = |size: f32| FontId::new(size, FontFamily::Name("ui-medium".into()));
                // The selection bar glides between rows instead of jumping.
                let selected_y = rows.iter().find(|(i, _, _)| *i == self.selected).map(|(_, y, _)| *y);
                let bar_y = selected_y.map(|y| ctx.animate_value_with_time(Id::new("selection-bar"), y, 0.16));
                for (n, &(idx, row_y, height)) in rows.iter().enumerate() {
                    let y = list_clip.top() + row_y - self.scroll;
                    if y > h - 35. || y + ROW_H < list_clip.top() || height <= 0.01 {
                        continue;
                    }
                    let m = self.mails[idx].clone();
                    let rect = Rect::from_min_size(pos2(sx + 8., y), vec2(lx - sx - 16., ROW_H));
                    // A collapsing row is clipped to its shrinking slot and fades out.
                    let slot = Rect::from_min_size(rect.min, vec2(rect.width(), ROW_STRIDE * height)).intersect(list_clip);
                    let leaving = self.leaving.as_ref().filter(|l| l.idx == idx).map(|l| (l.slide(now), l.from));
                    let motion = self.swipe_motion.entry(idx).or_default();
                    if self.swipe.is_some_and(|(i, _)| i == idx) || motion.offset < 0. {
                        pointer_in_swipe |= ctx.input(|i| i.pointer.interact_pos().is_some_and(|p| rect.contains(p)));
                    }
                    // Past the open width the row resists, like a rubber band.
                    let rubber = |raw: f32| if raw < -SWIPE_OPEN { -SWIPE_OPEN + (raw + SWIPE_OPEN) * 0.25 } else { raw };
                    let raw_offset = self.swipe.filter(|(i, _)| *i == idx).map_or(motion.offset, |(_, x)| x);
                    if leaving.is_none() && ui.rect_contains_pointer(slot) && horizontal_swipe {
                        let raw = (raw_offset + wheel.x).clamp(-SWIPE_OPEN - 120., 0.);
                        self.swipe = Some((idx, raw));
                        motion.follow(rubber(raw));
                        self.swipe_last_input = now;
                        ctx.request_repaint_after(std::time::Duration::from_millis(170));
                    }
                    let content_rect = rect.translate(vec2(motion.offset, 0.));
                    let r = ui.interact(content_rect.intersect(slot), Id::new(("swipe-mail", idx)), if leaving.is_some() { Sense::hover() } else { Sense::click_and_drag() });
                    if r.dragged() {
                        let delta = r.drag_delta();
                        if delta.x.abs() > delta.y.abs() || motion.offset < 0. {
                            let raw = (raw_offset + delta.x).clamp(-SWIPE_OPEN - 120., 0.);
                            self.swipe = Some((idx, raw));
                            motion.follow(rubber(raw));
                            self.swipe_last_input = now;
                        }
                    }
                    if r.drag_stopped() {
                        self.swipe = self.swipe.filter(|(i, x)| *i == idx && *x < -40.).map(|(i, _)| (i, -SWIPE_OPEN));
                    }
                    let target = self.swipe.filter(|(i, _)| *i == idx).map_or(0., |(_, x)| rubber(x));
                    if !r.dragged() && motion.settle(target, ctx.input(|i| i.stable_dt.min(0.1))) {
                        ctx.request_repaint();
                    }
                    let offset = match leaving {
                        Some((t, from)) => icons::lerp(from, -rect.width() - 8., t),
                        None => motion.offset,
                    };
                    let content_rect = rect.translate(vec2(offset, 0.));
                    let fade = height.powf(1.5);
                    if offset < 0. {
                        // Archive and Delete grow out from behind the row as it slides.
                        let reveal = Rect::from_min_max(pos2(content_rect.right() - CORNER_RADIUS as f32, y), rect.max);
                        let revealed = (-offset).max(0.);
                        let progress = (revealed / SWIPE_OPEN).min(1.);
                        let tile_w = reveal.width() / 2.;
                        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                            ui.set_clip_rect(slot);
                            ui.set_opacity(fade);
                            for (n, label, icon, color) in [
                                (0, "Archive", icons::Icon::Archive, SWIPE_ARCHIVE),
                                (1, "Delete", icons::Icon::Trash, SWIPE_DELETE),
                            ] {
                                let tile = Rect::from_min_size(pos2(reveal.left() + n as f32 * tile_w, y), vec2(tile_w, ROW_H));
                                let hit = tile.intersect(slot);
                                let response = (hit.is_positive() && leaving.is_none()).then(|| {
                                    ui.interact(hit, Id::new(("swipe-action", idx, n)), Sense::click())
                                        .on_hover_cursor(CursorIcon::PointingHand)
                                        .on_hover_text(format!("{label} · {}", shortcuts::hint(label)))
                                });
                                let hovered = icons::fade(ui.ctx(), Id::new(("swipe-hover", idx, n)), response.as_ref().is_some_and(|r| r.hovered()), icons::DUR_FAST);
                                let pressed = response.as_ref().is_some_and(|r| r.is_pointer_button_down_on());
                                let corner = if n == 1 { CornerRadius { ne: CORNER_RADIUS, se: CORNER_RADIUS, ..Default::default() } } else { CornerRadius::ZERO };
                                let fill = color.lerp_to_gamma(Color32::BLACK, 0.08 * hovered + if pressed { 0.08 } else { 0. });
                                ui.painter().rect_filled(tile, corner, fill);
                                let label_t = ((tile_w - 56.) / 24.).clamp(0., 1.);
                                let scale = icons::lerp(0.7, 1., progress) * if pressed { 0.94 } else { 1. };
                                let icon_center = tile.center() - vec2(0., 9. * label_t);
                                icons::paint(ui.painter(), ui.ctx(), icon, Rect::from_center_size(icon_center, vec2(18., 18.) * scale), Color32::WHITE.gamma_multiply(progress));
                                if label_t > 0. {
                                    ui.painter().text(tile.center() + vec2(0., 14.), Align2::CENTER_CENTER, label, medium(T_CAPTION), Color32::WHITE.gamma_multiply(label_t));
                                }
                                if response.is_some_and(|r| r.clicked()) {
                                    swipe_action = Some((idx, label, offset));
                                }
                            }
                        });
                    }
                    let row_painter = list_painter.with_clip_rect(slot);
                    let p = &row_painter;
                    if offset < 0. {
                        // The row lifts off the actions with a soft edge shadow.
                        let lift = ((-offset) / 40.).min(1.) * fade;
                        p.add(Shadow { offset: [2, 0], blur: 14, spread: 0, color: Color32::from_black_alpha((34. * lift) as u8) }
                            .as_shape(content_rect, CORNER_RADIUS));
                    }
                    let hover_t = icons::fade(ctx, Id::new(("row-hover", idx)), r.hovered() && offset == 0., icons::DUR_FAST);
                    let base = if idx == self.selected { self.theme.tint(0.07) } else { Color32::WHITE.lerp_to_gamma(Color32::from_rgb(246, 246, 249), hover_t) };
                    p.rect_filled(content_rect, CORNER_RADIUS, base);
                    let ink = INK.gamma_multiply(fade);
                    let muted = MUTED.gamma_multiply(fade);
                    let left = content_rect.left();
                    let right = content_rect.right() - 12.;
                    if m.unread {
                        p.circle_filled(pos2(left + 10., y + 30.), 3.5, self.theme.color().gamma_multiply(fade));
                    }
                    avatar(p, pos2(left + 36., y + 30.), &m.initials, m.color, 16.);
                    let text_x = left + 62.;
                    let time_galley = p.layout_no_wrap(m.time.clone(), FontId::proportional(T_CAPTION), muted);
                    let time_w = time_galley.size().x.min(150.);
                    p.with_clip_rect(slot.intersect(Rect::from_min_max(pos2(right - time_w, y), pos2(right, y + ROW_H))))
                        .galley(pos2(right - time_w, y + 15.), time_galley, muted);
                    let sender_font = if m.unread { medium(T_BODY) } else { FontId::proportional(T_BODY) };
                    mail_list_line_font(p, Rect::from_min_max(pos2(text_x, y + 12.), pos2(right - time_w - 10., y + 32.)), sender_name(&m.from), sender_font, ink);
                    let count = thread_counts[&groups[idx]];
                    let subject_right = if count > 1 {
                        let badge = p.layout_no_wrap(count.to_string(), medium(10.5), muted);
                        let badge_rect = Rect::from_min_size(pos2(right - badge.size().x - 12., y + 36.), badge.size() + vec2(12., 4.));
                        p.rect_filled(badge_rect, 9., Color32::from_rgb(240, 241, 245));
                        p.galley(badge_rect.center() - badge.size() / 2., badge, muted);
                        badge_rect.left() - 8.
                    } else {
                        right
                    };
                    let subject_font = if m.unread { medium(T_SMALL + 0.5) } else { FontId::proportional(T_SMALL + 0.5) };
                    mail_list_line_font(p, Rect::from_min_max(pos2(text_x, y + 36.), pos2(subject_right, y + 56.)), &m.subject, subject_font, ink);
                    mail_list_line_font(p, Rect::from_min_max(pos2(text_x, y + 59.), pos2(right - 22., y + 77.)), &m.preview, FontId::proportional(T_SMALL), muted);
                    if m.star {
                        icons::paint(p, ctx, icons::Icon::StarFilled, Rect::from_center_size(pos2(right - 7., y + 68.), vec2(14., 14.)), self.theme.color().gamma_multiply(fade));
                    }
                    if idx == self.selected {
                        if let Some(bar_y) = bar_y {
                            let bar_top = list_clip.top() + bar_y - self.scroll + 16.;
                            p.rect_filled(Rect::from_min_size(pos2(sx + 8. + offset.min(0.), bar_top), vec2(3., ROW_H - 32.)), 2., self.theme.color().gamma_multiply(fade));
                        }
                    }
                    // Hairline separator, inset to the text column, between rows of
                    // a section. It stays put while the row swipes.
                    let next_in_section = rows.get(n + 1).is_some_and(|&(_, next_y, _)| (next_y - (row_y + ROW_STRIDE * height)).abs() < 0.5);
                    let highlighted = |i: usize| i == self.selected;
                    if next_in_section && !highlighted(idx) && !rows.get(n + 1).is_some_and(|&(next, _, _)| highlighted(next)) {
                        let sep_y = y + ROW_H + (ROW_STRIDE - ROW_H) / 2.;
                        list_painter.with_clip_rect(slot.expand2(vec2(0., 2.)).intersect(list_clip)).hline(
                            (rect.left() + 62.)..=(rect.right() - 8.),
                            sep_y,
                            Stroke::new(1., Color32::from_rgb(236, 237, 242).gamma_multiply(fade)),
                        );
                    }
                    if r.clicked() && offset == 0. && leaving.is_none() {
                        self.swipe = None;
                        if self.compose && self.inline_reply && self.sending.is_none() {
                            self.store_composed("Drafts");
                        }
                        self.selected = idx;
                        self.open_selected();
                        self.persist();
                    }
                }
                // Date headers paint over the rows and stay pinned while their section scrolls.
                let mut toggle = None;
                for (n, &(group, head_y)) in heads.iter().enumerate() {
                    let natural = list_clip.top() + head_y - self.scroll;
                    let next = heads.get(n + 1).map_or(f32::INFINITY, |(_, y)| list_clip.top() + y - self.scroll);
                    let pinned = natural < list_clip.top();
                    let top = if pinned { list_clip.top().min(next - SECTION_H) } else { natural };
                    let rect = Rect::from_min_size(pos2(sx + 1., top), vec2(lx - sx - 2., SECTION_H - 2.));
                    let hit = rect.intersect(list_clip);
                    if !hit.is_positive() { continue; }
                    let collapsed = self.collapsed_dates[group];
                    let name = ["Today", "Yesterday", "Earlier"][group];
                    let response = ui.interact(hit, Id::new(("date-section", group)), Sense::click())
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(if collapsed { "Show emails" } else { "Hide emails" });
                    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true,
                        format!("{} {} emails", if collapsed { "Expand" } else { "Collapse" }, name)));
                    let hp = p.with_clip_rect(hit);
                    hp.rect_filled(rect, 0., Color32::WHITE);
                    let hover_t = icons::fade(ctx, Id::new(("section-hover", group)), response.hovered(), icons::DUR_FAST);
                    hp.rect_filled(rect.shrink2(vec2(7., 2.)), 5., self.theme.tint(0.06).gamma_multiply(hover_t));
                    if pinned {
                        line(&hp, pos2(rect.left(), rect.bottom()), pos2(rect.right(), rect.bottom()));
                    }
                    let section_count = rows.iter().filter(|(i, _, _)| date_groups::group(&self.mails[*i].time, date_groups::local_today()) == group).count();
                    let count = if collapsed { self.dated_sections(&visible)[group].len() } else { section_count };
                    icons::paint(&hp, ctx, if collapsed { icons::Icon::ChevronRight } else { icons::Icon::ChevronDown },
                        Rect::from_center_size(pos2(sx + 24., rect.center().y), vec2(14., 14.)), MUTED);
                    hp.text(pos2(sx + 38., rect.center().y), Align2::LEFT_CENTER, name.to_uppercase(), medium(10.5), MUTED);
                    let name_w = hp.layout_no_wrap(name.to_uppercase(), medium(10.5), MUTED).size().x;
                    hp.text(pos2(sx + 46. + name_w, rect.center().y), Align2::LEFT_CENTER, count.to_string(), FontId::proportional(10.5), MUTED);
                    if response.clicked() { toggle = Some(group); }
                }
                if let Some(group) = toggle {
                    self.collapsed_dates[group] = !self.collapsed_dates[group];
                    self.swipe = None;
                    self.swipe_motion.clear();
                    ctx.request_repaint();
                }
                if ctx.input(|i| i.pointer.any_pressed()) && !pointer_in_swipe {
                    self.swipe = None;
                    ctx.request_repaint();
                }
                if let Some((idx, action, offset)) = swipe_action {
                    self.selected = idx;
                    self.dismiss(action, offset);
                }
                if visible.is_empty() {
                    let center = pos2((sx + lx) / 2., 300.);
                    p.circle_filled(center, 28., self.theme.tint(0.08));
                    icons::paint(&p, ctx, if self.search.is_empty() { icons::Icon::Inbox } else { icons::Icon::Search },
                        Rect::from_center_size(center, vec2(24., 24.)), self.theme.color());
                    p.text(center + vec2(0., 50.), Align2::CENTER_CENTER,
                        if self.search.is_empty() { "Nothing here" } else { "No matching mail" },
                        FontId::new(T_BODY + 1., FontFamily::Name("ui-medium".into())), INK);
                    p.text(center + vec2(0., 74.), Align2::CENTER_CENTER,
                        if self.search.is_empty() { "A little breathing room." } else { "Try fewer or different words." },
                        FontId::proportional(T_SMALL), MUTED);
                }
                if !visible.is_empty() {
                    let m = self.mails[self.selected].clone();
                    let x = lx + 48.;
                    let right = w - 48.;
                    // Toolbar: one evenly spaced group of ghost buttons.
                    let mut bx = lx + 24.;
                    let toolbar: [(icons::Icon, &str, &'static str); 4] = [
                        (icons::Icon::Archive, "Archive", "Archive"),
                        (icons::Icon::Clock, "Snooze", "Snooze"),
                        (if m.star { icons::Icon::StarFilled } else { icons::Icon::Star }, "", "Star"),
                        (icons::Icon::Trash, "Delete", "Delete"),
                    ];
                    for (icon, label, action) in toolbar {
                        let bw = if label.is_empty() { 34. } else { 40. + p.layout_no_wrap(label.into(), FontId::new(13., FontFamily::Name("ui-medium".into())), MUTED).size().x };
                        let tint = if action == "Star" && m.star { self.theme.color() } else { MUTED };
                        let tip = if action == "Star" && m.star { "Unstar".to_owned() } else { action.to_owned() };
                        if button_icon(ui, Rect::from_min_size(pos2(bx, 82.), vec2(bw, 32.)), Some(icon), label, Color32::TRANSPARENT, tint)
                            .on_hover_text(format!("{tip} · {}", shortcuts::hint(action)))
                            .clicked()
                        {
                            match action {
                                "Star" => self.action("Star"),
                                _ => self.dismiss(action, 0.),
                            }
                        }
                        bx += bw + 4.;
                    }
                    if self.folder != "Inbox" {
                        p.line_segment([pos2(bx + 4., 90.), pos2(bx + 4., 106.)], Stroke::new(1., LINE));
                        if button_icon(ui, Rect::from_min_size(pos2(bx + 12., 82.), vec2(134., 32.)), Some(icons::Icon::ArchiveRestore), "Move to Inbox", Color32::TRANSPARENT, MUTED)
                            .on_hover_text(format!("Move to Inbox · {}", shortcuts::hint("Move to Inbox")))
                            .clicked()
                        {
                            self.dismiss("Move to Inbox", 0.);
                        }
                    }
                    let unread_label = if m.unread { "Mark read" } else { "Mark unread" };
                    if button_icon(ui, Rect::from_min_size(pos2(w - 152., 82.), vec2(128., 32.)),
                        Some(if m.unread { icons::Icon::MailOpen } else { icons::Icon::Mail }), unread_label, Color32::TRANSPARENT, MUTED)
                        .on_hover_text(format!("{unread_label} · {}", shortcuts::hint("Unread")))
                        .clicked()
                    {
                        self.action("Unread");
                    }
                    line(&p, pos2(lx, 131.), pos2(w, 131.));
                    let subject_rect = Rect::from_min_max(pos2(x, 152.), pos2(right, 186.));
                    mail_list_line_font(&p, subject_rect, &m.subject, FontId::new(T_TITLE + 2., FontFamily::Name("ui-medium".into())), INK);
                    ui.interact(subject_rect, Id::new("message-subject"), Sense::hover()).on_hover_text(&m.subject);
                    let folder_galley = p.layout_no_wrap(m.folder.clone(), FontId::new(T_CAPTION, FontFamily::Name("ui-medium".into())), self.theme.color());
                    let folder_rect = Rect::from_min_size(pos2(x, 206.), vec2(folder_galley.size().x + 18., 24.));
                    p.rect_filled(folder_rect, 12., self.theme.tint(0.09));
                    p.galley(folder_rect.center() - folder_galley.size() / 2., folder_galley, self.theme.color());
                    ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(pos2(folder_rect.right() + 12., 202.), vec2((right - x - 70.).max(100.), 30.))), |ui| {
                        ui.horizontal(|ui| {
                            let mut category = m.category.clone();
                            ComboBox::from_id_salt("message-label").selected_text(&category).width(106.).show_ui(ui, |ui| {
                                for label in classifier::CATEGORIES { ui.selectable_value(&mut category, label.into(), label); }
                            });
                            let mut labels = m.labels.clone();
                            ComboBox::from_id_salt("message-tags").selected_text(if labels.is_empty() { "Add label".into() } else { labels.join(", ") }).width(110.).show_ui(ui, |ui| {
                                for label in self.classifier.settings.labels() {
                                    let mut enabled = labels.iter().any(|l| l == &label);
                                    if ui.checkbox(&mut enabled, &label).changed() {
                                        labels.retain(|l| l != &label);
                                        if enabled { labels.push(label.into()); }
                                    }
                                }
                            });
                            if category != m.category || labels != m.labels {
                                self.mails[self.selected].category = category;
                                self.mails[self.selected].labels = labels;
                                self.learn_classification(self.selected);
                            }
                            if ui.add_enabled(!self.classifier.busy() && !matches!(m.folder.as_str(), "Sent" | "Drafts" | "Trash"), Button::new("Auto-classify")).clicked() {
                                self.classifier.enqueue(vec![self.mails[self.selected].clone()]);
                            }
                        });
                    });
                    avatar(&p, pos2(x + 19., 270.), &m.initials, m.color, 19.);
                    let time_w = p.layout_no_wrap(m.time.clone(), FontId::proportional(T_CAPTION + 0.5), MUTED).size().x;
                    let name = sender_name(&m.from);
                    let address = m.from.split('<').nth(1).map(|a| a.trim_end_matches('>').trim()).unwrap_or("");
                    let name_galley = p.layout_no_wrap(name.to_owned(), FontId::new(T_BODY + 1., FontFamily::Name("ui-medium".into())), INK);
                    let name_w = name_galley.size().x.min(right - x - 60. - time_w - 20.);
                    mail_list_line_font(&p, Rect::from_min_size(pos2(x + 50., 251.), vec2(name_w, 20.)), name, FontId::new(T_BODY + 1., FontFamily::Name("ui-medium".into())), INK);
                    if !address.is_empty() {
                        mail_list_line(&p, Rect::from_min_max(pos2(x + 58. + name_w, 253.), pos2(right - time_w - 16., 271.)), address, T_SMALL, MUTED);
                    }
                    let mailbox_label = m.mailbox_label();
                    let mailbox_rect = Rect::from_min_max(
                        pos2(x + 50., 273.), pos2(right, 291.),
                    );
                    mail_list_line(&p, mailbox_rect, &mailbox_label, T_SMALL, MUTED);
                    ui.interact(mailbox_rect, Id::new("message-mailbox"), Sense::hover())
                        .on_hover_text(&mailbox_label);
                    p.text(pos2(right, 261.), Align2::RIGHT_CENTER, &m.time, FontId::proportional(T_CAPTION + 0.5), MUTED);
                    let text_rect = Rect::from_min_max(pos2(x, 315.), pos2(right, h - 90.));
                    let replying = self.compose && self.inline_reply;
                    let reply_height = (h - 650.).clamp(180., 320.);
                    let conversation_rect = if replying {
                        Rect::from_min_max(
                            text_rect.min,
                            pos2(right, text_rect.bottom() - reply_height - 190.),
                        )
                    } else {
                        text_rect
                    };
                    ui.scope_builder(UiBuilder::new().max_rect(conversation_rect), |ui| {
                        ScrollArea::vertical()
                            .id_salt("conversation")
                            .show(ui, |ui| {
                                let groups = threading::threads(&self.mails);
                                let members: Vec<Mail> = self
                                    .mails
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, _)| groups[*i] == groups[self.selected])
                                    .map(|(_, m)| m.clone())
                                    .collect();
                                for (index, mail) in members.iter().enumerate() {
                                    if index > 0 {
                                        ui.separator();
                                        ui.label(
                                            RichText::new(format!("{} · {}", mail.from, mail.time))
                                                .small()
                                                .color(MUTED),
                                        );
                                    }
                                    let mut body = editor::layout(&mail.body, &mail.marks, ui.available_width());
                                    for section in &mut body.sections {
                                        section.format.line_height = Some(section.format.font_id.size * 1.5);
                                    }
                                    ui.add(Label::new(body).wrap());
                                    ui.add_space(20.);
                                }
                                if !self.inline_reply
                                    && ui
                                        .add_sized(
                                            [ui.available_width(), 55.],
                                            Button::image_and_text(
                                                Image::new(icons::Icon::Reply.source()).fit_to_exact_size(vec2(16., 16.)).tint(MUTED),
                                                RichText::new("Reply").color(MUTED),
                                            )
                                            .shortcut_text(RichText::new("R").color(MUTED)),
                                        )
                                        .clicked()
                                {
                                    self.action("Reply");
                                }
                            });
                    });
                    if replying {
                        let reply_rect = Rect::from_min_max(
                            pos2(x, conversation_rect.bottom() + 12.),
                            text_rect.max,
                        );
                        ui.scope_builder(UiBuilder::new().max_rect(reply_rect), |ui| {
                            Frame::new()
                                .stroke(Stroke::new(1., LINE))
                                .corner_radius(CORNER_RADIUS)
                                .inner_margin(18.)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("↩ Reply to {}", self.to))
                                            .color(MUTED),
                                    );
                                    self.writer(ui, reply_height);
                                });
                        });
                    }

                }
                p.rect_filled(
                    Rect::from_min_max(pos2(sx, h - 34.), pos2(w, h)),
                    0.,
                    Color32::from_rgb(251, 251, 253),
                );
                line(&p, pos2(sx, h - 34.), pos2(w, h - 34.));
                txt(
                    &p,
                    pos2(sx + 24., h - 22.),
                    &format!("{} conversations", visible.len()),
                    T_CAPTION,
                    MUTED,
                );
                // Footer hints as keycaps, laid out right to left. Items that map to
                // a command (Sync, Commands, Shortcuts) are also clickable.
                let syncing = self.syncing.is_some();
                let sync_hint = shortcuts::hint("Sync all accounts");
                let commands_hint = shortcuts::hint("Commands");
                let hints: Vec<(&str, &str, Option<&'static str>)> = if self.keyboard.pending_g.is_some() {
                    vec![("I", "Inbox", None), ("S", "Starred", None), ("H", "Snoozed", None), ("T", "Sent", None),
                         ("D", "Drafts", None), ("A", "Archive", None), ("X", "Trash", None)]
                } else {
                    vec![
                        ("J K", "Navigate", None),
                        ("E", "Archive", None),
                        ("C", "Compose", None),
                        (&sync_hint, if syncing { "Syncing…" } else { "Sync" }, Some("Sync all accounts")),
                        (&commands_hint, "Commands", Some("Commands")),
                        ("?", "Shortcuts", Some("Keyboard shortcuts")),
                    ]
                };
                let foot_rect = Rect::from_min_max(pos2(sx + 150., h - 34.), pos2(w, h));
                let foot = p.with_clip_rect(foot_rect);
                let mut fx = w - 16.;
                let mut run = None;
                for (keys, label, command) in hints.iter().rev() {
                    let font = FontId::proportional(T_CAPTION);
                    let busy = *command == Some("Sync all accounts") && syncing;
                    let label_w = foot.layout_no_wrap(label.to_string(), font.clone(), MUTED).size().x;
                    let item_w = label_w + 6. + keycaps_width(&foot, keys);
                    let item = Rect::from_min_max(pos2(fx - item_w - 8., h - 29.), pos2(fx + 8., h - 5.));
                    if let Some(name) = command {
                        let response = ui.interact(item.intersect(foot_rect), Id::new(("footer-item", *name)), Sense::click())
                            .on_hover_text(*name)
                            .on_hover_cursor(CursorIcon::PointingHand);
                        let hover = icons::fade(ctx, response.id.with("h"), response.hovered(), icons::DUR_FAST);
                        foot.rect_filled(item, 6., Color32::from_black_alpha((10. * hover) as u8));
                        if response.clicked() && self.command_enabled(name) {
                            run = Some(*name);
                        }
                    }
                    let color = if busy {
                        // A gentle pulse while accounts sync.
                        ctx.request_repaint();
                        self.theme.color().gamma_multiply(0.6 + 0.4 * (ctx.input(|i| i.time) as f32 * 4.).sin().abs())
                    } else {
                        MUTED
                    };
                    foot.text(pos2(fx, h - 17.), Align2::RIGHT_CENTER, *label, font, color);
                    fx -= label_w + 6.;
                    fx -= paint_keycaps(&foot, pos2(fx, h - 18.), keys, true) + 22.;
                }
                if self.keyboard.pending_g.is_some() {
                    foot.text(pos2(fx, h - 17.), Align2::RIGHT_CENTER, "G then", FontId::new(T_CAPTION, FontFamily::Name("ui-medium".into())), self.theme.color());
                }
                if let Some(name) = run {
                    self.execute_command(name, ctx);
                }
            });
        if self.compose && !self.inline_reply {
            let mut open = true;
            Window::new("New message")
                .frame(glass_frame())
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(620.)
                .anchor(Align2::CENTER_CENTER, vec2(0., 0.))
                .show(ctx, |ui| {
                    paper_frame()
                        .inner_margin(Margin {
                            bottom: 10,
                            ..Margin::same(20)
                        })
                        .show(ui, |ui| {
                            if self.sending.is_some() {
                                ui.disable();
                            }
                            compose_address_row(ui, "To", &mut self.to, Some(&mut self.show_bcc));
                            ui.add_space(4.);
                            compose_address_row(ui, "CC", &mut self.cc, None);
                            if self.show_bcc {
                                ui.add_space(4.);
                                compose_address_row(ui, "Bcc", &mut self.bcc, None);
                            }
                            ui.add_space(6.);
                            let row = ui
                                .allocate_ui_with_layout(
                                    vec2(ui.available_width(), 36.),
                                    Layout::left_to_right(Align::Center),
                                    |ui| {
                                        ui.add_sized(
                                            vec2(ui.available_width(), 34.),
                                            TextEdit::singleline(&mut self.subject)
                                                .id_salt("compose-subject")
                                                .frame(false)
                                                .font(FontId::proportional(14.))
                                                .hint_text(
                                                    RichText::new("Add a subject")
                                                        .color(Color32::from_gray(117)),
                                                )
                                                .margin(vec2(8., 6.)),
                                        );
                                    },
                                )
                                .response
                                .rect;
                            ui.painter().hline(
                                row.x_range(),
                                row.bottom(),
                                Stroke::new(1., Color32::from_gray(170)),
                            );
                            // Reserve status space only when there is a saved draft to show.
                            if self.draft_saved {
                                ui.allocate_ui_with_layout(
                                    vec2(ui.available_width(), 20.),
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new("Draft saved").size(13.).color(MUTED),
                                        );
                                    },
                                );
                            }
                            self.writer(ui, 280.);
                        });
                });
            if self.compose && open && self.sending.is_none() {
                let signature = serde_json::to_string(&(
                    &self.to,
                    &self.cc,
                    &self.bcc,
                    &self.subject,
                    &self.body,
                    &self.editor.marks,
                    self.account.as_ref().map(|a| a.scope()),
                ))
                .unwrap_or_default();
                if signature != self.draft_signature {
                    self.draft_signature = signature;
                    self.draft_dirty_since = Some(std::time::Instant::now());
                    self.draft_saved = false;
                }
                if let Some(since) = self.draft_dirty_since {
                    ctx.request_repaint_after(std::time::Duration::from_millis(800));
                    if since.elapsed().as_millis() >= 800
                        && (!self.to.is_empty()
                            || !self.cc.is_empty()
                            || !self.bcc.is_empty()
                            || !self.subject.is_empty()
                            || !self.body.is_empty()
                            || self.editing_draft.is_some())
                    {
                        self.save_composed("Drafts");
                        self.draft_dirty_since = None;
                    }
                }
            }
            if !open && self.sending.is_none() {
                self.store_composed("Drafts");
                self.toast = Toast::new("Draft saved");
            }
        }
        let palette_t = icons::fade(ctx, Id::new("palette-anim"), self.palette, icons::DUR_MED);
        let guide_t = icons::fade(ctx, Id::new("guide-anim"), self.keyboard.help, icons::DUR_MED);
        let search_t = icons::fade(ctx, Id::new("search-anim"), self.search_open, icons::DUR_MED);
        if self.palette {
            self.command_palette(ctx, palette_t);
        }
        if self.search_open {
            self.search_modal(ctx, search_t);
        }
        if self.settings {
            self.account_settings(ctx);
        }
        if self.keyboard.help {
            self.shortcut_guide(ctx, guide_t);
        }
        if self.welcome {
            self.welcome_modal(ctx);
        }
        self.toast.show(ctx);
    }
}
fn data_directory() -> Result<std::path::PathBuf, String> {
    // A separate mailbox for demos and recordings, never the real one.
    if let Some(dir) = std::env::var_os("SPARKY_DATA_DIR").filter(|d| !d.is_empty()) {
        return Ok(std::path::PathBuf::from(dir));
    }
    // Development keeps its existing mailbox; distributed builds never reference
    // the developer's source folder or write inside the application bundle.
    if cfg!(debug_assertions) {
        return Ok(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    }
    let home = std::env::var_os("HOME").ok_or("Could not locate your home folder")?;
    Ok(std::path::PathBuf::from(home).join("Library/Application Support/Sparky"))
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1440., 900.])
            .with_min_inner_size([1180., 800.])
            .with_title("Sparky — a little clarity")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/Sparky.png"))
                    .expect("embedded Sparky app icon must be a valid PNG"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Sparky",
        options,
        Box::new(move |cc| {
            let record = args.iter().position(|a| a == "--record")
                .and_then(|i| Some((args.get(i + 1)?.clone(), args.get(i + 2)?.clone())));
            // Recording, like screenshots, never touches the Keychain.
            let mut app = App::new(cc, screenshot.clone().or_else(|| record.as_ref().map(|_| String::new())));
            if let Some((dir, script)) = &record {
                app.screenshot = None;
                app.welcome = false;
                app.recorder = Some(recorder::Recorder::new(dir, script));
            }
            let query = args.iter().position(|a| a == "--query").and_then(|i| args.get(i + 1)).cloned();
            if screenshot.is_some() {
                if args.iter().any(|arg| arg == "--preview") {
                    app.welcome = false;
                }
                match args
                    .iter()
                    .position(|a| a == "--preview")
                    .and_then(|i| args.get(i + 1))
                    .map(String::as_str)
                {
                    Some("toast") => app.toast = Toast::new("Moved to Trash"),
                    Some("compose") => app.begin_compose(),
                    Some("reply") => app.action("Reply"),
                    Some("commands") => app.open_commands(),
                    Some("search") => app.open_search(),
                    Some("search-results") => {
                        app.open_search();
                        app.search_query = query.clone().unwrap_or_else(|| "account".into());
                    }
                    Some("commands-search") => {
                        app.open_commands();
                        app.command_search = query.clone().unwrap_or_else(|| "go to".into());
                    }
                    Some("swipe") => {
                        if let Some(&(idx, _)) = app.expanded_rows().get(1) {
                            app.swipe = Some((idx, -SWIPE_OPEN));
                        }
                    }
                    Some("accounts") => app.settings = true,
                    Some("shortcuts") => {
                        app.keyboard.help = true;
                        app.keyboard.focus_help = true;
                    }
                    _ => {}
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    /// A context with the app's real font families, as `App::new` installs them.
    fn test_context() -> Context {
        let ctx = Context::default();
        ctx.set_fonts(app_fonts());
        egui_extras::install_image_loaders(&ctx);
        ctx
    }
    #[test]
    fn detail_shows_receiving_address_from_message_even_when_another_account_is_active() {
        let mut app = mailbox();
        app.account = Some(sync::Account::new("gmail".into(), "other@example.com".into(), "test-token".into(), "".into(), "".into()).unwrap());
        for scope in ["gmail:received@example.com", "graph:received@example.com", "imap:received@example.com", "received@example.com"] {
            app.mails[0].account = scope.into();
            assert_eq!(app.mails[0].mailbox_label(), "To: received@example.com");
        }
        app.mails[0].account.clear();
        assert_eq!(app.mails[0].mailbox_label(), "Sample message");
        app.mails[0].remote_id = "remote".into();
        assert_eq!(app.mails[0].mailbox_label(), "Receiving account unavailable");
    }

    #[test]
    fn initial_download_opens_received_mail_within_current_account_filter() {
        let mut app = mailbox();
        let mut received = app.mails[0].clone();
        received.account = "gmail:received@example.com".into();
        received.remote_id = "downloaded".into();
        app.mails.push(received.clone());
        app.select_received_mail();
        assert_eq!(app.selected, app.mails.len() - 1);
        assert_eq!(app.mails[app.selected].mailbox_label(), "To: received@example.com");
        app.account_filter = Some("gmail:other@example.com".into());
        received.account = "gmail:other@example.com".into();
        app.mails.push(received);
        app.select_received_mail();
        assert_eq!(app.selected, app.mails.len() - 1);
    }

    #[test]
    fn toast_fades_and_expires_after_five_seconds() {
        let toast = Toast::new("Moved to Trash");
        assert_eq!(toast.opacity(0.), 1.);
        assert_eq!(toast.opacity(4.5), 1.);
        assert!((toast.opacity(4.8) - 0.5).abs() < 0.001);
        assert_eq!(toast.opacity(5.), 0.);
        assert_eq!(toast.opacity(6.), 0.);
        let ctx = test_context();
        let mut expired = Toast::new("Done");
        expired.started = Some(std::time::Instant::now() - std::time::Duration::from_secs(6));
        expired.show(&ctx);
        assert!(expired.started.is_none());
        assert!(expired.message.is_empty());
    }

    #[test]
    fn sync_without_accounts_opens_connection_settings() {
        let mut app = mailbox();
        app.accounts.clear();
        app.start_sync();
        assert!(app.settings);
        assert!(app.syncing.is_none());
        assert!(app.toast.message.contains("No accounts are connected"));
    }

    #[test]
    fn old_downloaded_accounts_prompt_reconnection_instead_of_claiming_none() {
        let mut app = mailbox();
        app.accounts.clear();
        app.mails[0].account = "gmail:previous@example.com".into();
        app.start_sync();
        assert!(app.settings);
        assert!(app.syncing.is_none());
        assert!(app.toast.message.contains("Reconnect your accounts"));
        assert!(!app.toast.message.contains("No accounts"));
    }

    #[test]
    fn sync_attempts_every_account_and_preserves_successful_mail() {
        let accounts: Vec<_> = ["one@example.com", "two@example.com", "three@example.com"]
            .into_iter()
            .map(|address| sync::Account::new("gmail".into(), address.into(), "test-token".into(), "".into(), "".into()).unwrap())
            .collect();
        let mut visited = Vec::new();
        let messages = mailbox().mails;
        assert!(!messages.is_empty());
        let batch = sync::fetch_all(&accounts, |account| {
            visited.push(account.address.clone());
            if account.address == "two@example.com" {
                Err("Connection failed".into())
            } else {
                Ok(messages.clone())
            }
        });
        assert_eq!(visited.len(), 3);
        assert_eq!(batch.mails.len(), messages.len() * 2);
        assert_eq!(batch.failures, vec!["two@example.com: Connection failed"]);
    }

    #[test]
    fn verified_google_connection_does_not_wait_for_inbox_or_disappear_on_sync_failure() {
        let mut app = mailbox();
        app.accounts.clear();
        app.welcome = true;
        app.connect_error = "Previous attempt failed".into();
        let account = sync::Account::new("gmail".into(), "test@example.com".into(), "test-token".into(), "".into(), "".into()).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        app.syncing = Some(rx);
        app.register_connected_account(account.clone());
        app.poll_network(); // The inbox worker has not returned yet.
        assert_eq!(app.accounts.len(), 1);
        assert_eq!(app.account.as_ref().unwrap().address, "test@example.com");
        assert!(!app.welcome);
        assert!(app.connect_error.is_empty());
        assert!(app.pending_account.is_none());
        assert!(app.syncing.is_some());
        tx.send(Err("Inbox temporarily unavailable".into())).unwrap();
        app.poll_network();
        assert!(app.syncing.is_none());
        assert_eq!(app.accounts.len(), 1);
        assert!(app.toast.message.contains("Inbox temporarily unavailable"));
        app.register_connected_account(account);
        assert_eq!(app.accounts.len(), 1); // Reconnection replaces the same account.
    }

    #[test]
    fn connection_removes_samples_before_download_and_keeps_local_drafts() {
        let mut app = mailbox();
        app.accounts.clear();
        app.mails[0].folder = "Archive".into();
        app.mails[1].folder = "Trash".into();
        let mut draft = app.mails[0].clone();
        draft.folder = "Drafts".into();
        app.mails.push(draft);
        app.editing_draft = Some(app.mails.len() - 1);
        app.selected = app.mails.len() - 1;
        app.compose = true;
        let account = sync::Account::new("gmail".into(), "test@example.com".into(), "test-token".into(), "".into(), "".into()).unwrap();
        app.register_connected_account(account);
        assert_eq!(app.mails.len(), 1);
        assert_eq!(app.mails[0].folder, "Drafts");
        assert_eq!(app.editing_draft, Some(0));
        assert_eq!(app.selected, 0);
        app.accounts.clear();
        app.remove_sample_messages();
        assert_eq!(app.mails.len(), 1); // Disconnecting cannot restore sample mail.
    }

    #[test]
    fn returning_mailbox_removes_legacy_samples_and_persists_cleanup() {
        let mut app = mailbox();
        app.accounts.clear();
        app.remove_sample_messages();
        assert_eq!(app.mails.len(), sample().len()); // New users can still explore.
        let dir = std::env::temp_dir().join(format!("sparky-samples-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&dir).unwrap();
        app.store = Some(storage::Store::open(&dir.join("mail.sqlite3"), &dir.join("mailbox.json")).unwrap());
        let mut received = app.mails[0].clone();
        received.account = "gmail:test@example.com".into();
        received.remote_id = "real-message".into();
        app.mails.push(received);
        app.remove_sample_messages();
        assert_eq!(app.mails.len(), 1);
        assert_eq!(app.mails[app.selected].remote_id, "real-message");
        assert_eq!(app.store.as_ref().unwrap().load().unwrap().len(), 1);
        drop(app);
        let reopened = storage::Store::open(&dir.join("mail.sqlite3"), &dir.join("mailbox.json")).unwrap();
        assert_eq!(reopened.load().unwrap()[0].remote_id, "real-message");
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn connected_account_closes_welcome() {
        use eframe::App as _;
        let ctx = test_context();
        let mut app = mailbox();
        app.welcome = true;
        app.accounts.push(sync::Account::new("gmail".into(), "test@example.com".into(), "test-token".into(), "".into(), "".into()).unwrap());
        let mut frame = eframe::Frame::_new_kittest();
        let _ = ctx.run(input(vec![]), |ctx| app.update(ctx, &mut frame));
        assert!(!app.welcome);
    }

    #[test]
    fn swipe_settles_smoothly_at_different_refresh_rates() {
        let simulate = |hz: u32| {
            let mut motion = SwipeMotion::default();
            for _ in 0..hz / 4 {
                let previous = motion.offset;
                motion.settle(-160., 1. / hz as f32);
                assert!(motion.offset <= previous && motion.offset >= -160.);
            }
            motion
        };
        let sixty = simulate(60);
        let fast = simulate(120);
        assert!((sixty.offset - fast.offset).abs() < 0.001);
        assert!((sixty.velocity - fast.velocity).abs() < 0.01);
        let mut motion = simulate(60);
        for _ in 0..60 {
            motion.settle(-160., 1. / 60.);
        }
        assert_eq!(motion.offset, -160.);
        assert_eq!(motion.velocity, 0.);
        assert!(!motion.settle(-160., 1. / 60.));
    }

    #[test]
    fn swipe_can_close_and_be_grabbed_mid_animation() {
        let mut motion = SwipeMotion::default();
        motion.follow(-160.);
        assert!(motion.settle(0., 1. / 60.));
        assert!(motion.offset > -160. && motion.offset < -140.);
        for _ in 0..6 {
            motion.settle(0., 1. / 60.);
        }
        let grabbed = motion.offset - 10.;
        motion.follow(grabbed);
        assert_eq!(motion.offset, grabbed);
        assert_eq!(motion.velocity, 0.);
        assert!(motion.settle(-160., 1. / 120.));
        assert!(motion.offset < grabbed && motion.offset > grabbed - 5.);
        for _ in 0..120 {
            motion.settle(0., 1. / 120.);
        }
        assert_eq!(motion.offset, 0.);
        assert!(!motion.settle(0., 1. / 120.));
    }

    use super::*;
    fn mailbox() -> App {
        App {
            theme: Default::default(),
            classifier: Default::default(),
            mails: sample(),
            selected: 0,
            folder: "Inbox".into(),
            category: "All".into(),
            search: String::new(),
            compose: false,
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            show_bcc: false,
            draft_signature: String::new(),
            draft_dirty_since: None,
            draft_saved: false,
            subject: String::new(),
            body: String::new(),
            palette: false,
            command_search: String::new(),
            command_index: 0,
            command_focus: false,
            keyboard: shortcuts::State::default(),
            editor: editor::Editor::default(),
            writer_focus: false,
            inline_reply: false,
            accounts: sync::Account::configured().into_iter().collect(),
            account_filter: None,
            pending_account: None,
            google_signin: None,
            google_setup_json: String::new(),
            connect_provider: "gmail".into(),
            connect_address: String::new(),
            connect_secret: String::new(),
            connect_imap: String::new(),
            connect_smtp: String::new(),
            connect_error: String::new(),
            toast: Toast::default(),
            frames: 0,
            screenshot: None,
            saved_shot: false,
            settings: false,
            welcome: false,
            scroll: 0.,
            collapsed_dates: [false; 3],
            swipe: None,
            swipe_motion: Default::default(),
            swipe_last_input: 0.,
            leaving: None,
            now: 0.,
            recent_commands: Vec::new(),
            recorder: None,
            credentials_error: None,
            search_open: false,
            search_query: String::new(),
            search_index: 0,
            search_focus: false,
            recent_searches: Vec::new(),
            editing_draft: None,
            replying_to: None,
            store: None,
            account: None,
            syncing: None,
            sending: None,
        }
    }
    #[test]
    fn classification_targets_identity_after_insertion_and_respects_manual_changes() {
        let mut app = mailbox();
        for (i, mail) in app.mails.iter_mut().enumerate() { mail.message_id = format!("<identity-{i}@local>"); }
        let active = app.mails[0].clone();
        let key = classifier::identity(&active);
        let (tx, rx) = std::sync::mpsc::channel();
        app.classifier.active = Some(active);
        app.classifier.result = Some(rx);
        app.classifier.total = 1;
        app.mails.insert(0, app.mails[1].clone());
        tx.send(Ok(Some(classifier::Decision { category: "Notifications".into(), labels: vec!["Work".into()], confidence: 0.95, suggestions: Vec::new() }))).unwrap();
        app.poll_classifier();
        assert!(app.mails[1].labels.contains(&"Work".into()));
        app.folder = "Work".into();
        assert_eq!(app.visible(), vec![1]);
        assert_eq!(app.mails.iter().find(|m| classifier::identity(m) == key).unwrap().category, "Notifications");
        assert_eq!(app.classifier.done, 1);
        let active = app.mails[1].clone();
        let (tx, rx) = std::sync::mpsc::channel();
        app.classifier.active = Some(active);
        app.classifier.result = Some(rx);
        app.mails[1].category = "Newsletters".into();
        tx.send(Ok(Some("People".into()))).unwrap();
        app.poll_classifier();
        assert_eq!(app.mails[1].category, "Newsletters");
    }
    #[test]
    fn manual_corrections_clear_reviews_and_override_stale_results() {
        let mut app = mailbox();
        app.mails[0].message_id = "<learning@local>".into();
        app.mails[0].category = "Notifications".into();
        app.mails[0].labels = vec!["Banking".into()];
        app.learn_classification(0);
        assert_eq!(app.classifier.settings.corrections.len(), 1);
        let active = app.mails[0].clone();
        let (tx, rx) = std::sync::mpsc::channel();
        app.classifier.active = Some(active); app.classifier.result = Some(rx);
        tx.send(Ok(Some(classifier::Decision { category: "People".into(), labels: vec!["Work".into()],
            confidence: 0.99, suggestions: Vec::new() }))).unwrap();
        app.poll_classifier();
        assert_eq!(app.mails[0].category, "Notifications");
        assert_eq!(app.mails[0].labels, vec!["Banking"]);
        app.mails[0].labels.clear(); app.learn_classification(0);
        assert!(app.classifier.settings.corrected(&app.mails[0]).unwrap().labels.is_empty());
        assert_eq!(app.classifier.settings.corrections.len(), 1);
    }
    #[test]
    fn low_confidence_waits_for_edited_criteria_and_reuses_approved_choices() {
        let mut app = mailbox();
        for (i, mail) in app.mails.iter_mut().enumerate() { mail.message_id = format!("<review-{i}@local>"); }
        let original = app.mails[0].clone();
        let (tx, rx) = std::sync::mpsc::channel();
        app.classifier.active = Some(original.clone()); app.classifier.result = Some(rx);
        tx.send(Ok(Some(classifier::Decision { category: "Notifications".into(), labels: Vec::new(), confidence: 0.4,
            suggestions: vec![classifier::Criterion { name: "Bills".into(), description: "Invoices and payment reminders".into() }] }))).unwrap();
        app.poll_classifier();
        assert_eq!(app.mails[0].category, original.category);
        assert_eq!(app.mails[0].labels, original.labels);
        assert!(!app.classifier.settings.labels().contains(&"Bills".into()));
        // Pending edits and accepted criteria survive serialization used by SQLite.
        let encoded = serde_json::to_string(&app.classifier.settings).unwrap();
        app.classifier.settings = serde_json::from_str(&encoded).unwrap();
        app.classifier.settings.reviews[0].decision.suggestions[0].name = "Finance".into();
        assert!(app.accept_classifier_criterion(0, 0));
        assert!(app.mails[0].labels.contains(&"Finance".into()));
        assert_eq!(app.mails[0].category, "Notifications");
        assert!(app.classifier.settings.reviews.is_empty());
        assert!(app.classifier.settings.labels().contains(&"Finance".into()));
        app.folder = "Finance".into(); app.category = "All".into();
        assert_eq!(app.visible(), vec![0]);
    }
    #[test]
    fn review_acceptance_respects_later_manual_edits() {
        let mut app = mailbox();
        let mail = app.mails[0].clone();
        app.classifier.settings.reviews.push(classifier::Review { key: classifier::identity(&mail), subject: mail.subject,
            original_category: mail.category, original_labels: mail.labels,
            decision: classifier::Decision { category: "People".into(), labels: Vec::new(), confidence: 0.2,
                suggestions: vec![classifier::Criterion { name: "Finance".into(), description: "Invoices".into() }] } });
        app.mails[0].category = "Manually corrected".into();
        assert!(!app.accept_classifier_criterion(0, 0));
        assert!(app.classifier.settings.criteria.is_empty());
        assert_eq!(app.classifier.settings.reviews.len(), 1);
        assert_eq!(app.mails[0].category, "Manually corrected");
    }
    #[test]
    fn cancellation_ignores_inflight_and_sync_preserves_labels() {
        let mut app = mailbox();
        for (i, mail) in app.mails.iter_mut().enumerate() { mail.message_id = format!("<fixture-{i}@local>"); }
        app.classifier.enqueue(app.mails.clone());
        app.classifier.enqueue(app.mails.clone());
        assert_eq!(app.classifier.total, app.mails.len());
        let active = app.classifier.queue.pop_front().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        app.classifier.active = Some(active);
        app.classifier.result = Some(rx);
        app.classifier.cancel();
        tx.send(Ok(Some("Notifications".into()))).unwrap();
        app.poll_classifier();
        assert_eq!(app.mails[0].category, "People");
        assert!(!app.classifier.busy());
        app.mails[0].account = "gmail:test@example.com".into();
        app.mails[0].remote_id = "remote1".into();
        app.mails[0].category = "Newsletters".into();
        app.mails[0].labels = vec!["Personal".into()];
        let mut remote = app.mails[0].clone();
        remote.category = "People".into();
        remote.labels.clear();
        let (tx, rx) = std::sync::mpsc::channel();
        app.syncing = Some(rx);
        tx.send(Ok(sync::SyncBatch::from(vec![remote]))).unwrap();
        app.poll_network();
        assert_eq!(app.mails[0].category, "Newsletters");
        assert_eq!(app.mails[0].labels, vec!["Personal"]);
    }
    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            events,
            ..Default::default()
        }
    }
    fn key(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn collapsed_dates_hide_rows_and_keep_other_sections_available() {
        let mut app = mailbox();
        assert_eq!(app.expanded_rows().len(), 8);
        app.collapsed_dates[0] = true;
        assert_eq!(app.expanded_rows().iter().map(|(idx, _)| *idx).collect::<Vec<_>>(), vec![4, 5, 6, 7]);
        assert_eq!(app.expanded_rows()[0].1, 68.);
        app.collapsed_dates[1] = true;
        assert_eq!(app.expanded_rows(), vec![(7, 102.)]);
        app.collapsed_dates[2] = true;
        assert!(app.expanded_rows().is_empty());
        assert_eq!(app.visible().len(), 8); // Collapsing never removes mail.
        app.collapsed_dates[0] = false;
        assert_eq!(app.expanded_rows().len(), 4);
    }

    #[test]
    fn swipe_buttons_act_on_the_swiped_row() {
        use eframe::App as _;
        let ctx = test_context();
        let mut app = mailbox();
        let mut frame = eframe::Frame::_new_kittest();
        let mut time = 0.;
        let mut tick = |app: &mut App, events| {
            time += 0.2;
            let mut raw = input(events);
            raw.time = Some(time);
            let _ = ctx.run(raw, |ctx| app.update(ctx, &mut frame));
        };
        let pointer = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        tick(&mut app, vec![]);
        tick(
            &mut app,
            vec![
                Event::PointerMoved(pos2(380., 320.)),
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(-120., 0.),
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        assert_eq!(app.selected, 0);
        assert_eq!(app.swipe, Some((1, -120.)));
        assert_eq!(app.swipe_motion[&1].offset, -120.);
        tick(&mut app, vec![]);
        assert_eq!(app.swipe, Some((1, -SWIPE_OPEN)));
        assert!(app.swipe_motion[&1].offset < -120.);
        assert!(app.swipe_motion[&1].offset > -SWIPE_OPEN);
        // List right edge is 636 at 1440 wide; Archive and Delete tiles are 87 wide.
        let archive = pos2(505., 330.);
        tick(
            &mut app,
            vec![Event::PointerMoved(archive), pointer(archive, true)],
        );
        tick(&mut app, vec![pointer(archive, false)]);
        // The row slides out and collapses before the move lands.
        assert!(app.leaving.as_ref().is_some_and(|l| l.idx == 1));
        assert_eq!(app.mails[1].folder, "Inbox");
        tick(&mut app, vec![]);
        tick(&mut app, vec![]);
        assert!(app.leaving.is_none());
        assert_eq!(app.mails[1].folder, "Archive");
        assert_eq!(app.mails[0].folder, "Inbox");
        assert!(app.swipe.is_none());
        let start = pos2(400., 235.);
        tick(
            &mut app,
            vec![Event::PointerMoved(start), pointer(start, true)],
        );
        tick(&mut app, vec![Event::PointerMoved(pos2(280., 235.))]);
        tick(&mut app, vec![pointer(pos2(280., 235.), false)]);
        assert_eq!(app.swipe, Some((0, -SWIPE_OPEN)));
        let delete = pos2(592., 235.);
        tick(
            &mut app,
            vec![Event::PointerMoved(delete), pointer(delete, true)],
        );
        tick(&mut app, vec![pointer(delete, false)]);
        tick(&mut app, vec![]);
        tick(&mut app, vec![]);
        assert_eq!(app.mails[0].folder, "Trash");
        assert!(app.swipe.is_none());
    }
    #[test]
    fn swipe_past_the_open_width_resists_like_a_rubber_band() {
        use eframe::App as _;
        let ctx = test_context();
        let mut app = mailbox();
        let mut frame = eframe::Frame::_new_kittest();
        let mut raw = input(vec![]);
        raw.time = Some(0.2);
        let _ = ctx.run(raw, |ctx| app.update(ctx, &mut frame));
        let mut raw = input(vec![
            Event::PointerMoved(pos2(380., 330.)),
            Event::MouseWheel { unit: MouseWheelUnit::Point, delta: vec2(-240., 0.), modifiers: Modifiers::NONE },
        ]);
        raw.time = Some(0.4);
        let _ = ctx.run(raw, |ctx| app.update(ctx, &mut frame));
        assert_eq!(app.swipe, Some((1, -240.)));
        // 72 px past the open width moves only a quarter as far.
        assert_eq!(app.swipe_motion[&1].offset, -SWIPE_OPEN - 18.);
    }
    #[test]
    fn keyboard_folder_chords_actions_and_draft_opening() {
        let ctx = test_context();
        let mut app = mailbox();
        let _ = ctx.run(input(vec![key(Key::G), key(Key::S)]), |ctx| {
            app.handle_shortcuts(ctx)
        });
        assert_eq!(app.folder, "Starred");
        assert!(app.visible().contains(&app.selected));
        let selected = app.selected;
        let _ = ctx.run(input(vec![key(Key::H)]), |ctx| app.handle_shortcuts(ctx));
        assert_eq!(app.mails[selected].folder, "Snoozed");
        assert!(app.mails[selected].snoozed_until.is_some());
        app.go_to_folder("Snoozed");
        app.execute_command("Move to Inbox", &ctx);
        // The row animates out first; the move lands when the exit finishes.
        assert_eq!(app.mails[selected].folder, "Snoozed");
        assert!(app.leaving.is_some());
        app.finish_leaving();
        assert!(app.leaving.is_none());
        assert_eq!(app.mails[selected].folder, "Inbox");
        assert!(app.mails[selected].snoozed_until.is_none());
        app.begin_compose();
        app.body = "Draft opened with Enter".into();
        app.store_composed("Drafts");
        app.go_to_folder("Drafts");
        let _ = ctx.run(input(vec![key(Key::Enter)]), |ctx| {
            app.handle_shortcuts(ctx)
        });
        assert!(app.compose && !app.inline_reply);
        assert_eq!(app.body, "Draft opened with Enter");
    }
    #[test]
    fn keyboard_scopes_protect_hidden_mail_and_active_writers() {
        let ctx = test_context();
        let mut app = mailbox();
        app.folder = "Trash".into();
        let before = serde_json::to_string(&app.mails).unwrap();
        app.execute_command("Delete", &ctx);
        app.execute_command("Reply", &ctx);
        assert_eq!(before, serde_json::to_string(&app.mails).unwrap());
        assert!(!app.compose);
        app.begin_compose();
        app.body = "Keep my work".into();
        let _ = ctx.run(input(vec![key(Key::E), key(Key::R)]), |ctx| {
            app.handle_shortcuts(ctx)
        });
        assert_eq!(app.body, "Keep my work");
        assert!(!app.inline_reply);
        app.keyboard.help = true;
        let _ = ctx.run(input(vec![key(Key::Escape)]), |ctx| {
            app.handle_shortcuts(ctx)
        });
        assert!(!app.keyboard.help && app.compose);
        let mut escape_release = key(Key::Escape);
        if let Event::Key { pressed, .. } = &mut escape_release {
            *pressed = false;
        }
        let _ = ctx.run(input(vec![escape_release, key(Key::Escape)]), |ctx| {
            app.handle_shortcuts(ctx)
        });
        assert!(!app.compose);
        assert_eq!(app.mails.last().unwrap().body, "Keep my work");
    }
    #[test]
    fn commands_filter_and_execute_with_keyboard() {
        let ctx = test_context();
        let mut app = mailbox();
        app.open_commands();
        let _ = ctx.run(input(vec![]), |ctx| app.command_palette(ctx, 1.));
        let _ = ctx.run(input(vec![Event::Text("account".into())]), |ctx| {
            app.command_palette(ctx, 1.)
        });
        assert_eq!(app.command_search, "account");
        let _ = ctx.run(input(vec![key(Key::ArrowDown)]), |ctx| {
            app.command_palette(ctx, 1.)
        });
        assert_eq!(app.command_index, 1);
        let _ = ctx.run(input(vec![key(Key::ArrowUp), key(Key::Enter)]), |ctx| {
            app.command_palette(ctx, 1.)
        });
        assert!(app.settings);
        assert!(!app.palette);
        assert_eq!(app.recent_commands, vec!["Connect email account".to_owned()]);
        // Fuzzy: "gti" runs Go to Inbox from anywhere.
        app.settings = false;
        app.go_to_folder("Archive");
        app.open_commands();
        let _ = ctx.run(input(vec![Event::Text("gti".into())]), |ctx| app.command_palette(ctx, 1.));
        let _ = ctx.run(input(vec![key(Key::Enter)]), |ctx| app.command_palette(ctx, 1.));
        assert_eq!(app.folder, "Inbox");
        assert_eq!(app.recent_commands[0], "Go to Inbox");
    }
    #[test]
    fn search_modal_finds_mail_in_any_folder_and_opens_it() {
        let ctx = test_context();
        let mut app = mailbox();
        let maya = app.mails.iter().position(|m| m.subject == "Coffee next week?").unwrap();
        app.mails[maya].folder = "Archive".into();
        // "/" opens the modal instead of focusing a field.
        let _ = ctx.run(input(vec![key(Key::Slash)]), |ctx| app.handle_shortcuts(ctx));
        assert!(app.search_open);
        let _ = ctx.run(input(vec![]), |ctx| app.search_modal(ctx, 1.));
        let _ = ctx.run(input(vec![Event::Text("coffee".into())]), |ctx| app.search_modal(ctx, 1.));
        assert_eq!(app.search_query, "coffee");
        // Grouped by folder: the Inbox hit first, then Archive.
        let results = app.search_results("coffee");
        assert_eq!(results.len(), 2);
        assert_eq!(app.mails[results[0]].folder, "Inbox");
        assert_eq!(results[1], maya);
        let _ = ctx.run(input(vec![key(Key::ArrowDown), key(Key::Enter)]), |ctx| app.search_modal(ctx, 1.));
        assert!(!app.search_open);
        assert_eq!(app.folder, "Archive");
        assert_eq!(app.selected, maya);
        assert!(!app.mails[maya].unread);
        assert_eq!(app.recent_searches, vec!["coffee".to_owned()]);
        // The last item keeps the query as a filter on the current list.
        app.go_to_folder("Inbox");
        app.open_search();
        app.search_query = "coffee".into();
        let _ = ctx.run(input(vec![key(Key::ArrowUp), key(Key::Enter)]), |ctx| app.search_modal(ctx, 1.));
        assert_eq!(app.search, "coffee");
        assert_eq!(app.visible().len(), 1);
        // Esc closes the modal before anything else.
        app.open_search();
        app.execute_command("Close / save draft", &ctx);
        assert!(!app.search_open);
        assert_eq!(app.search, "coffee");
    }
    #[test]
    fn search_highlights_word_prefixes_and_excerpts_around_the_match() {
        let tokens = vec!["cof".to_owned(), "tue".to_owned()];
        let text = "Decaf coffee, Tuesday";
        let hits = token_hits(text, &tokens);
        assert_eq!(hits, vec![6, 7, 8, 14, 15, 16]); // "cof" and "Tue", not "caf" inside "Decaf".
        let body = format!("{} the coffee shop is new", "word ".repeat(30));
        let excerpt = snippet(&body, &["coffee".to_owned()]);
        assert!(excerpt.starts_with('…') && excerpt.contains("coffee shop"));
        assert_eq!(snippet("Short body", &["zzz".to_owned()]), "Short body");
    }
    #[test]
    fn list_navigation_wraps_and_reports_key_presses() {
        let ctx = test_context();
        let mut index = 0;
        let mut moved = false;
        let _ = ctx.run(input(vec![key(Key::ArrowUp)]), |ctx| moved = list_navigation(ctx, &mut index, 5));
        assert!(moved);
        assert_eq!(index, 4);
        let _ = ctx.run(input(vec![key(Key::PageDown)]), |ctx| moved = list_navigation(ctx, &mut index, 5));
        assert_eq!(index, 4);
        let _ = ctx.run(input(vec![]), |ctx| moved = list_navigation(ctx, &mut index, 5));
        assert!(!moved);
    }
    #[test]
    fn footer_items_run_their_commands_when_clicked() {
        use eframe::App as _;
        let ctx = test_context();
        let mut app = mailbox();
        let mut frame = eframe::Frame::_new_kittest();
        let mut time = 0.;
        let mut click = |app: &mut App, at: Pos2| {
            for pressed in [true, false] {
                time += 0.1;
                let mut raw = input(vec![Event::PointerMoved(at), Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }]);
                raw.time = Some(time);
                let _ = ctx.run(raw, |ctx| app.update(ctx, &mut frame));
            }
        };
        // egui hit-tests against the previous frame's widgets.
        click(&mut app, pos2(700., 500.));
        // Footer, right to left at 1440 wide: "? Shortcuts", then "⌘ K Commands".
        click(&mut app, pos2(1400., 883.));
        assert!(app.keyboard.help);
        app.keyboard.help = false;
        click(&mut app, pos2(700., 500.)); // Let the closed guide's backdrop go away.
        click(&mut app, pos2(1300., 883.));
        assert!(app.palette);
    }
    #[test]
    fn autosaved_draft_keeps_copy_recipients_without_duplicates() {
        let mut app = mailbox();
        app.begin_compose();
        app.to = "alex@example.com".into();
        app.cc = "team@example.com".into();
        app.bcc = "private@example.com".into();
        app.body = "First version".into();
        let count = app.mails.len();
        app.save_composed("Drafts");
        assert!(app.compose);
        app.body = "Updated version".into();
        app.save_composed("Drafts");
        assert_eq!(app.mails.len(), count + 1);
        let draft = &app.mails[app.editing_draft.unwrap()];
        let restored: Mail = serde_json::from_str(&serde_json::to_string(draft).unwrap()).unwrap();
        assert_eq!(restored.cc, "team@example.com");
        assert_eq!(restored.bcc, "private@example.com");
        assert_eq!(restored.body, "Updated version");
        app.store_composed("Sent");
        assert_eq!(app.mails.len(), count + 1);
        assert_eq!(app.mails.last().unwrap().folder, "Sent");
        app.begin_compose();
        assert!(app.cc.is_empty() && app.bcc.is_empty() && !app.show_bcc);
    }
    #[test]
    fn composer_shortcuts_save_locally_and_discard_saved_draft() {
        let ctx = test_context();
        ctx.set_fonts(app_fonts());
        let mut app = mailbox();
        app.account = None;
        app.begin_compose();
        let count = app.mails.len();
        let mut submit = key(Key::Enter);
        if let Event::Key { modifiers, .. } = &mut submit {
            *modifiers = Modifiers::COMMAND;
        }
        let _ = ctx.run(input(vec![submit.clone()]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        assert!(app.compose);
        assert_eq!(app.mails.len(), count);
        app.to = "alex@example.com".into();
        app.body = "Local message".into();
        // A second deliberate press includes release; holding Send must not resend.
        let mut release = submit.clone();
        if let Event::Key { pressed, .. } = &mut release {
            *pressed = false;
        }
        let _ = ctx.run(input(vec![release, submit]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        assert!(!app.compose);
        assert_eq!(app.mails.last().unwrap().folder, "Sent");
        app.begin_compose();
        app.body = "Draft to discard".into();
        app.save_composed("Drafts");
        let mut discard = key(Key::Backspace);
        if let Event::Key { modifiers, .. } = &mut discard {
            *modifiers = Modifiers::COMMAND | Modifiers::SHIFT;
        }
        let _ = ctx.run(input(vec![discard]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        assert!(!app.compose && app.body.is_empty());
        assert_eq!(app.mails.len(), count + 1);
        assert!(app.editing_draft.is_none());
    }
    #[test]
    fn composing_focuses_body_and_reply_is_inline() {
        let ctx = test_context();
        ctx.set_fonts(app_fonts());
        let mut app = mailbox();
        app.begin_compose();
        assert!(app.writer_focus && !app.inline_reply);
        let _ = ctx.run(input(vec![]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        let _ = ctx.run(input(vec![Event::Text("Hello".into())]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        assert_eq!(app.body, "Hello");
        let mut italic = key(Key::I);
        if let Event::Key { modifiers, .. } = &mut italic {
            *modifiers = Modifiers::COMMAND;
        }
        let _ = ctx.run(input(vec![italic, Event::Text(" there".into())]), |ctx| {
            CentralPanel::default().show(ctx, |ui| app.writer(ui, 200.));
        });
        assert_eq!(app.body, "Hello there");
        assert!(app.editor.marks[5..].iter().all(|mark| *mark == 2));
        assert!(app.to.is_empty());
        app.compose = false;
        app.action("Reply");
        assert!(app.compose && app.inline_reply && app.writer_focus);
        app.body = "Formatted draft".into();
        app.editor.marks = vec![2; app.body.chars().count()];
        app.store_composed("Drafts");
        assert_eq!(app.mails.last().unwrap().marks, vec![2; 15]);
    }
    #[test]
    fn reply_selects_its_account_among_multiple_providers() {
        let mut app = mailbox();
        let gmail = sync::Account::new(
            "gmail".into(),
            "me@gmail.com".into(),
            "token".into(),
            String::new(),
            String::new(),
        )
        .unwrap();
        let outlook = sync::Account::new(
            "graph".into(),
            "me@outlook.com".into(),
            "token".into(),
            String::new(),
            String::new(),
        )
        .unwrap();
        app.mails[0].account = outlook.scope();
        app.account = Some(gmail.clone());
        app.accounts = vec![gmail, outlook.clone()];
        app.action("Reply");
        assert_eq!(app.account.as_ref().unwrap().scope(), outlook.scope());
        assert_eq!(app.to, app.mails[0].from);
    }
    #[test]
    fn account_filter_combines_with_search_categories_and_folders() {
        let mut app = mailbox();
        app.mails[0].account = "gmail:one@example.com".into();
        app.mails[1].account = "gmail:two@example.com".into();
        app.mails[3].account = "gmail:one@example.com".into();
        app.account_filter = Some("gmail:one@example.com".into());
        assert_eq!(app.visible(), vec![0, 3]);
        app.search = "coffee".into();
        assert_eq!(app.visible(), vec![0, 3]);
        app.category = "Newsletters".into();
        assert!(app.visible().is_empty());
        app.category = "All".into();
        app.search.clear();
        app.folder = "Starred".into();
        assert!(app.visible().is_empty());
        app.account_filter = Some("gmail:two@example.com".into());
        assert_eq!(app.visible(), vec![1]);
        app.folder = "Inbox".into();
        app.account_filter = None;
        assert_eq!(app.visible().len(), 8);
        app.account_filter = Some("gmail:empty@example.com".into());
        assert!(app.visible().is_empty());
    }
    #[test]
    fn composing_uses_filtered_account() {
        let mut app = mailbox();
        let first = sync::Account::new("gmail".into(), "one@example.com".into(),
            "token".into(), String::new(), String::new()).unwrap();
        let second = sync::Account::new("gmail".into(), "two@example.com".into(),
            "token".into(), String::new(), String::new()).unwrap();
        app.accounts = vec![first.clone(), second.clone()];
        app.account = Some(first);
        app.account_filter = Some(second.scope());
        app.begin_compose();
        assert_eq!(app.account.as_ref().unwrap().scope(), second.scope());
        assert_eq!(app.account_filter, Some(second.scope()));
    }
    #[test]
    fn search_combines_category_and_folder_filters() {
        let mut app = mailbox();
        assert_eq!(app.visible().len(), 8);
        app.category = "People".into();
        assert_eq!(app.visible().len(), 5);
        app.search = "COFFEE".into();
        assert_eq!(app.visible().len(), 2);
        app.folder = "Archive".into();
        assert!(app.visible().is_empty());
        app.folder = "Starred".into();
        app.category = "All".into();
        app.search.clear();
        assert_eq!(app.visible(), vec![1]);
    }
    #[test]
    fn local_reply_retains_reference_and_joins_parent_conversation() {
        let mut app = mailbox();
        app.mails[0].message_id = "<parent@local>".into();
        app.action("Reply");
        app.body = "Thanks!".into();
        app.store_composed("Sent");
        let reply = app.mails.last().unwrap();
        assert_eq!(reply.references, vec!["<parent@local>"]);
        assert!(!reply.message_id.is_empty());
        let groups = threading::threads(&app.mails);
        assert_eq!(groups[0], *groups.last().unwrap());
    }
    #[test]
    fn mail_state_round_trips_without_losing_flags_or_snooze() {
        let mut mails = sample();
        mails[0].folder = "Snoozed".into();
        mails[0].snoozed_until = Some(1900000000);
        mails[0].star = true;
        let encoded = serde_json::to_string(&Stored { mails }).unwrap();
        let decoded: Stored = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.mails[0].snoozed_until, Some(1900000000));
        assert!(decoded.mails[0].star);
        assert_eq!(decoded.mails[0].folder, "Snoozed");
        assert_eq!(decoded.mails.len(), 8);
    }
}
