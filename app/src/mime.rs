//! Attachments: a small RFC 2045/2047/2231 MIME reader for IMAP mail, the
//! multipart/mixed writer used when sending, and the on-disk attachment cache.
//! Attachment bytes never enter the mailbox database.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A file attached to a message. Bytes live in the cache, fetched on demand.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime: String,
    pub size: u64,
    /// Gmail attachmentId or Graph attachment id; empty when bytes are cached at sync.
    #[serde(default)]
    pub remote_id: String,
    /// File name inside the attachment cache, once downloaded.
    #[serde(default)]
    pub cached: Option<String>,
}

/// A parsed message: its plain-text body and the files it carries.
#[derive(Default)]
pub struct Parsed {
    pub text: String,
    pub files: Vec<(Attachment, Vec<u8>)>,
}

/// Unfolded header value for `key`, or "".
pub fn header(headers: &str, key: &str) -> String {
    unfold(headers)
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim().to_owned())
        .unwrap_or_default()
}
fn unfold(headers: &str) -> String {
    headers.replace("\r\n", "\n").replace("\n\t", " ").replace("\n ", " ")
}

/// A `key=value` parameter from a structured header such as Content-Type,
/// joining RFC 2231 continuations (`name*0*=`) and decoding `name*=utf-8''…`.
pub fn param(value: &str, key: &str) -> Option<String> {
    let mut plain = None;
    let mut pieces: Vec<(usize, String, bool)> = Vec::new();
    for item in split_params(value).into_iter().skip(1) {
        let Some((k, v)) = item.split_once('=') else { continue };
        let k = k.trim().to_ascii_lowercase();
        let v = v.trim().trim_matches('"').to_owned();
        let key = key.to_ascii_lowercase();
        if k == key {
            plain = Some(v);
        } else if let Some(rest) = k.strip_prefix(&format!("{key}*")) {
            let encoded = rest.ends_with('*') || rest.is_empty();
            let index = rest.trim_end_matches('*').parse().unwrap_or(0);
            pieces.push((index, v, encoded));
        }
    }
    if pieces.is_empty() {
        return plain.map(|v| decode_words(&v));
    }
    pieces.sort_by_key(|p| p.0);
    let mut bytes = Vec::new();
    let mut charset_utf8 = true;
    for (i, (_, value, encoded)) in pieces.iter().enumerate() {
        let mut value = value.as_str();
        if i == 0 && *encoded {
            // charset'language'percent-encoded
            let mut parts = value.splitn(3, '\'');
            let charset = parts.next().unwrap_or_default();
            charset_utf8 = charset.is_empty() || charset.eq_ignore_ascii_case("utf-8");
            parts.next();
            value = parts.next().unwrap_or(value);
        }
        if *encoded { bytes.extend(percent_decode(value)); } else { bytes.extend(value.as_bytes()); }
    }
    Some(if charset_utf8 { String::from_utf8_lossy(&bytes).into_owned() } else { bytes.iter().map(|&b| b as char).collect() })
}
fn split_params(value: &str) -> Vec<String> {
    let (mut out, mut current, mut quoted) = (Vec::new(), String::new(), false);
    for c in value.chars() {
        match c {
            '"' => { quoted = !quoted; current.push(c) }
            ';' if !quoted => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}
fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(Ok(b)) = text.get(i + 1..i + 3).map(|hex| u8::from_str_radix(hex, 16)) { out.push(b); i += 3; continue; }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Decodes RFC 2047 encoded words (`=?utf-8?B?…?=`, `=?iso-8859-1?Q?…?=`).
pub fn decode_words(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    let mut last_was_word = false;
    while let Some(start) = rest.find("=?") {
        let (before, after) = rest.split_at(start);
        let Some(word) = encoded_word(after) else {
            out.push_str(before);
            out.push_str("=?");
            rest = &after[2..];
            last_was_word = false;
            continue;
        };
        // Whitespace between adjacent encoded words is dropped.
        if !(last_was_word && before.trim().is_empty()) { out.push_str(before); }
        out.push_str(&word.0);
        rest = &after[word.1..];
        last_was_word = true;
    }
    out.push_str(rest);
    out
}
fn encoded_word(text: &str) -> Option<(String, usize)> {
    let mut parts = text[2..].splitn(3, '?');
    let charset = parts.next()?;
    let encoding = parts.next()?;
    let tail = parts.next()?;
    let end = tail.find("?=")?;
    let payload = &tail[..end];
    let consumed = 2 + charset.len() + 1 + encoding.len() + 1 + end + 2;
    let bytes = match encoding.to_ascii_uppercase().as_str() {
        "B" => STANDARD.decode(payload).ok()?,
        "Q" => quoted_printable(&payload.replace('_', " ")),
        _ => return None,
    };
    let utf8 = charset.split('*').next().unwrap_or_default().eq_ignore_ascii_case("utf-8");
    Some((if utf8 { String::from_utf8_lossy(&bytes).into_owned() } else { bytes.iter().map(|&b| b as char).collect() }, consumed))
}
fn quoted_printable(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            if bytes[i + 1..].starts_with(b"\r\n") { i += 3; continue; }
            if bytes[i + 1..].starts_with(b"\n") { i += 2; continue; }
            if let Some(hex) = text.get(i + 1..i + 3) {
                if let Ok(b) = u8::from_str_radix(hex, 16) { out.push(b); i += 3; continue; }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}
fn decode_body(headers: &str, body: &[u8]) -> Vec<u8> {
    let encoding = header(headers, "Content-Transfer-Encoding").to_ascii_lowercase();
    match encoding.as_str() {
        "base64" => {
            let compact: Vec<u8> = body.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
            STANDARD.decode(&compact).unwrap_or_else(|_| body.to_vec())
        }
        "quoted-printable" => quoted_printable(&String::from_utf8_lossy(body)),
        _ => body.to_vec(),
    }
}
fn split_head(raw: &[u8]) -> (String, &[u8]) {
    for (sep, len) in [(&b"\r\n\r\n"[..], 4), (&b"\n\n"[..], 2)] {
        if let Some(i) = raw.windows(len).position(|w| w == sep) {
            return (String::from_utf8_lossy(&raw[..i]).into_owned(), &raw[i + len..]);
        }
    }
    (String::from_utf8_lossy(raw).into_owned(), &[])
}

/// Parses a full RFC 822 message (headers + body).
pub fn parse(raw: &[u8]) -> (String, Parsed) {
    let (headers, body) = split_head(raw);
    let mut parsed = Parsed::default();
    let mut html = String::new();
    walk(&headers, body, &mut parsed, &mut html, 0);
    if parsed.text.trim().is_empty() && !html.is_empty() {
        parsed.text = strip_html(&html);
    }
    (headers, parsed)
}
fn walk(headers: &str, body: &[u8], parsed: &mut Parsed, html: &mut String, depth: usize) {
    let content_type = header(headers, "Content-Type");
    let kind = content_type.split(';').next().unwrap_or("text/plain").trim().to_ascii_lowercase();
    let kind = if kind.is_empty() { "text/plain".to_owned() } else { kind };
    if kind.starts_with("multipart/") && depth < 12 {
        if let Some(boundary) = param(&content_type, "boundary") {
            for part in split_parts(body, &boundary) {
                let (h, b) = split_head(part);
                walk(&h, b, parsed, html, depth + 1);
            }
            return;
        }
    }
    let disposition = header(headers, "Content-Disposition");
    let name = param(&disposition, "filename").or_else(|| param(&content_type, "name")).map(|n| sanitize(&n));
    let attachment = disposition.trim_start().to_ascii_lowercase().starts_with("attachment")
        || (name.is_some() && !kind.starts_with("text/"));
    let data = decode_body(headers, body);
    if attachment || (name.is_some() && parsed.text.is_empty() && kind != "text/plain" && kind != "text/html") {
        let name = name.unwrap_or_else(|| "attachment".into());
        parsed.files.push((Attachment { mime: kind, size: data.len() as u64, name, ..Default::default() }, data));
    } else if kind == "text/plain" && parsed.text.is_empty() {
        parsed.text = String::from_utf8_lossy(&data).into_owned();
    } else if kind == "text/html" && html.is_empty() {
        *html = String::from_utf8_lossy(&data).into_owned();
    } else if let Some(name) = name {
        parsed.files.push((Attachment { mime: kind, size: data.len() as u64, name, ..Default::default() }, data));
    }
}
fn split_parts<'a>(body: &'a [u8], boundary: &str) -> Vec<&'a [u8]> {
    let marker = format!("--{boundary}");
    let marker = marker.as_bytes();
    let mut starts = Vec::new();
    let mut i = 0;
    while i + marker.len() <= body.len() {
        let at_line_start = i == 0 || body[i - 1] == b'\n';
        if at_line_start && &body[i..i + marker.len()] == marker {
            starts.push(i);
            i += marker.len();
        } else {
            i += 1;
        }
    }
    let mut parts = Vec::new();
    for pair in starts.windows(2) {
        let begin = pair[0] + marker.len();
        if body[begin..].starts_with(b"--") { break; }
        let mut part = &body[begin..pair[1]];
        // Drop the rest of the boundary line and the CRLF before the next boundary.
        if let Some(nl) = part.iter().position(|&b| b == b'\n') { part = &part[nl + 1..]; } else { continue; }
        let part = part.strip_suffix(b"\r\n").or_else(|| part.strip_suffix(b"\n")).unwrap_or(part);
        parts.push(part);
    }
    parts
}
fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").trim().to_owned()
}

/// A safe display/file name: no path separators, control characters or quotes.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = decode_words(name)
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | '"' | ':' | '*' | '?' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').trim();
    let cleaned: String = cleaned.chars().take(180).collect();
    if cleaned.is_empty() { "attachment".into() } else { cleaned }
}

/// MIME type from a file extension; octet-stream when unknown.
pub fn guess_mime(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "svg" => "image/svg+xml",
        "txt" | "md" | "log" => "text/plain",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "key" => "application/x-iwork-keynote-sffkey",
        "pages" => "application/x-iwork-pages-sffpages",
        "numbers" => "application/x-iwork-numbers-sffnumbers",
        "ics" => "text/calendar",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

/// "1.2 MB" style sizes.
pub fn human_size(bytes: u64) -> String {
    match bytes {
        0..=999 => format!("{bytes} B"),
        1_000..=999_999 => format!("{:.0} KB", bytes as f64 / 1e3),
        1_000_000..=999_999_999 => format!("{:.1} MB", bytes as f64 / 1e6),
        _ => format!("{:.1} GB", bytes as f64 / 1e9),
    }
}

pub fn base64_lines(bytes: &[u8]) -> String {
    STANDARD
        .encode(bytes)
        .as_bytes()
        .chunks(76)
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}
fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}
/// A boundary string guaranteed not to occur in any of the given contents.
pub fn boundary(prefix: &str, contents: &[&[u8]]) -> String {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
    let mut n = seed ^ (std::process::id() as u128) << 64;
    loop {
        let candidate = format!("sparky-{prefix}-{n:032x}");
        if !contents.iter().any(|c| c.windows(candidate.len()).any(|w| w == candidate.as_bytes())) {
            return candidate;
        }
        n = n.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }
}
/// One `multipart/mixed` body part for a file.
pub fn file_part(boundary: &str, name: &str, mime: &str, bytes: &[u8]) -> String {
    let name = sanitize(name);
    let ascii: String = name.chars().map(|c| if c.is_ascii() { c } else { '_' }).collect();
    format!(
        "--{boundary}\r\nContent-Type: {mime}; name=\"{ascii}\"\r\nContent-Disposition: attachment; filename=\"{ascii}\"; filename*=UTF-8''{}\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n",
        percent_encode(&name),
        base64_lines(bytes)
    )
}

// ---- Cache --------------------------------------------------------------

pub fn cache_dir() -> Result<PathBuf, String> {
    Ok(crate::data_directory()?.join("attachments"))
}
/// Writes bytes to the private cache and returns the cache file name.
pub fn store(name: &str, bytes: &[u8]) -> Result<String, String> {
    let dir = cache_dir()?;
    store_in(&dir, name, bytes)
}
pub fn store_in(dir: &Path, name: &str, bytes: &[u8]) -> Result<String, String> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    name.hash(&mut h);
    let file = format!("{:016x}-{}", h.finish(), sanitize(name));
    create_private_dir(dir)?;
    let path = dir.join(&file);
    if !path.exists() {
        let tmp = dir.join(format!(".{file}.part"));
        write_private(&tmp, bytes)?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("Could not save the attachment: {e}"))?;
    }
    Ok(file)
}
pub fn cached_path(file: &str) -> Result<PathBuf, String> {
    // Cache names are generated by `store`; never follow anything path-like.
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err("Invalid attachment cache entry".into());
    }
    Ok(cache_dir()?.join(file))
}
fn create_private_dir(dir: &Path) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir).map_err(|e| format!("Could not create the attachment folder: {e}"))
}
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| format!("Could not save the attachment: {e}"))?;
    file.write_all(bytes).map_err(|e| format!("Could not save the attachment: {e}"))
}

/// The user's Downloads folder (XDG_DOWNLOAD_DIR on Linux when set).
pub fn downloads_dir() -> Result<PathBuf, String> {
    if cfg!(not(target_os = "macos")) {
        if let Some(dir) = std::env::var_os("XDG_DOWNLOAD_DIR").filter(|d| !d.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
    }
    let home = std::env::var_os("HOME").ok_or("Could not locate your home folder")?;
    Ok(PathBuf::from(home).join("Downloads"))
}
/// Copies `source` into `dir` as `name`, adding " (1)", " (2)"… instead of overwriting.
pub fn save_copy(source: &Path, dir: &Path, name: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not open {}: {e}", dir.display()))?;
    let name = sanitize(name);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_owned(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    for n in 0..1000 {
        let candidate = if n == 0 { dir.join(&name) } else { dir.join(format!("{stem} ({n}){ext}")) };
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        match options.open(&candidate) {
            Ok(mut out) => {
                let mut input = std::fs::File::open(source).map_err(|e| format!("Could not read the attachment: {e}"))?;
                std::io::copy(&mut input, &mut out).map_err(|e| format!("Could not save the attachment: {e}"))?;
                return Ok(candidate);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Could not save the attachment: {e}")),
        }
    }
    Err("Too many files with this name in Downloads".into())
}
/// Opens a file or URL with the desktop's default application.
pub fn open_path(target: &std::ffi::OsStr) -> Result<(), String> {
    let program = if cfg!(target_os = "macos") { "/usr/bin/open" } else { "xdg-open" };
    let status = std::process::Command::new(program)
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|_| "Could not open the file".to_owned())?;
    if status.success() { Ok(()) } else { Err("No application could open this file".into()) }
}

/// Native multi-file picker via the platform's own dialog tool.
/// Ok(empty) means cancelled; Err means no picker is available.
pub fn pick_files() -> Result<Vec<PathBuf>, String> {
    use std::process::Command;
    let output = if cfg!(target_os = "macos") {
        Command::new("/usr/bin/osascript")
            .args(["-e", "set fs to choose file with prompt \"Attach files\" with multiple selections allowed", "-e", "set out to \"\"", "-e", "repeat with f in fs", "-e", "set out to out & POSIX path of f & linefeed", "-e", "end repeat", "-e", "return out"])
            .output()
    } else {
        Command::new("zenity")
            .args(["--file-selection", "--multiple", "--separator=\n", "--title=Attach files"])
            .output()
            .or_else(|_| Command::new("kdialog").args(["--getopenfilename", "--multiple", "--separate-output", "."]).output())
    }
    .map_err(|_| "No file picker is available here. Drag files onto the writer instead (or install zenity).".to_owned())?;
    // Cancelling returns a non-zero status with no output.
    Ok(String::from_utf8_lossy(&output.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_message_yields_text_and_decoded_attachments() {
        let raw = "From: a@example.com\r\nSubject: Report\r\nContent-Type: multipart/mixed; boundary=\"outer\"\r\n\r\npreamble\r\n--outer\r\nContent-Type: multipart/alternative; boundary=inner\r\n\r\n--inner\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nCaf=C3=A9 is =\r\nopen\r\n--inner\r\nContent-Type: text/html\r\n\r\n<p>Caf\u{e9}</p>\r\n--inner--\r\n--outer\r\nContent-Type: application/pdf; name=\"=?utf-8?B?UmVwb3J0IMOpdMOpLnBkZg==?=\"\r\nContent-Disposition: attachment\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0x\r\nLjQK\r\n--outer\r\nContent-Type: text/plain\r\nContent-Disposition: attachment; filename*0*=utf-8''notes%20; filename*1=part.txt\r\n\r\nhello\r\n--outer--\r\n";
        let (headers, parsed) = parse(raw.as_bytes());
        assert_eq!(header(&headers, "subject"), "Report");
        assert_eq!(parsed.text, "Café is open");
        assert_eq!(parsed.files.len(), 2);
        assert_eq!(parsed.files[0].0.name, "Report été.pdf");
        assert_eq!(parsed.files[0].0.mime, "application/pdf");
        assert_eq!(parsed.files[0].1, b"%PDF-1.4\n");
        assert_eq!(parsed.files[0].0.size, 9);
        assert_eq!(parsed.files[1].0.name, "notes part.txt");
        assert_eq!(parsed.files[1].1, b"hello");
    }

    #[test]
    fn single_part_and_html_only_messages_have_text() {
        let (_, plain) = parse(b"Subject: x\n\nJust text\n");
        assert_eq!(plain.text, "Just text\n");
        assert!(plain.files.is_empty());
        let (_, html) = parse(b"Content-Type: text/html\n\n<p>Hi &amp; bye</p>");
        assert_eq!(html.text, "Hi & bye");
    }

    #[test]
    fn built_file_parts_round_trip_and_boundary_is_unique() {
        let data: Vec<u8> = (0..=255u8).cycle().take(5000).collect();
        let b = boundary("mixed", &[&data, b"body"]);
        assert!(!data.windows(b.len()).any(|w| w == b.as_bytes()));
        let raw = format!(
            "Content-Type: multipart/mixed; boundary=\"{b}\"\r\n\r\n--{b}\r\nContent-Type: text/plain\r\n\r\nbody\r\n{}--{b}--\r\n",
            file_part(&b, "Ünïcode \"q\".bin", "application/octet-stream", &data)
        );
        let (_, parsed) = parse(raw.as_bytes());
        assert_eq!(parsed.text, "body");
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].0.name, "Ünïcode _q_.bin");
        assert_eq!(parsed.files[0].1, data);
    }

    #[test]
    fn names_are_sanitized_and_cache_entries_cannot_escape() {
        assert_eq!(sanitize("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize("a\r\nb.txt"), "a__b.txt");
        assert_eq!(sanitize("   "), "attachment");
        assert!(cached_path("../x").is_err());
        assert!(cached_path(".hidden").is_err());
        assert_eq!(guess_mime("Photo.JPG"), "image/jpeg");
        assert_eq!(guess_mime("noext"), "application/octet-stream");
        assert_eq!(human_size(1_500_000), "1.5 MB");
    }

    #[test]
    fn saving_never_overwrites_existing_files() {
        let dir = std::env::temp_dir().join(format!("sparky-save-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = dir.join("cache");
        let file = store_in(&cache, "report.pdf", b"one").unwrap();
        assert_eq!(store_in(&cache, "report.pdf", b"one").unwrap(), file, "same content reuses the cache entry");
        let source = cache.join(&file);
        let out = dir.join("out");
        let first = save_copy(&source, &out, "report.pdf").unwrap();
        let second = save_copy(&source, &out, "report.pdf").unwrap();
        assert_eq!(first.file_name().unwrap(), "report.pdf");
        assert_eq!(second.file_name().unwrap(), "report (1).pdf");
        assert_eq!(std::fs::read(second).unwrap(), b"one");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
