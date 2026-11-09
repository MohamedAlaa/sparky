//! The shared catalog for dispatch, command search, key hints and shortcut help.
use eframe::egui::{Context, Event, Key, Modifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Global,
    Mailbox,
    Message,
    Writer,
    Editor,
}

pub struct Command {
    pub name: &'static str,
    pub group: &'static str,
    pub scope: Scope,
    pub key: Key,
    pub modifiers: Modifiers,
    pub chord: bool,
    pub hint: &'static str,
}
const SHIFT: Modifiers = Modifiers::SHIFT;
const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers { shift: true, ..CMD };
macro_rules! commands {
    ($(($name:literal, $group:literal, $scope:ident, $key:ident, $mods:expr, $chord:literal, $hint:literal)),* $(,)?) => {
        pub static COMMANDS: &[Command] = &[$(Command { name: $name, group: $group, scope: Scope::$scope, key: Key::$key, modifiers: $mods, chord: $chord, hint: $hint }),*];
    }
}
commands! {
    ("Commands", "Essentials", Global, K, CMD, false, "⌘ K"),
    ("Commands", "Essentials", Global, P, CMD_SHIFT, false, "⌘ ⇧ P"),
    ("Keyboard shortcuts", "Essentials", Global, Slash, CMD, false, "⌘ /"),
    ("Keyboard shortcuts", "Essentials", Mailbox, Questionmark, SHIFT, false, "?"),
    ("Keyboard shortcuts", "Essentials", Mailbox, Slash, SHIFT, false, "?"),
    ("Settings & shortcuts", "Essentials", Global, Comma, CMD, false, "⌘ ,"),
    ("Close / save draft", "Essentials", Global, Escape, Modifiers::NONE, false, "Esc"),
    ("Compose", "Mail", Mailbox, C, Modifiers::NONE, false, "C"),
    ("Search mail", "Mail", Mailbox, Slash, Modifiers::NONE, false, "/"),
    ("Sync all accounts", "Mail", Mailbox, Y, CMD_SHIFT, false, "⌘ ⇧ Y"),
    ("Next conversation", "Navigation", Mailbox, J, Modifiers::NONE, false, "J"),
    ("Next conversation", "Navigation", Mailbox, ArrowDown, Modifiers::NONE, false, "↓"),
    ("Previous conversation", "Navigation", Mailbox, K, Modifiers::NONE, false, "K"),
    ("Previous conversation", "Navigation", Mailbox, ArrowUp, Modifiers::NONE, false, "↑"),
    ("Open conversation / draft", "Navigation", Message, Enter, Modifiers::NONE, false, "Enter"),
    ("Reply", "Message actions", Message, R, Modifiers::NONE, false, "R"),
    ("Archive", "Message actions", Message, E, Modifiers::NONE, false, "E"),
    ("Star", "Message actions", Message, S, Modifiers::NONE, false, "S"),
    ("Snooze", "Message actions", Message, H, Modifiers::NONE, false, "H"),
    ("Unread", "Message actions", Message, U, Modifiers::NONE, false, "U"),
    ("Delete", "Message actions", Message, Num3, SHIFT, false, "#"),
    ("Delete", "Message actions", Message, Delete, Modifiers::NONE, false, "Delete"),
    ("Move to Inbox", "Message actions", Message, I, SHIFT, false, "⇧ I"),
    ("Go to Inbox", "Go to · press G, then…", Mailbox, I, Modifiers::NONE, true, "G then I"),
    ("Go to Starred", "Go to · press G, then…", Mailbox, S, Modifiers::NONE, true, "G then S"),
    ("Go to Snoozed", "Go to · press G, then…", Mailbox, H, Modifiers::NONE, true, "G then H"),
    ("Go to Sent", "Go to · press G, then…", Mailbox, T, Modifiers::NONE, true, "G then T"),
    ("Go to Drafts", "Go to · press G, then…", Mailbox, D, Modifiers::NONE, true, "G then D"),
    ("Go to Archive", "Go to · press G, then…", Mailbox, A, Modifiers::NONE, true, "G then A"),
    ("Go to Trash", "Go to · press G, then…", Mailbox, X, Modifiers::NONE, true, "G then X"),
    ("Show All", "Categories", Mailbox, Num1, Modifiers::NONE, false, "1"),
    ("Show People", "Categories", Mailbox, Num2, Modifiers::NONE, false, "2"),
    ("Show Newsletters", "Categories", Mailbox, Num3, Modifiers::NONE, false, "3"),
    ("Show Notifications", "Categories", Mailbox, Num4, Modifiers::NONE, false, "4"),
    ("Send / save locally", "Writing", Writer, Enter, CMD, false, "⌘ Enter"),
    ("Discard draft", "Writing", Writer, Backspace, CMD_SHIFT, false, "⌘ ⇧ Backspace"),
    ("Bold", "Formatting", Editor, B, CMD, false, "⌘ B"),
    ("Italic", "Formatting", Editor, I, CMD, false, "⌘ I"),
    ("Underline", "Formatting", Editor, U, CMD, false, "⌘ U"),
    ("Heading", "Formatting", Editor, H, CMD_SHIFT, false, "⌘ ⇧ H"),
    ("Code", "Formatting", Editor, X, CMD_SHIFT, false, "⌘ ⇧ X"),
}

pub fn display(hint: &str) -> String {
    if cfg!(target_os = "macos") {
        hint.into()
    } else {
        hint.replace('⌘', "Ctrl").replace('⇧', "Shift")
    }
}
pub fn hint(name: &str) -> String {
    COMMANDS
        .iter()
        .find(|c| c.name == name)
        .map(|c| display(c.hint))
        .unwrap_or_default()
}
/// Every distinct binding for a command, already localized for display.
pub fn hints(name: &str) -> Vec<String> {
    let mut all: Vec<String> = Vec::new();
    for c in COMMANDS.iter().filter(|c| c.name == name) {
        let h = display(c.hint);
        if !all.contains(&h) {
            all.push(h);
        }
    }
    all
}
/// Splits a display hint into keycaps. A `None` entry is a sequence arrow ("G then I").
pub fn keycaps(hint: &str) -> Vec<Option<String>> {
    hint.split_whitespace()
        .map(|k| (k != "then").then(|| k.to_owned()))
        .collect()
}
/// Command bindings that an exact key press would trigger, for "Record keys".
pub fn bound_to(key: Key, modifiers: Modifiers) -> Vec<&'static str> {
    let mut names = Vec::new();
    for c in COMMANDS.iter().filter(|c| c.key == key && modifiers.matches_exact(c.modifiers)) {
        if !names.contains(&c.name) {
            names.push(c.name);
        }
    }
    names
}

/// VS Code-style fuzzy match: query characters in order, case-insensitive.
/// Returns a score (higher is better) and the matched char indices of `text`.
pub fn fuzzy(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    let query: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect();
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
    let word_start = |i: usize| i == 0 || !chars[i - 1].is_alphanumeric();
    // Greedy forward pass, preferring word starts when the next query char appears at one.
    let mut matched = Vec::with_capacity(query.len());
    let mut from = 0;
    for q in &query {
        let candidates: Vec<usize> = (from..lower.len()).filter(|&i| lower[i] == *q).collect();
        let pick = candidates
            .iter()
            .copied()
            .find(|&i| matched.last().is_some_and(|&l: &usize| i == l + 1))
            .or_else(|| candidates.iter().copied().find(|&i| word_start(i)))
            .or_else(|| candidates.first().copied())?;
        matched.push(pick);
        from = pick + 1;
    }
    let mut score = 0;
    for (n, &i) in matched.iter().enumerate() {
        score += 1;
        if word_start(i) {
            score += 8;
        }
        if n > 0 && i == matched[n - 1] + 1 {
            score += 5;
        }
    }
    if matched[0] == 0 {
        score += 6;
    }
    // Whole-word matches ("account" in "email account", not "accounts") rank higher.
    let last = *matched.last().unwrap_or(&0);
    if last + 1 == chars.len() || !chars[last + 1].is_alphanumeric() {
        score += 4;
    }
    score -= (chars.len() as i32 - query.len() as i32).max(0) / 4;
    Some((score, matched))
}

pub fn consume(ctx: &Context, name: &str) -> bool {
    COMMANDS
        .iter()
        .filter(|c| c.name == name && !c.chord)
        .any(|c| {
            let exact = ctx.input(|i| {
                i.events.iter().any(|e| {
                    matches!(e,
            Event::Key { key, modifiers, pressed: true, repeat: false, .. }
                if *key == c.key && modifiers.matches_exact(c.modifiers))
                })
            });
            exact && ctx.input_mut(|i| i.consume_key(c.modifiers, c.key))
        })
}

#[derive(Default)]
pub struct State {
    pub help: bool,
    pub query: String,
    pub focus_help: bool,
    pub pending_g: Option<f64>,
    /// The guide is waiting for a key press to look up ("Record keys").
    pub recording: bool,
    /// The recorded key press filtering the guide.
    pub recorded: Option<(Key, Modifiers)>,
    /// The guide's group chip filter; `None` shows every group.
    pub group: Option<&'static str>,
}
/// Short chip label for a catalog group.
pub fn group_label(group: &str) -> &str {
    group.split(" ·").next().unwrap_or(group)
}
/// Catalog groups in display order.
pub fn groups() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for c in COMMANDS {
        if !all.contains(&c.group) {
            all.push(c.group);
        }
    }
    all
}
/// Human-readable form of a recorded key press, e.g. "⌘ ⇧ P".
pub fn describe(key: Key, modifiers: Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.command || modifiers.mac_cmd || modifiers.ctrl {
        parts.push("⌘".to_owned());
    }
    if modifiers.alt {
        parts.push("⌥".to_owned());
    }
    if modifiers.shift {
        parts.push("⇧".to_owned());
    }
    parts.push(key.symbol_or_name().to_owned());
    display(&parts.join(" "))
}
impl State {
    /// Resolve one command per event. Letter bindings never run while typing.
    pub fn resolve(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
        now: f64,
        typing: bool,
        blocked: bool,
    ) -> Option<&'static Command> {
        if typing || blocked || self.pending_g.is_some_and(|t| now - t > 1.5) {
            self.pending_g = None;
        }
        if let Some(c) = COMMANDS.iter().find(|c| {
            c.scope == Scope::Global && c.key == key && modifiers.matches_exact(c.modifiers)
        }) {
            self.pending_g = None;
            return (!repeat).then_some(c);
        }
        if typing || blocked {
            return None;
        }
        if let Some(c) = COMMANDS.iter().find(|c| {
            c.chord
                && c.key == key
                && self.pending_g.is_some()
                && modifiers.matches_exact(c.modifiers)
        }) {
            self.pending_g = None;
            return (!repeat).then_some(c);
        }
        // An invalid second key cancels the chord without becoming a mail action.
        if self.pending_g.take().is_some() {
            return None;
        }
        if key == Key::G && modifiers.matches_exact(Modifiers::NONE) && !repeat {
            self.pending_g = Some(now);
            return None;
        }
        COMMANDS.iter().find(|c| {
            !c.chord
                && matches!(c.scope, Scope::Mailbox | Scope::Message)
                && c.key == key
                && modifiers.matches_exact(c.modifiers)
                && (!repeat || c.group == "Navigation")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typing_modifiers_chords_and_repeats_are_safe() {
        let mut s = State::default();
        assert!(s
            .resolve(Key::E, Modifiers::NONE, false, 0., true, false)
            .is_none());
        assert!(s.resolve(Key::E, CMD, false, 0., false, false).is_none());
        assert!(s
            .resolve(Key::E, Modifiers::NONE, true, 0., false, false)
            .is_none());
        assert!(s
            .resolve(Key::G, Modifiers::NONE, false, 1., false, false)
            .is_none());
        assert_eq!(
            s.resolve(Key::S, Modifiers::NONE, false, 1.1, false, false)
                .unwrap()
                .name,
            "Go to Starred"
        );
        s.resolve(Key::G, Modifiers::NONE, false, 2., false, false);
        assert!(s
            .resolve(Key::E, Modifiers::NONE, false, 2.1, false, false)
            .is_none());
        s.resolve(Key::G, Modifiers::NONE, false, 3., false, false);
        assert_eq!(
            s.resolve(Key::S, Modifiers::NONE, false, 5., false, false)
                .unwrap()
                .name,
            "Star"
        );
        assert!(s
            .resolve(Key::C, Modifiers::NONE, false, 6., false, true)
            .is_none());
        assert_eq!(
            s.resolve(Key::K, CMD, false, 6., true, true).unwrap().name,
            "Commands"
        );
    }
    #[test]
    fn fuzzy_prefers_word_starts_and_rejects_misses() {
        assert!(fuzzy("arch", "Archive").is_some());
        let (_, idx) = fuzzy("gti", "Go to Inbox").unwrap();
        assert_eq!(idx, vec![0, 3, 6]);
        assert!(fuzzy("snz", "Snooze").is_some());
        assert!(fuzzy("zzz", "Snooze").is_none());
        assert_eq!(fuzzy("", "Reply"), Some((0, vec![])));
        let star = fuzzy("st", "Star").unwrap().0;
        let sent = fuzzy("st", "Go to Sent").unwrap().0;
        assert!(star > sent, "a prefix match ranks first");
        let exact = fuzzy("account", "Connect email account").unwrap().0;
        let partial = fuzzy("account", "Sync all accounts").unwrap().0;
        assert!(exact > partial, "a whole word beats a longer word");
    }
    #[test]
    fn keycaps_split_sequences_and_record_lookup() {
        assert_eq!(keycaps("G then I"), vec![Some("G".into()), None, Some("I".into())]);
        assert_eq!(bound_to(Key::E, Modifiers::NONE), vec!["Archive"]);
        assert_eq!(hints("Next conversation").len(), 2);
    }
    #[test]
    fn catalog_has_no_conflicting_bindings() {
        for (i, a) in COMMANDS.iter().enumerate() {
            for b in &COMMANDS[i + 1..] {
                assert!(
                    !(a.key == b.key && a.modifiers == b.modifiers && a.chord == b.chord),
                    "{} conflicts with {}",
                    a.name,
                    b.name
                );
            }
        }
    }
}
