//! One background Rust sync core, with Gmail REST, Microsoft Graph and native IMAP/SMTP adapters.
//! Accounts come from the connection UI, Keychain, or environment.
use crate::Mail;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

/// Successful accounts are imported even when another account cannot be reached.
#[derive(Default)]
pub struct SyncBatch {
    pub mails: Vec<Mail>,
    pub failures: Vec<String>,
}
impl From<Vec<Mail>> for SyncBatch {
    fn from(mails: Vec<Mail>) -> Self {
        Self { mails, failures: Vec::new() }
    }
}
pub fn fetch_all(
    accounts: &[Account],
    mut fetch: impl FnMut(&Account) -> Result<Vec<Mail>, String>,
) -> SyncBatch {
    let mut batch = SyncBatch::default();
    for account in accounts {
        match fetch(account) {
            Ok(mails) => batch.mails.extend(mails),
            Err(error) => batch.failures.push(format!("{}: {error}", account.address)),
        }
    }
    batch
}

#[derive(Clone)]
pub struct Account {
    pub provider: String,
    pub address: String,
    profile_name: std::sync::Arc<std::sync::OnceLock<String>>,
    secret: String,
    oauth: Option<std::sync::Arc<std::sync::Mutex<crate::oauth::Session>>>,
    imap_host: String,
    smtp_host: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedAccount {
    provider: String,
    address: String,
    secret: String,
    imap_host: String,
    smtp_host: String,
    profile_name: String,
    oauth: Option<Value>,
}
pub fn encode_accounts(accounts: &[Account]) -> Result<Vec<u8>, String> {
    let saved = accounts.iter().map(|account| {
        Ok(SavedAccount {
            provider: account.provider.clone(), address: account.address.clone(),
            secret: account.secret.clone(), imap_host: account.imap_host.clone(),
            smtp_host: account.smtp_host.clone(),
            profile_name: account.profile_name.get().cloned().unwrap_or_default(),
            oauth: account.oauth.as_ref().map(|session| session.lock()
                .map(|session| session.saved()).map_err(|_| "Account session unavailable".to_string())).transpose()?,
        })
    }).collect::<Result<Vec<_>, String>>()?;
    serde_json::to_vec(&saved).map_err(|_| "Could not save accounts".into())
}
pub fn restore_accounts(data: &[u8]) -> Result<Vec<Account>, String> {
    let saved: Vec<SavedAccount> = serde_json::from_slice(data)
        .map_err(|_| "Saved accounts are invalid. Reconnect in Settings.")?;
    saved.into_iter().map(|saved| {
        let oauth = saved.oauth.map(crate::oauth::Session::restore).transpose()?;
        let mut account = Account::new(saved.provider, saved.address,
            oauth.as_ref().map(|session| session.token.clone()).unwrap_or(saved.secret),
            saved.imap_host, saved.smtp_host)?;
        if let Some(session) = oauth {
            account.secret.clear();
            account.oauth = Some(std::sync::Arc::new(std::sync::Mutex::new(session)));
        }
        if !saved.profile_name.is_empty() { let _ = account.profile_name.set(saved.profile_name); }
        Ok(account)
    }).collect()
}
fn recipients(value: &str) -> Result<Vec<String>, String> {
    value
        .split([',', ';'])
        .filter(|s| !s.trim().is_empty())
        .map(|part| {
            let address = part
                .rsplit_once('<')
                .map(|(_, rest)| rest.trim_end_matches('>').trim())
                .unwrap_or(part.trim());
            if !address.contains('@') || address.contains([' ', '<', '>', '\r', '\n']) {
                Err("Enter valid recipient email addresses, separated by commas.".into())
            } else {
                Ok(address.to_owned())
            }
        })
        .collect()
}
fn graph_recipients(addresses: &[String]) -> Value {
    Value::Array(
        addresses
            .iter()
            .map(|address| serde_json::json!({"emailAddress":{"address":address}}))
            .collect(),
    )
}
fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}
impl Account {
    pub fn configured() -> Option<Self> {
        let account = Self {
            provider: env("SPARKY_PROVIDER"),
            address: env("SPARKY_EMAIL"),
            secret: env("SPARKY_SECRET"),
            oauth: None,
            profile_name: Default::default(),
            imap_host: env("SPARKY_IMAP_HOST"),
            smtp_host: env("SPARKY_SMTP_HOST"),
        };
        if !matches!(account.provider.as_str(), "gmail" | "graph" | "imap")
            || account.address.is_empty()
            || account.secret.is_empty()
        {
            return None;
        }
        if account.provider == "imap" && !valid_host(&account.imap_host) {
            return None;
        }
        Some(account)
    }
    pub fn new(
        provider: String,
        address: String,
        secret: String,
        imap_host: String,
        smtp_host: String,
    ) -> Result<Self, String> {
        let address = address.trim().to_lowercase();
        if !matches!(provider.as_str(), "gmail" | "graph" | "imap") {
            return Err("Choose a supported provider.".into());
        }
        if !address.contains('@') || address.contains(['\r', '\n', ' ']) {
            return Err("Enter your email address.".into());
        }
        if secret.trim().is_empty() || secret.contains(['\r', '\n', '\0']) {
            return Err("Enter a valid access token or app password.".into());
        }
        if provider == "imap" && (!valid_host(&imap_host) || !valid_host(&smtp_host)) {
            return Err("Enter both IMAP and SMTP hosts, including their TLS ports.".into());
        }
        Ok(Self {
            provider,
            address,
            secret,
            oauth: None,
            profile_name: Default::default(),
            imap_host,
            smtp_host,
        })
    }
    pub fn google(session: crate::oauth::Session) -> Result<Self, String> {
        let token = session.token.clone();
        let profile: Value = serde_json::from_str(&transport(&[
            (
                "url",
                "https://gmail.googleapis.com/gmail/v1/users/me/profile".into(),
            ),
            ("header", format!("Authorization: Bearer {token}")),
        ])?)
        .map_err(|_| "Google returned an invalid profile")?;
        let address = profile["emailAddress"]
            .as_str()
            .ok_or("Google did not return an email address")?;
        let mut account = Self::new(
            "gmail".into(),
            address.into(),
            token,
            String::new(),
            String::new(),
        )?;
        account.secret.clear();
        account.oauth = Some(std::sync::Arc::new(std::sync::Mutex::new(session)));
        Ok(account)
    }
    /// Providers without profile access keep their address as the honest fallback.
    pub fn display_name(&self) -> &str {
        self.profile_name.get().filter(|name| !name.is_empty())
            .map(String::as_str).unwrap_or(&self.address)
    }
    pub fn initials(&self) -> String {
        self.display_name().split_whitespace().take(2)
            .filter_map(|part| part.chars().next()).collect::<String>().to_uppercase()
    }
    fn load_profile(&self) {
        self.profile_name.get_or_init(|| {
            let (url, field) = match self.provider.as_str() {
                "gmail" => ("https://openidconnect.googleapis.com/v1/userinfo", "name"),
                "graph" => ("https://graph.microsoft.com/v1.0/me?$select=displayName", "displayName"),
                _ => return String::new(),
            };
            // Missing optional profile permissions must never block mail access.
            self.get(url, &[]).ok()
                .and_then(|body| serde_json::from_str::<Value>(&body).ok())
                .map(|profile| profile[field].as_str().unwrap_or_default()
                    .split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default()
        });
    }
    pub fn scope(&self) -> String {
        format!("{}:{}", self.provider, self.address)
    }
    fn get(&self, url: &str, extra: &[(&str, String)]) -> Result<String, String> {
        let mut config = vec![("url", url.to_owned())];
        if self.provider == "imap" {
            config.push(("user", format!("{}:{}", self.address, self.secret)));
        } else {
            config.push((
                "header",
                format!(
                    "Authorization: Bearer {}",
                    match &self.oauth {
                        Some(session) => session
                            .lock()
                            .map_err(|_| "Google session unavailable")?
                            .access_token()?,
                        None => self.secret.clone(),
                    }
                ),
            ));
        }
        config.extend_from_slice(extra);
        transport(&config)
    }
    pub fn fetch(&self) -> Result<Vec<Mail>, String> {
        self.load_profile();
        let mut mails = Vec::new();
        match self.provider.as_str() {
            "gmail" => {
                let list: Value = serde_json::from_str(&self.get("https://gmail.googleapis.com/gmail/v1/users/me/messages?labelIds=INBOX&maxResults=50", &[])?).map_err(|e| e.to_string())?;
                for item in list["messages"].as_array().into_iter().flatten() {
                    let id = item["id"]
                        .as_str()
                        .ok_or("Gmail returned a message without an id")?;
                    if !id.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Err("Invalid Gmail id".into());
                    }
                    let value: Value = serde_json::from_str(&self.get(&format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}?format=full"), &[])?).map_err(|e| e.to_string())?;
                    let mut mail = parse_headers(
                        &gmail_headers(&value["payload"]),
                        gmail_body(&value["payload"]),
                    );
                    mail.remote_id = id.into();
                    mail.remote_thread = value["threadId"].as_str().unwrap_or_default().into();
                    let labels = value["labelIds"].as_array();
                    mail.unread = labels.is_some_and(|v| v.iter().any(|s| s == "UNREAD"));
                    mail.star = labels.is_some_and(|v| v.iter().any(|s| s == "STARRED"));
                    mails.push(mail);
                }
            }
            "graph" => {
                let value: Value = serde_json::from_str(&self.get("https://graph.microsoft.com/v1.0/me/mailFolders/inbox/messages?$top=50&$select=id,conversationId,internetMessageId,internetMessageHeaders,from,subject,body,receivedDateTime,isRead,flag", &[("header", "Prefer: outlook.body-content-type=\"text\"".into())])?).map_err(|e| e.to_string())?;
                for item in value["value"]
                    .as_array()
                    .ok_or("Graph did not return messages")?
                {
                    let mut headers = String::new();
                    for header in item["internetMessageHeaders"]
                        .as_array()
                        .into_iter()
                        .flatten()
                    {
                        headers.push_str(&format!(
                            "{}: {}\n",
                            header["name"].as_str().unwrap_or_default(),
                            header["value"].as_str().unwrap_or_default()
                        ));
                    }
                    let mut mail = parse_headers(
                        &headers,
                        item["body"]["content"].as_str().unwrap_or_default().into(),
                    );
                    mail.from = format!(
                        "{} <{}>",
                        item["from"]["emailAddress"]["name"].as_str().unwrap_or(""),
                        item["from"]["emailAddress"]["address"]
                            .as_str()
                            .unwrap_or("")
                    );
                    mail.subject = item["subject"].as_str().unwrap_or("(No subject)").into();
                    mail.remote_id = item["id"]
                        .as_str()
                        .ok_or("Graph message id missing")?
                        .into();
                    mail.remote_thread = item["conversationId"].as_str().unwrap_or_default().into();
                    mail.message_id = item["internetMessageId"]
                        .as_str()
                        .unwrap_or_default()
                        .into();
                    mail.time = crate::date_groups::normalize(item["receivedDateTime"].as_str().unwrap_or_default());
                    mail.unread = !item["isRead"].as_bool().unwrap_or(true);
                    mail.star = item["flag"]["flagStatus"] == "flagged";
                    mails.push(mail);
                }
            }
            "imap" => {
                let url = format!("imaps://{}/INBOX", self.imap_host);
                let search = self.get(&url, &[("request", "UID SEARCH ALL".into())])?;
                let ids: Vec<_> = search
                    .lines()
                    .find_map(|line| line.strip_prefix("* SEARCH "))
                    .unwrap_or_default()
                    .split_whitespace()
                    .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
                    .collect();
                for id in ids.iter().rev().take(50) {
                    let raw = self.get(&format!("{url}/;UID={id}"), &[])?;
                    let normalized = raw.replace("\r\n", "\n");
                    let (headers, body) =
                        normalized.split_once("\n\n").unwrap_or((&normalized, ""));
                    let mut mail = parse_headers(headers, body.into());
                    // RFC Message-ID survives UIDVALIDITY changes. Do not deduplicate by bare UID.
                    mail.remote_id = if mail.message_id.is_empty() {
                        format!("imap-{}", hash(raw.as_bytes()))
                    } else {
                        mail.message_id.clone()
                    };
                    mails.push(mail);
                }
            }
            _ => return Err("Unsupported provider".into()),
        }
        for mail in &mut mails {
            mail.account = self.scope();
        }
        Ok(mails)
    }
    pub fn send(
        &self,
        to: &str,
        cc: &str,
        bcc: &str,
        subject: &str,
        body: &str,
        html: &str,
    ) -> Result<(), String> {
        if [to, cc, bcc, subject, &self.address]
            .iter()
            .any(|s| s.contains(['\r', '\n']))
        {
            return Err("Message headers must fit on one line".into());
        }
        let to_recipients = recipients(to)?;
        if to_recipients.is_empty() {
            return Err("Enter a recipient email address.".into());
        }
        let cc_recipients = recipients(cc)?;
        let bcc_recipients = recipients(bcc)?;
        // SMTP uses envelope recipients; never expose Bcc in the MIME headers.
        let copy_headers = if cc.trim().is_empty() {
            String::new()
        } else {
            format!("Cc: {}\r\n", cc_recipients.join(", "))
        };
        // Gmail's raw-message API derives hidden recipients from this header.
        let copy_headers = if self.provider == "gmail" && !bcc.trim().is_empty() {
            format!("{copy_headers}Bcc: {}\r\n", bcc_recipients.join(", "))
        } else {
            copy_headers
        };
        let to = to_recipients.join(", ");
        let raw = format!("From: {}\r\nTo: {to}\r\n{copy_headers}Subject: {subject}\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"sparky-alternative\"\r\n\r\n--sparky-alternative\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--sparky-alternative\r\nContent-Type: text/html; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--sparky-alternative--\r\n", self.address, mime_base64(body), mime_base64(html));
        let (url, payload, mut options) = match self.provider.as_str() {
            "gmail" => ("https://gmail.googleapis.com/gmail/v1/users/me/messages/send".to_owned(), serde_json::json!({"raw": URL_SAFE_NO_PAD.encode(raw)}).to_string(), vec![("header", "Content-Type: application/json".into())]),
            "graph" => ("https://graph.microsoft.com/v1.0/me/sendMail".to_owned(), serde_json::json!({"message":{"subject":subject,"body":{"contentType":"HTML","content":html},"toRecipients":graph_recipients(&to_recipients),"ccRecipients":graph_recipients(&cc_recipients),"bccRecipients":graph_recipients(&bcc_recipients)}}).to_string(), vec![("header", "Content-Type: application/json".into())]),
            "imap" if valid_host(&self.smtp_host) => (format!("smtps://{}", self.smtp_host), raw, vec![("mail-from", self.address.clone()), ("mail-rcpt", to_recipients[0].clone())]),
            _ => return Err("Add a valid SMTP host in account settings to enable sending".into()),
        };
        if self.provider == "imap" {
            options.retain(|(key, _)| *key != "mail-rcpt");
            options.extend(
                to_recipients
                    .iter()
                    .chain(&cc_recipients)
                    .chain(&bcc_recipients)
                    .map(|address| ("mail-rcpt", address.clone())),
            );
        }
        // Private temporary payload; credentials stay in curl's stdin configuration.
        let path = std::env::temp_dir().join(format!(
            "sparky-send-{}-{}",
            std::process::id(),
            hash(payload.as_bytes())
        ));
        let mut file_options = std::fs::OpenOptions::new();
        file_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            file_options.mode(0o600);
        }
        let result = (|| {
            let mut file = file_options.open(&path).map_err(|e| e.to_string())?;
            file.write_all(payload.as_bytes())
                .map_err(|e| e.to_string())?;
            options.push((
                if self.provider == "imap" {
                    "upload-file"
                } else {
                    "data-binary"
                },
                format!(
                    "{}{}",
                    if self.provider == "imap" { "" } else { "@" },
                    path.display()
                ),
            ));
            self.get(&url, &options).map(|_| ())
        })();
        let _ = std::fs::remove_file(path);
        result
    }
}
fn mime_base64(text: &str) -> String {
    STANDARD
        .encode(text)
        .as_bytes()
        .chunks(76)
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}
fn hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}
fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
}
pub(crate) fn transport(options: &[(&str, String)]) -> Result<String, String> {
    let mut config = String::from("silent\nshow-error\nfail\nconnect-timeout = 15\nmax-time = 45\nproto = \"=https,imaps,smtps\"\n");
    for (key, value) in options {
        if value.contains(['\r', '\n', '\0']) {
            return Err("Invalid account configuration".into());
        }
        config.push_str(&format!(
            "{key} = \"{}\"\n",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        ));
    }
    let mut child = Command::new("/usr/bin/curl")
        .args(["--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("Could not open secure transport input")?
        .write_all(config.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(
            "Mail server request failed. Check the account token, permissions and connection."
                .into(),
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
fn gmail_headers(payload: &Value) -> String {
    payload["headers"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| {
            format!(
                "{}: {}\n",
                h["name"].as_str().unwrap_or_default(),
                h["value"].as_str().unwrap_or_default()
            )
        })
        .collect()
}
fn gmail_body(payload: &Value) -> String {
    if payload["mimeType"] == "text/plain" {
        return URL_SAFE_NO_PAD
            .decode(
                payload["body"]["data"]
                    .as_str()
                    .unwrap_or_default()
                    .trim_end_matches('='),
            )
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
    }
    payload["parts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(gmail_body)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
fn parse_headers(headers: &str, mut body: String) -> Mail {
    let unfolded = headers
        .replace("\r\n", "\n")
        .replace("\n\t", " ")
        .replace("\n ", " ");
    let header = |key: &str| {
        unfolded
            .lines()
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_owned())
            .unwrap_or_default()
    };
    if header("Content-Transfer-Encoding").eq_ignore_ascii_case("base64") {
        if let Ok(bytes) = STANDARD.decode(body.split_whitespace().collect::<String>()) {
            body = String::from_utf8_lossy(&bytes).into_owned();
        }
    }
    let from = header("From");
    let references = format!("{} {}", header("References"), header("In-Reply-To"))
        .split_whitespace()
        .filter(|s| s.starts_with('<') && s.ends_with('>'))
        .map(str::to_owned)
        .collect();
    Mail {
        initials: from
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(2)
            .collect::<String>()
            .to_uppercase(),
        from,
        cc: header("Cc"),
        bcc: String::new(),
        subject: header("Subject"),
        preview: body.chars().take(65).collect(),
        body,
        marks: Vec::new(),
        time: crate::date_groups::normalize(&header("Date")),
        category: "People".into(),
        labels: Vec::new(),
        folder: "Inbox".into(),
        unread: true,
        star: false,
        color: [225, 219, 244],
        snoozed_until: None,
        account: String::new(),
        remote_id: String::new(),
        remote_thread: String::new(),
        message_id: header("Message-ID"),
        references,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipient_lists_support_copy_fields_and_reject_header_injection() {
        let addresses =
            recipients("Alex <alex@example.com>, team@example.com; backup@example.com").unwrap();
        assert_eq!(
            addresses,
            vec!["alex@example.com", "team@example.com", "backup@example.com"]
        );
        assert!(recipients("").unwrap().is_empty());
        assert!(recipients("not-an-address").is_err());
        assert!(recipients("a@example.com\r\nBcc: hidden@example.com").is_err());
        assert_eq!(
            graph_recipients(&addresses)[1]["emailAddress"]["address"],
            "team@example.com"
        );
    }
    #[test]
    fn connection_validates_fields_without_network() {
        let account = Account::new(
            "gmail".into(),
            " ME@Example.com ".into(),
            "token".into(),
            String::new(),
            String::new(),
        )
        .unwrap();
        assert_eq!(account.scope(), "gmail:me@example.com");
        assert!(Account::new(
            "imap".into(),
            "me@example.com".into(),
            "password".into(),
            "imap.example.com:993".into(),
            "smtp.example.com:465".into()
        )
        .is_ok());
        assert!(Account::new(
            "imap".into(),
            "me@example.com".into(),
            "password".into(),
            "bad/host".into(),
            String::new()
        )
        .is_err());
        assert!(Account::new(
            "gmail".into(),
            "me@example.com".into(),
            "token\nheader".into(),
            String::new(),
            String::new()
        )
        .is_err());
    }
    #[test]
    fn multiple_accounts_restore_with_credentials_and_disconnect_stays_removed() {
        let one = Account::new("imap".into(), "one@example.com".into(), "app-password".into(),
            "imap.example.com:993".into(), "smtp.example.com:465".into()).unwrap();
        one.profile_name.set("First Person".into()).unwrap();
        let mut two = Account::new("gmail".into(), "two@example.com".into(), "access-token".into(), "".into(), "".into()).unwrap();
        two.secret.clear();
        two.oauth = Some(std::sync::Arc::new(std::sync::Mutex::new(crate::oauth::Session::restore(
            serde_json::json!({"token":"access-token", "refresh":"refresh-token", "config":{
                "id":"123.apps.googleusercontent.com", "secret":"desktop-secret"}})
        ).unwrap())));
        let restored = restore_accounts(&encode_accounts(&[one, two]).unwrap()).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].display_name(), "First Person");
        assert_eq!(restored[0].secret, "app-password");
        assert_eq!(restored[0].imap_host, "imap.example.com:993");
        assert_eq!(restored[0].smtp_host, "smtp.example.com:465");
        assert_eq!(restored[1].scope(), "gmail:two@example.com");
        assert!(restored[1].secret.is_empty());
        assert_eq!(restored[1].oauth.as_ref().unwrap().lock().unwrap().saved()["refresh"], "refresh-token");
        let remaining = restore_accounts(&encode_accounts(&restored[1..]).unwrap()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].scope(), restored[1].scope());
        assert!(restore_accounts(&encode_accounts(&[]).unwrap()).unwrap().is_empty());
        assert!(restore_accounts(b"invalid").is_err());
    }
    #[test]
    fn profile_name_is_shared_and_address_is_the_fallback() {
        let account = Account::new("gmail".into(), "me@example.com".into(),
            "token".into(), String::new(), String::new()).unwrap();
        assert_eq!(account.display_name(), "me@example.com");
        account.profile_name.set("Jamie Rivera".into()).unwrap();
        assert_eq!(account.clone().display_name(), "Jamie Rivera");
        assert_eq!(account.initials(), "JR");
    }
    #[test]
    fn metadata_and_gmail_plain_text_are_normalized() {
        let mail = parse_headers("From: Alex\r\nSubject: Hello\r\n world\r\nMessage-ID: <b>\r\nReferences: <a>\r\nIn-Reply-To: <a>", "Hi".into());
        assert_eq!(mail.subject, "Hello world");
        assert_eq!(mail.message_id, "<b>");
        assert_eq!(mail.references, vec!["<a>", "<a>"]);
        assert_eq!(
            gmail_body(
                &serde_json::json!({"mimeType":"multipart/alternative","parts":[{"mimeType":"text/plain","body":{"data":"SGVsbG8"}}]})
            ),
            "Hello"
        );
    }
}
