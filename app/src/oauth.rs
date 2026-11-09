//! Google desktop OAuth: system browser, ephemeral loopback callback, PKCE and state.
//! Session tokens are stored only in macOS Keychain, never in command-line arguments.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
const SCOPES: &str =
    "openid profile https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.send";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Config {
    id: String,
    secret: String,
}
impl Config {
    fn load() -> Result<Self, String> {
        let config = Self {
            id: std::env::var("SPARKY_GOOGLE_CLIENT_ID").unwrap_or_else(|_| {
                option_env!("SPARKY_GOOGLE_CLIENT_ID")
                    .unwrap_or_default()
                    .into()
            }),
            secret: std::env::var("SPARKY_GOOGLE_CLIENT_SECRET").unwrap_or_else(|_| {
                option_env!("SPARKY_GOOGLE_CLIENT_SECRET")
                    .unwrap_or_default()
                    .into()
            }),
        };
        if !config.id.is_empty() || !config.secret.is_empty() {
            return config.validate();
        }
        // A distributed app carries its own registered desktop client.
        if let Ok(exe) = std::env::current_exe() {
            if let Some(contents) = exe.parent().and_then(|p| p.parent()) {
                let path = contents.join("Resources/google-oauth.json");
                if path.exists() {
                    return Self::from_json(
                        &std::fs::read_to_string(path)
                            .map_err(|_| "Could not read bundled Google setup")?,
                    );
                }
            }
        }
        let path = config_path()?;
        let json = std::fs::read_to_string(path)
            .map_err(|_| "Open Google sign-in setup to register this app before connecting.")?;
        Self::from_json(&json)
    }
    fn validate(self) -> Result<Self, String> {
        if !self.id.ends_with(".apps.googleusercontent.com")
            || self.id.contains(char::is_whitespace)
            || self.secret.trim().is_empty()
        {
            return Err(
                "Use the configuration downloaded for a Google Desktop app OAuth client.".into(),
            );
        }
        Ok(self)
    }
    fn from_json(json: &str) -> Result<Self, String> {
        let data: Value = serde_json::from_str(json)
            .map_err(|_| "Paste the complete JSON file downloaded from Google Cloud.")?;
        let installed = data.get("installed").ok_or(
            "Choose a Desktop app client in Google Cloud, rather than a Web application client.",
        )?;
        Self {
            id: installed["client_id"].as_str().unwrap_or_default().into(),
            secret: installed["client_secret"]
                .as_str()
                .unwrap_or_default()
                .into(),
        }
        .validate()
    }
}
fn config_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("Could not locate your home folder")?;
    Ok(std::path::PathBuf::from(home).join("Library/Application Support/Sparky/google-oauth.json"))
}
/// Store only desktop client configuration, never account tokens or unrelated JSON.
pub fn save_configuration(json: &str) -> Result<(), String> {
    let config = Config::from_json(json)?;
    let path = config_path()?;
    std::fs::create_dir_all(path.parent().unwrap())
        .map_err(|_| "Could not create the Google setup folder")?;
    let data = serde_json::json!({"installed": {
        "client_id": config.id, "client_secret": config.secret
    }});
    std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap())
        .map_err(|_| "Could not save Google setup".into())
}
pub fn configured() -> bool {
    Config::load().is_ok()
}
pub struct Session {
    pub token: String,
    refresh: String,
    expires: Instant,
    config: Config,
}
impl Session {
    pub fn saved(&self) -> Value {
        serde_json::json!({"token": self.token, "refresh": self.refresh, "config": self.config})
    }
    pub fn restore(value: Value) -> Result<Self, String> {
        let config: Config = serde_json::from_value(value["config"].clone())
            .map_err(|_| "Saved Google setup is invalid. Reconnect in Settings.")?;
        Ok(Self {
            token: token(&serde_json::json!({"access_token": value["token"]}))?,
            refresh: value["refresh"].as_str().unwrap_or_default().into(),
            config: config.validate()?,
            // Renew before the first request; wall clock changes cannot extend a token.
            expires: Instant::now(),
        })
    }

    pub fn access_token(&mut self) -> Result<String, String> {
        if Instant::now() >= self.expires {
            if self.refresh.is_empty() {
                return Err("Google session expired. Connect with Google again.".into());
            }
            let data = exchange(
                &self.config,
                &[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", &self.refresh),
                ],
            )?;
            self.token = token(&data)?;
            self.expires = expiry(&data);
            if let Some(refresh) = data["refresh_token"].as_str() {
                self.refresh = refresh.into();
            }
        }
        Ok(self.token.clone())
    }
}
fn expiry(data: &Value) -> Instant {
    Instant::now()
        + Duration::from_secs(
            data["expires_in"]
                .as_u64()
                .unwrap_or(3600)
                .min(86400)
                .saturating_sub(60),
        )
}
fn token(data: &Value) -> Result<String, String> {
    let value = data["access_token"]
        .as_str()
        .filter(|s| !s.is_empty() && !s.contains(['\r', '\n', '\0']))
        .ok_or("Google did not return an access token. Please reconnect.")?;
    Ok(value.into())
}
fn form(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs.iter().copied())
        .finish()
}
fn exchange(config: &Config, pairs: &[(&str, &str)]) -> Result<Value, String> {
    let mut pairs = pairs.to_vec();
    pairs.extend([
        ("client_id", config.id.as_str()),
        ("client_secret", config.secret.as_str()),
    ]);
    let body = crate::sync::transport(&[
        ("url", "https://oauth2.googleapis.com/token".into()),
        (
            "header",
            "Content-Type: application/x-www-form-urlencoded".into(),
        ),
        ("data", form(&pairs)),
    ])
    .map_err(|_| {
        "Google sign-in or renewal failed. Check your connection and reconnect with Google."
            .to_string()
    })?;
    serde_json::from_str(&body).map_err(|_| "Google returned an invalid sign-in response.".into())
}
fn random() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| "Could not create a secure sign-in request")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn challenge(verifier: &str) -> Result<String, String> {
    let mut child = Command::new("/usr/bin/openssl")
        .args(["dgst", "-sha256", "-binary"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not prepare secure Google sign-in")?;
    child
        .stdin
        .take()
        .ok_or("Could not open PKCE input")?
        .write_all(verifier.as_bytes())
        .map_err(|_| "Could not prepare PKCE")?;
    let output = child
        .wait_with_output()
        .map_err(|_| "Could not prepare PKCE")?;
    if !output.status.success() || output.stdout.len() != 32 {
        return Err("Could not prepare PKCE".into());
    }
    Ok(URL_SAFE_NO_PAD.encode(output.stdout))
}
// None means an unrelated or untrusted request; keep waiting for the genuine callback.
fn callback(target: &str, state: &str) -> Option<Result<String, String>> {
    let url = url::Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    if url.path() != "/oauth/callback" {
        return None;
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    if pairs.iter().filter(|(k, _)| k == "state").count() != 1
        || !pairs.iter().any(|(k, v)| k == "state" && v == state)
    {
        return None;
    }
    if pairs.iter().any(|(k, _)| k == "error") {
        return Some(Err("Google sign-in was declined. You can try again.".into()));
    }
    let codes: Vec<_> = pairs.iter().filter(|(k, _)| k == "code").collect();
    Some(if codes.len() == 1 && !codes[0].1.is_empty() {
        Ok(codes[0].1.to_string())
    } else {
        Err("Google did not return an authorization code. Try again.".into())
    })
}
// Keep the browser response static: never echo authorization codes or state into HTML.
fn callback_response(result: &Option<Result<String, String>>) -> String {
    let (status, title, message) = match result {
        Some(Ok(_)) => (
            "200 OK",
            "Returning to Sparky…",
            "Google authorization received. Sparky is opening automatically to finish connecting. You can close this tab.",
        ),
        Some(Err(_)) => (
            "200 OK",
            "Returning to Sparky…",
            "Google sign-in could not complete. Sparky is opening automatically so you can try again. You can close this tab.",
        ),
        None => (
            "400 Bad Request",
            "Invalid callback",
            "Return to the Google sign-in window and try again.",
        ),
    };
    let body = format!(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title><body><main><h1>{title}</h1><p>{message}</p></main></body></html>"#
    );
    format!("HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len())
}
pub fn connect(
    cancel: Arc<AtomicBool>,
    return_to_app: impl FnOnce(),
) -> Result<crate::sync::Account, String> {
    let config = Config::load()?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|_| "Could not start the Google sign-in callback")?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let redirect = format!(
        "http://127.0.0.1:{}/oauth/callback",
        listener.local_addr().map_err(|e| e.to_string())?.port()
    );
    let state = random()?;
    let verifier = random()?;
    let challenge = challenge(&verifier)?;
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?{}",
        form(&[
            ("client_id", &config.id),
            ("redirect_uri", &redirect),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("state", &state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ])
    );
    let status = Command::new("/usr/bin/open")
        .arg(&url)
        .status()
        .map_err(|_| "Could not open your browser")?;
    if !status.success() {
        return Err("Could not open your browser. Please try again.".into());
    }
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Google sign-in cancelled.".into());
        }
        if Instant::now() >= deadline {
            return Err("Google sign-in timed out. Please try again.".into());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                let mut request = Vec::new();
                let mut byte = [0];
                while request.len() < 8192 && !request.ends_with(b"\r\n") {
                    if stream.read(&mut byte).unwrap_or(0) == 0 {
                        break;
                    }
                    request.push(byte[0]);
                }
                let line = String::from_utf8_lossy(&request);
                let parts: Vec<_> = line.split_whitespace().collect();
                let result = if parts.first() == Some(&"GET") {
                    parts.get(1).and_then(|p| callback(p, &state))
                } else {
                    None
                };
                let _ = stream.write_all(callback_response(&result).as_bytes());
                if let Some(result) = result {
                    // Only a callback for this pending sign-in may activate our window.
                    // Activate before network work, including when Google declines sign-in.
                    return_to_app();
                    let code = result?;
                    let data = exchange(
                        &config,
                        &[
                            ("grant_type", "authorization_code"),
                            ("code", &code),
                            ("redirect_uri", &redirect),
                            ("code_verifier", &verifier),
                        ],
                    )?;
                    let scopes = data["scope"].as_str().unwrap_or_default();
                    if !SCOPES
                        .split_whitespace()
                        .filter(|scope| scope.starts_with("https://www.googleapis.com/auth/gmail."))
                        .all(|scope| scopes.split_whitespace().any(|s| s == scope))
                    {
                        return Err(
                            "Please allow both reading and sending Gmail to connect Sparky.".into(),
                        );
                    }
                    return crate::sync::Account::google(Session {
                        token: token(&data)?,
                        refresh: data["refresh_token"].as_str().unwrap_or_default().into(),
                        expires: expiry(&data),
                        config,
                    });
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(_) => return Err("Google sign-in callback stopped. Please try again.".into()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_page_does_not_expose_credentials_or_claim_connection() {
        let response = callback_response(&Some(Ok("private-authorization-code".into())));
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("Sparky is opening automatically to finish connecting"));
        assert!(!response.contains("private-authorization-code"));
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(headers.contains(&format!("Content-Length: {}", body.len())));
        assert!(
            callback_response(&Some(Err("private-error".into()))).contains("so you can try again")
        );
        assert!(callback_response(&None).starts_with("HTTP/1.1 400 Bad Request\r\n"));
    }
    #[test]
    fn restored_google_session_requires_renewal() {
        let restored = Session::restore(serde_json::json!({"token":"old-token", "refresh":"refresh-token",
            "config":{"id":"123.apps.googleusercontent.com", "secret":"desktop-secret"}})).unwrap();
        assert!(restored.expires <= Instant::now());
        assert_eq!(restored.refresh, "refresh-token");
        assert_eq!(restored.saved()["token"], "old-token");
    }
    #[test]
    fn desktop_configuration_validation() {
        assert!(Config::from_json(r#"{"installed":{"client_id":"123.apps.googleusercontent.com","client_secret":"desktop-secret"}}"#).is_ok());
        for invalid in [
            "not json",
            r#"{"web":{"client_id":"123.apps.googleusercontent.com","client_secret":"secret"}}"#,
            r#"{"installed":{"client_id":"invalid","client_secret":"secret"}}"#,
            r#"{"installed":{"client_id":"123.apps.googleusercontent.com"}}"#,
        ] {
            assert!(Config::from_json(invalid).is_err());
        }
    }
    #[test]
    fn pkce_matches_rfc7636() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
    #[test]
    fn callbacks_require_state_and_decode_code() {
        assert!(callback("/oauth/callback?code=secret&state=wrong", "expected").is_none());
        assert!(callback("/other?code=secret&state=expected", "expected").is_none());
        assert!(callback(
            "/oauth/callback?code=secret&state=expected&state=expected",
            "expected"
        )
        .is_none());
        assert_eq!(
            callback("/oauth/callback?code=a%2Bb&state=expected", "expected")
                .unwrap()
                .unwrap(),
            "a+b"
        );
        assert!(callback(
            "/oauth/callback?error=access_denied&state=expected",
            "expected"
        )
        .unwrap()
        .is_err());
        assert!(callback("/oauth/callback?state=expected", "expected")
            .unwrap()
            .is_err());
    }
}
