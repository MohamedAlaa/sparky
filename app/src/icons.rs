//! Lucide icons (ISC, see assets/icons/LICENSE-lucide) and shared motion helpers.
//! The vendored SVGs use white strokes so `tint` can color them with any accent.
use eframe::egui::{
    include_image, pos2, vec2, Color32, Context, CursorIcon, Id, Image, ImageSource, Painter,
    Rect, Response, Sense, Ui,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Archive,
    ArchiveRestore,
    ArrowDown,
    ArrowUp,
    Check,
    ChevronDown,
    ChevronRight,
    Alert,
    Clock,
    Command,
    Enter,
    FilePen,
    Inbox,
    Info,
    Keyboard,
    Mail,
    MailOpen,
    Refresh,
    Reply,
    Search,
    Send,
    Settings,
    Compose,
    Star,
    StarFilled,
    Tag,
    Trash,
    UserPlus,
    X,
}

impl Icon {
    pub fn source(self) -> ImageSource<'static> {
        match self {
            Self::Archive => include_image!("../assets/icons/archive.svg"),
            Self::ArchiveRestore => include_image!("../assets/icons/archive-restore.svg"),
            Self::ArrowDown => include_image!("../assets/icons/arrow-down.svg"),
            Self::ArrowUp => include_image!("../assets/icons/arrow-up.svg"),
            Self::Check => include_image!("../assets/icons/check.svg"),
            Self::ChevronDown => include_image!("../assets/icons/chevron-down.svg"),
            Self::ChevronRight => include_image!("../assets/icons/chevron-right.svg"),
            Self::Alert => include_image!("../assets/icons/circle-alert.svg"),
            Self::Clock => include_image!("../assets/icons/clock.svg"),
            Self::Command => include_image!("../assets/icons/command.svg"),
            Self::Enter => include_image!("../assets/icons/corner-down-left.svg"),
            Self::FilePen => include_image!("../assets/icons/file-pen.svg"),
            Self::Inbox => include_image!("../assets/icons/inbox.svg"),
            Self::Info => include_image!("../assets/icons/info.svg"),
            Self::Keyboard => include_image!("../assets/icons/keyboard.svg"),
            Self::Mail => include_image!("../assets/icons/mail.svg"),
            Self::MailOpen => include_image!("../assets/icons/mail-open.svg"),
            Self::Refresh => include_image!("../assets/icons/refresh-cw.svg"),
            Self::Reply => include_image!("../assets/icons/reply.svg"),
            Self::Search => include_image!("../assets/icons/search.svg"),
            Self::Send => include_image!("../assets/icons/send.svg"),
            Self::Settings => include_image!("../assets/icons/settings.svg"),
            Self::Compose => include_image!("../assets/icons/square-pen.svg"),
            Self::Star => include_image!("../assets/icons/star.svg"),
            Self::StarFilled => include_image!("../assets/icons/star-filled.svg"),
            Self::Tag => include_image!("../assets/icons/tag.svg"),
            Self::Trash => include_image!("../assets/icons/trash-2.svg"),
            Self::UserPlus => include_image!("../assets/icons/user-plus.svg"),
            Self::X => include_image!("../assets/icons/x.svg"),
        }
    }

    /// Icon for a folder name or command label; `None` when nothing fits.
    pub fn for_name(name: &str) -> Option<Self> {
        Some(match name {
            "Inbox" | "Go to Inbox" => Self::Inbox,
            "Starred" | "Go to Starred" | "Star" => Self::Star,
            "Snoozed" | "Go to Snoozed" | "Snooze" => Self::Clock,
            "Sent" | "Go to Sent" | "Send / save locally" => Self::Send,
            "Drafts" | "Go to Drafts" | "Discard draft" => Self::FilePen,
            "Archive" | "Go to Archive" => Self::Archive,
            "Trash" | "Go to Trash" | "Delete" => Self::Trash,
            "Move to Inbox" => Self::ArchiveRestore,
            "Compose" => Self::Compose,
            "Reply" => Self::Reply,
            "Unread" => Self::MailOpen,
            "Search mail" => Self::Search,
            "Sync all accounts" => Self::Refresh,
            "Settings & shortcuts" => Self::Settings,
            "Keyboard shortcuts" => Self::Keyboard,
            "Commands" => Self::Command,
            "Connect email account" => Self::UserPlus,
            "Open conversation / draft" => Self::Enter,
            "Next conversation" => Self::ArrowDown,
            "Previous conversation" => Self::ArrowUp,
            "Close / save draft" => Self::X,
            n if n.starts_with("Show ") => Self::Tag,
            _ => return None,
        })
    }
}

/// Paints an icon into `rect` through any (possibly clipped) painter.
pub fn paint(p: &Painter, ctx: &Context, icon: Icon, rect: Rect, tint: Color32) {
    let image = Image::new(icon.source()).fit_to_exact_size(rect.size());
    match image.load_for_size(ctx, rect.size()) {
        Ok(eframe::egui::load::TexturePoll::Ready { texture }) => {
            p.image(
                texture.id,
                rect,
                Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                tint,
            );
        }
        // SVGs rasterize asynchronously on first use.
        _ => ctx.request_repaint(),
    }
}

/// A square ghost button holding one icon, with an animated hover fill.
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tint: Color32, hover: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let t = ease(ui.ctx().animate_bool_with_time(response.id, response.hovered(), DUR_FAST));
    if t > 0. {
        ui.painter().rect_filled(rect, 6., hover.gamma_multiply(t));
    }
    let glyph = Rect::from_center_size(rect.center(), vec2(size * 0.5, size * 0.5));
    paint(ui.painter(), ui.ctx(), icon, glyph, tint);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub const DUR_FAST: f32 = 0.12;
pub const DUR_MED: f32 = 0.22;

/// Ease-out cubic: quick start, gentle landing.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    1. - (1. - t).powi(3)
}

/// An eased 0→1 transition driven by `on`.
pub fn fade(ctx: &Context, id: Id, on: bool, duration: f32) -> f32 {
    ease(ctx.animate_bool_with_time(id, on, duration))
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_is_bounded_and_monotonic() {
        assert_eq!(ease(0.), 0.);
        assert_eq!(ease(1.), 1.);
        assert_eq!(ease(2.), 1.);
        let samples: Vec<f32> = (0..=10).map(|i| ease(i as f32 / 10.)).collect();
        assert!(samples.windows(2).all(|w| w[0] <= w[1]));
    }
    #[test]
    fn every_folder_has_an_icon() {
        for name in ["Inbox", "Starred", "Snoozed", "Sent", "Drafts", "Archive", "Trash"] {
            assert!(Icon::for_name(name).is_some(), "{name}");
        }
    }
}
