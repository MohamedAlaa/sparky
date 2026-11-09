//! A native, selection-based rich text editor shared by compose and inline replies.
use eframe::egui::{self, *};
#[derive(Default)]
pub struct Editor {
    pub marks: Vec<u8>,
    selection: std::ops::Range<usize>,
    typing: u8,
}
impl Editor {
    pub fn reset(&mut self, marks: Vec<u8>) {
        *self = Self {
            marks,
            ..Default::default()
        };
    }
    pub fn show(&mut self, ui: &mut Ui, text: &mut String, focus: &mut bool, height: f32) {
        let id = ui.make_persistent_id("friendly-writer");
        let mut restore = false;
        let mut insert_cursor = None;
        if ui.memory(|m| m.has_focus(id)) {
            for (name, bit) in [
                ("Bold", 1),
                ("Italic", 2),
                ("Underline", 4),
                ("Heading", 8),
                ("Code", 16),
            ] {
                if super::shortcuts::consume(ui.ctx(), name) {
                    self.toggle(bit);
                }
            }
        }
        super::paper_frame().inner_margin(0.).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            Frame::new()
                .fill(Color32::from_rgb(248, 249, 252))
                .corner_radius(CornerRadius {
                    nw: super::CORNER_RADIUS,
                    ne: super::CORNER_RADIUS,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(8.)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.;
                        ui.spacing_mut().button_padding = vec2(10., 6.);
                        ui.spacing_mut().interact_size.y = 30.;
                        for (label, tip, bit) in [
                            ("B", "Bold", 1),
                            ("I", "Italic", 2),
                            ("U", "Underline", 4),
                            ("H", "Heading", 8),
                            ("‹/›", "Code", 16),
                        ] {
                            let title = match bit {
                                1 => RichText::new(label).strong(),
                                2 => RichText::new(label).italics(),
                                4 => RichText::new(label).underline(),
                                _ => RichText::new(label),
                            };
                            if ui
                                .add(
                                    Button::new(title)
                                        .selected(self.typing & bit != 0)
                                        .frame(self.typing & bit != 0)
                                        .min_size(vec2(30., 30.)),
                                )
                                .on_hover_text(format!("{} · {}", tip, super::shortcuts::hint(tip)))
                                .clicked()
                            {
                                self.toggle(bit);
                                restore = true;
                            }
                        }
                        ui.separator();
                        if block_button(ui, "List", false).clicked() {
                            insert_cursor = Some(self.toggle_lines(text, "• "));
                            restore = true;
                        }
                        if block_button(ui, "Quote", true).clicked() {
                            insert_cursor = Some(self.toggle_lines(text, "> "));
                            restore = true;
                        }
                        ui.add_space(8.);
                        ui.label(
                            RichText::new("Select text to format")
                                .small()
                                .color(super::MUTED),
                        );
                    });
                });
            Frame::new().inner_margin(16.).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let before = text.clone();
                let marks = self.marks.clone();
                let mut layouter = |ui: &Ui, value: &str, width: f32| {
                    ui.fonts(|f| f.layout_job(layout(value, &marks, width)))
                };
                let output = ScrollArea::vertical()
                    .id_salt(id)
                    .max_height(height)
                    .show(ui, |ui| {
                        TextEdit::multiline(text)
                            .id(id)
                            .hint_text("Start writing…")
                            .font(FontId::proportional(16.))
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .min_size(vec2(ui.available_width(), height))
                            .layouter(&mut layouter)
                            .show(ui)
                    })
                    .inner;
                if *focus {
                    output.response.request_focus();
                    let mut state = output.state.clone();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(0),
                        )));
                    state.store(ui.ctx(), id);
                    *focus = false;
                } else if restore {
                    output.response.request_focus();
                    if let Some(at) = insert_cursor {
                        let mut state = output.state.clone();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(at),
                            )));
                        state.store(ui.ctx(), id);
                    }
                }
                if output.response.changed() {
                    self.reconcile(&before, text);
                }
                if let Some(range) = output.cursor_range {
                    let [a, b] = range.sorted_cursors();
                    self.selection = a.ccursor.index..b.ccursor.index;
                    if output.response.has_focus() && a.ccursor.index != b.ccursor.index {
                        self.typing = self.marks.get(a.ccursor.index).copied().unwrap_or_default();
                    }
                }
            });
        });
    }
    fn toggle(&mut self, bit: u8) {
        self.typing ^= bit;
        for mark in self
            .marks
            .get_mut(self.selection.clone())
            .into_iter()
            .flatten()
        {
            if self.typing & bit != 0 {
                *mark |= bit;
            } else {
                *mark &= !bit;
            }
        }
    }
    fn toggle_lines(&mut self, text: &mut String, prefix: &str) -> usize {
        let chars: Vec<_> = text.chars().collect();
        let start = self.selection.start.min(chars.len());
        let end = self.selection.end.min(chars.len());
        let first = chars[..start]
            .iter()
            .rposition(|c| *c == '\n')
            .map_or(0, |i| i + 1);
        // A selection ending at the next line's start excludes that line.
        let last = if end > start && chars.get(end - 1) == Some(&'\n') {
            end - 1
        } else {
            end
        };
        let mut starts = vec![first];
        starts.extend((first..last).filter(|&i| chars[i] == '\n').map(|i| i + 1));
        let prefix_chars: Vec<_> = prefix.chars().collect();
        let remove = starts
            .iter()
            .all(|&i| chars[i..].starts_with(&prefix_chars));
        self.marks.resize(chars.len(), 0);
        let mut cursor = end;
        for at in starts.into_iter().rev() {
            let byte = text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
            if remove {
                let bytes = text[byte..]
                    .chars()
                    .take(prefix_chars.len())
                    .map(char::len_utf8)
                    .sum::<usize>();
                text.replace_range(byte..byte + bytes, "");
                self.marks.drain(at..at + prefix_chars.len());
                cursor -= prefix_chars.len().min(cursor.saturating_sub(at));
            } else {
                text.insert_str(byte, prefix);
                self.marks
                    .splice(at..at, std::iter::repeat(0).take(prefix_chars.len()));
                cursor += prefix_chars.len();
            }
        }
        self.selection = cursor..cursor;
        cursor
    }
    fn reconcile(&mut self, before: &str, after: &str) {
        let old: Vec<_> = before.chars().collect();
        let new: Vec<_> = after.chars().collect();
        self.marks.resize(old.len(), 0);
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        self.marks.splice(
            prefix..old.len() - suffix,
            std::iter::repeat(self.typing).take(new.len() - prefix - suffix),
        );
    }
}
// Draw the block icons directly so font fallback cannot crowd the labels.
fn block_button(ui: &mut Ui, label: &str, quote: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(if quote { 88. } else { 76. }, 30.), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4., Color32::from_rgb(236, 238, 245));
    }
    let icon = rect.left_center() + vec2(10., 0.);
    let stroke = Stroke::new(1.4, super::INK);
    if quote {
        ui.painter().line_segment(
            [icon + vec2(0., -7.), icon + vec2(0., 7.)],
            Stroke::new(2.5, super::MUTED),
        );
    }
    for y in [-5., 0., 5.] {
        if !quote {
            ui.painter()
                .circle_filled(icon + vec2(0., y), 1.4, super::INK);
        }
        ui.painter()
            .line_segment([icon + vec2(5., y), icon + vec2(15., y)], stroke);
    }
    ui.painter().text(
        rect.left_center() + vec2(32., 0.),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.),
        super::INK,
    );
    response.on_hover_text(if quote {
        "Toggle quote on selected lines"
    } else {
        "Toggle list on selected lines"
    })
}
pub fn layout(text: &str, marks: &[u8], width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    for (i, c) in text.chars().enumerate() {
        let mark = marks.get(i).copied().unwrap_or_default();
        let mut format = TextFormat {
            font_id: FontId::new(
                if mark & 8 != 0 { 23. } else { 16. },
                if mark & 16 != 0 {
                    FontFamily::Monospace
                } else if mark & 1 != 0 {
                    FontFamily::Name("writer-bold".into())
                } else {
                    FontFamily::Proportional
                },
            ),
            color: super::INK,
            italics: mark & 2 != 0,
            ..Default::default()
        };
        if mark & 4 != 0 {
            format.underline = Stroke::new(1., super::INK);
        }
        job.append(&c.to_string(), 0., format);
    }
    job
}
pub fn html(text: &str, marks: &[u8]) -> String {
    let mut html = String::from("<div style=\"white-space:pre-wrap\">");
    for (i, c) in text.chars().enumerate() {
        let mark = marks.get(i).copied().unwrap_or_default();
        for (bit, tag) in [(1, "b"), (2, "i"), (4, "u"), (8, "big"), (16, "code")] {
            if mark & bit != 0 {
                html.push_str(&format!("<{tag}>"));
            }
        }
        html.push_str(&match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\n' => "<br>".into(),
            _ => c.to_string(),
        });
        for (bit, tag) in [(16, "code"), (8, "big"), (4, "u"), (2, "i"), (1, "b")] {
            if mark & bit != 0 {
                html.push_str(&format!("</{tag}>"));
            }
        }
    }
    html.push_str("</div>");
    html
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn block_controls_toggle_selected_lines_and_keep_unicode_formatting() {
        let mut e = Editor::default();
        let mut text = "héllo\nworld\nlast".to_owned();
        e.marks = vec![1; text.chars().count()];
        e.selection = 2..12;
        e.toggle_lines(&mut text, "• ");
        assert_eq!(text, "• héllo\n• world\nlast");
        assert_eq!(e.marks.len(), text.chars().count());
        assert_eq!(&e.marks[..3], &[0, 0, 1]);
        e.selection = 0..16;
        e.toggle_lines(&mut text, "• ");
        assert_eq!(text, "héllo\nworld\nlast");
        e.selection = 8..8;
        e.toggle_lines(&mut text, "> ");
        assert_eq!(text, "héllo\n> world\nlast");
    }
    #[test]
    fn unicode_edits_preserve_formats_and_html_escapes_content() {
        let mut e = Editor::default();
        e.marks = vec![1, 1, 2];
        e.reconcile("hé!", "héllo!");
        assert_eq!(e.marks, vec![1, 1, 0, 0, 0, 2]);
        assert_eq!(
            html("<&", &[1, 0]),
            "<div style=\"white-space:pre-wrap\"><b>&lt;</b>&amp;</div>"
        );
    }
}
