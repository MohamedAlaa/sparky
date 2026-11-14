//! SQLite3 owns persistence and tokenization; each connection stays on its own thread.
use crate::{Mail, Stored};
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    path::Path,
    ptr,
};

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open_v2(
        path: *const c_char,
        db: *mut *mut c_void,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn sqlite3_errmsg(db: *mut c_void) -> *const c_char;
    fn sqlite3_busy_timeout(db: *mut c_void, ms: c_int) -> c_int;
    fn sqlite3_exec(
        db: *mut c_void,
        sql: *const c_char,
        callback: Option<
            unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
        >,
        data: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
}

struct Connection(*mut c_void);
impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.0);
        }
    }
}
impl Connection {
    fn open(path: &Path, read_only: bool) -> Result<Self, String> {
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        let mut db = ptr::null_mut();
        // FULLMUTEX; the UI reader and writer never share a native handle.
        let code = unsafe {
            sqlite3_open_v2(
                path.as_ptr(),
                &mut db,
                if read_only { 1 } else { 2 | 4 } | 0x10000,
                ptr::null(),
            )
        };
        let connection = Self(db);
        if code != 0 {
            return Err(connection.error());
        }
        unsafe {
            sqlite3_busy_timeout(db, 5000);
        }
        connection.exec("PRAGMA mmap_size=268435456;")?;
        Ok(connection)
    }
    fn error(&self) -> String {
        if self.0.is_null() {
            return "Could not open SQLite database".into();
        }
        unsafe {
            CStr::from_ptr(sqlite3_errmsg(self.0))
                .to_string_lossy()
                .into_owned()
        }
    }
    fn query(&self, sql: &str) -> Result<Vec<Vec<String>>, String> {
        unsafe extern "C" fn collect(
            data: *mut c_void,
            count: c_int,
            values: *mut *mut c_char,
            _: *mut *mut c_char,
        ) -> c_int {
            let rows = unsafe { &mut *(data as *mut Vec<Vec<String>>) };
            let row = (0..count as usize)
                .map(|i| {
                    let value = unsafe { *values.add(i) };
                    if value.is_null() {
                        String::new()
                    } else {
                        unsafe { CStr::from_ptr(value).to_string_lossy().into_owned() }
                    }
                })
                .collect();
            rows.push(row);
            0
        }
        let sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut rows: Vec<Vec<String>> = Vec::new();
        let code = unsafe {
            sqlite3_exec(
                self.0,
                sql.as_ptr(),
                Some(collect),
                &mut rows as *mut _ as *mut c_void,
                ptr::null_mut(),
            )
        };
        if code != 0 {
            Err(self.error())
        } else {
            Ok(rows)
        }
    }
    fn exec(&self, sql: &str) -> Result<(), String> {
        self.query(sql).map(|_| ())
    }
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\0', "").replace('\'', "''"))
}

pub struct Store {
    reader: Connection,
    writer: Connection,
}
impl Store {
    pub fn open(path: &Path, legacy: &Path) -> Result<Self, String> {
        let writer = Connection::open(path, false)?;
        writer.exec("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, sender TEXT, subject TEXT, body TEXT);
            CREATE VIRTUAL TABLE IF NOT EXISTS mail_fts USING fts5(sender, subject, body, content='messages', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
            CREATE TRIGGER IF NOT EXISTS mail_insert AFTER INSERT ON messages BEGIN INSERT INTO mail_fts(rowid,sender,subject,body) VALUES(new.id,new.sender,new.subject,new.body); END;
            CREATE TRIGGER IF NOT EXISTS mail_delete AFTER DELETE ON messages BEGIN INSERT INTO mail_fts(mail_fts,rowid,sender,subject,body) VALUES('delete',old.id,old.sender,old.subject,old.body); END;
            CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT);")?;
        let reader = Connection::open(path, true)?;
        let store = Self { reader, writer };
        if store
            .reader
            .query("SELECT value FROM metadata WHERE key='initialized'")?
            .is_empty()
        {
            // A corrupt existing mailbox must never be silently replaced with demo messages.
            let mails = if legacy.exists() {
                serde_json::from_str::<Stored>(
                    &std::fs::read_to_string(legacy).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
                .mails
            } else {
                crate::sample()
            };
            store.save(&mails)?;
            store
                .writer
                .exec("INSERT INTO metadata VALUES('initialized','1')")?;
        }
        Ok(store)
    }
    pub fn accent(&self) -> Result<crate::theme::Accent, String> {
        let rows = self.reader.query("SELECT value FROM metadata WHERE key='main_color'")?;
        Ok(rows.first().map(|r| crate::theme::Accent::from_name(&r[0])).unwrap_or_default())
    }
    pub fn save_accent(&self, accent: crate::theme::Accent) -> Result<(), String> {
        self.writer.exec(&format!("INSERT OR REPLACE INTO metadata VALUES('main_color',{})", quote(accent.name())))
    }
    pub fn classifier_settings(&self) -> Result<crate::classifier::Settings, String> {
        let rows = self.reader.query("SELECT value FROM metadata WHERE key='classifier_settings'")?;
        rows.first().map(|r| serde_json::from_str(&r[0]).map_err(|e| e.to_string())).unwrap_or_else(|| Ok(Default::default()))
    }
    pub fn save_classifier_settings(&self, settings: &crate::classifier::Settings) -> Result<(), String> {
        let value = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        self.writer.exec(&format!("INSERT OR REPLACE INTO metadata VALUES('classifier_settings',{})", quote(&value)))
    }
    /// Most recently used palette commands, newest first.
    pub fn recent_commands(&self) -> Result<Vec<String>, String> {
        let rows = self.reader.query("SELECT value FROM metadata WHERE key='recent_commands'")?;
        Ok(rows.first().and_then(|r| serde_json::from_str(&r[0]).ok()).unwrap_or_default())
    }
    pub fn save_recent_commands(&self, names: &[String]) -> Result<(), String> {
        let value = serde_json::to_string(names).map_err(|e| e.to_string())?;
        self.writer.exec(&format!("INSERT OR REPLACE INTO metadata VALUES('recent_commands',{})", quote(&value)))
    }
    /// Most recent mail searches, newest first.
    pub fn recent_searches(&self) -> Result<Vec<String>, String> {
        let rows = self.reader.query("SELECT value FROM metadata WHERE key='recent_searches'")?;
        Ok(rows.first().and_then(|r| serde_json::from_str(&r[0]).ok()).unwrap_or_default())
    }
    pub fn save_recent_searches(&self, queries: &[String]) -> Result<(), String> {
        let value = serde_json::to_string(queries).map_err(|e| e.to_string())?;
        self.writer.exec(&format!("INSERT OR REPLACE INTO metadata VALUES('recent_searches',{})", quote(&value)))
    }
    pub fn onboarding_complete(&self) -> Result<bool, String> {
        Ok(!self.reader.query("SELECT value FROM metadata WHERE key='onboarding_complete' AND value='1'")?.is_empty())
    }
    pub fn complete_onboarding(&self) -> Result<(), String> {
        self.writer.exec("INSERT OR REPLACE INTO metadata VALUES('onboarding_complete','1')")
    }
    pub fn load(&self) -> Result<Vec<Mail>, String> {
        self.reader
            .query("SELECT payload FROM messages ORDER BY id")?
            .into_iter()
            .map(|r| serde_json::from_str(&r[0]).map_err(|e| e.to_string()))
            .collect()
    }
    pub fn save(&self, mails: &[Mail]) -> Result<(), String> {
        self.writer.exec("BEGIN IMMEDIATE")?;
        let result = (|| {
            self.writer.exec("DELETE FROM messages")?;
            for (i, m) in mails.iter().enumerate() {
                let json = serde_json::to_string(m).map_err(|e| e.to_string())?;
                self.writer.exec(&format!(
                    "INSERT INTO messages VALUES({},{},{},{},{})",
                    i,
                    quote(&json),
                    quote(&m.from),
                    quote(&m.subject),
                    // Attachment names are searchable alongside the body text.
                    quote(&m.attachments.iter().fold(m.body.clone(), |text, a| text + "\n" + &a.name))
                ))?;
            }
            self.writer.exec("COMMIT")
        })();
        if result.is_err() {
            let _ = self.writer.exec("ROLLBACK");
        }
        result
    }
    pub fn search(&self, text: &str) -> Result<Vec<usize>, String> {
        let tokens: Vec<_> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{s}\"*"))
            .collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }
        self.reader
            .query(&format!(
                "SELECT rowid FROM mail_fts WHERE mail_fts MATCH {}",
                quote(&tokens.join(" AND "))
            ))?
            .into_iter()
            .map(|r| r[0].parse::<usize>().map_err(|e| e.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn onboarding_stays_complete_with_an_empty_mailbox_after_restart() {
        let dir = std::env::temp_dir().join(format!("sparky-onboarding-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        let legacy = dir.join("absent.json");
        {
            let store = Store::open(&db, &legacy).unwrap();
            assert!(!store.onboarding_complete().unwrap());
            let mut settings = crate::classifier::Settings { automatic: true, context: 1024, ..Default::default() };
            settings.learn(&crate::sample()[0]);
            store.save_classifier_settings(&settings).unwrap();
            store.save_accent(crate::theme::Accent::Teal).unwrap();
            assert!(store.recent_commands().unwrap().is_empty());
            store.save_recent_commands(&["Archive".into(), "It's \"quoted\"".into()]).unwrap();
            store.complete_onboarding().unwrap();
            store.complete_onboarding().unwrap();
            store.save(&[]).unwrap();
        }
        let store = Store::open(&db, &legacy).unwrap();
        assert_eq!(store.accent().unwrap(), crate::theme::Accent::Teal);
        assert_eq!(store.recent_commands().unwrap(), vec!["Archive".to_owned(), "It's \"quoted\"".to_owned()]);
        assert!(store.onboarding_complete().unwrap());
        assert!(store.classifier_settings().unwrap().automatic);
        assert_eq!(store.classifier_settings().unwrap().context, 1024);
        assert!(store.classifier_settings().unwrap().corrected(&crate::sample()[0]).is_some());
        assert!(store.load().unwrap().is_empty());
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn migration_fts_and_wal_reader_snapshot() {
        let dir = std::env::temp_dir().join(format!("sparky-db-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");
        let store = Store::open(&db, &dir.join("absent.json")).unwrap();
        assert_eq!(
            store.reader.query("PRAGMA journal_mode").unwrap()[0][0],
            "wal"
        );
        assert!(
            store.reader.query("PRAGMA mmap_size").unwrap()[0][0]
                .parse::<u64>()
                .unwrap()
                > 0
        );
        assert_eq!(store.search("COFFEE").unwrap().len(), 2);
        assert!(store
            .search("\" OR * ; DROP TABLE messages;")
            .unwrap()
            .is_empty());
        store.reader.exec("BEGIN").unwrap();
        let before = store.load().unwrap().len();
        store.save(&[]).unwrap();
        assert_eq!(store.load().unwrap().len(), before);
        store.reader.exec("COMMIT").unwrap();
        assert!(store.load().unwrap().is_empty());
        assert!(store.search("coffee").unwrap().is_empty());
        drop(store);
        assert!(Store::open(&db, &dir.join("absent.json"))
            .unwrap()
            .load()
            .unwrap()
            .is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
