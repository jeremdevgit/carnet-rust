//! Own database only. Never opens or changes a Joplin profile.
use crate::{Error, Result, id, now};
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};

pub const MAX_BODY: usize = 4 * 1024 * 1024;
pub const MAX_ATTACHMENT: u64 = 32 * 1024 * 1024;
pub const MAX_RESULTS: usize = 500;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notebook {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub notebook_id: String,
    pub title: String,
    pub body: String,
    pub is_todo: bool,
    pub completed: bool,
    pub due: i64,
    pub created: i64,
    pub updated: i64,
    pub deleted: bool,
    pub version: i64,
    /// Unknown Joplin fields are retained for round trips, never interpreted.
    pub extra: String,
}
#[derive(Clone, Debug)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub is_todo: bool,
    pub completed: bool,
    pub updated: i64,
}
#[derive(Clone, Debug)]
pub struct Revision {
    pub seq: i64,
    pub at: i64,
    pub title: String,
    pub body: String,
}
#[derive(Clone, Debug)]
pub struct Attachment {
    pub id: String,
    pub title: String,
    pub extension: String,
    pub mime: String,
    pub data: Vec<u8>,
    pub extra: String,
}

pub struct Store {
    pub(crate) conn: Connection,
    // Released by RAII; OS-level lock prevents two writers losing drafts.
    _lock: Option<File>,
    key: Option<crate::vault::Secret>,
    path: Option<PathBuf>,
}

fn note_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: r.get(0)?,
        notebook_id: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        is_todo: r.get(4)?,
        completed: r.get(5)?,
        due: r.get(6)?,
        created: r.get(7)?,
        updated: r.get(8)?,
        deleted: r.get(9)?,
        version: r.get(10)?,
        extra: r.get(11)?,
    })
}
pub fn valid_id(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn validate_note(n: &Note) -> Result<()> {
    if !valid_id(&n.id) || !valid_id(&n.notebook_id) {
        return Err(Error::Invalid("identifier"));
    }
    if n.title.trim().is_empty() || n.title.len() > 1024 || n.title.contains(['\n', '\r', '\0']) {
        return Err(Error::Invalid("title"));
    }
    if n.body.len() > MAX_BODY || n.body.contains('\0') || n.extra.len() > MAX_BODY {
        return Err(Error::Invalid("note is too large or contains NUL"));
    }
    serde_json::from_str::<std::collections::BTreeMap<String, String>>(&n.extra)
        .map_err(|_| Error::Invalid("note metadata"))?;
    Ok(())
}

impl Store {
    pub fn open(dir: &Path, password: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("profile.lock"))?;
        lock.try_lock_exclusive()?;
        let path = crate::vault::database_path(dir);
        match crate::vault::profile_state(dir)? {
            crate::vault::ProfileState::New => crate::vault::validate_new_password(password)?,
            crate::vault::ProfileState::Plaintext => {
                crate::vault::migrate_plaintext(&path, password)?
            }
            crate::vault::ProfileState::Encrypted => {}
        }
        let conn = Connection::open(&path)?;
        crate::vault::key_connection(&conn, password)?;
        let mut store = Self::initialize(conn, Some(lock))?;
        store.key = Some(crate::vault::Secret::new(password));
        store.path = Some(path);
        Ok(store)
    }
    pub fn profile_dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }
    pub fn is_encrypted(&self) -> bool {
        self.key.is_some()
    }
    pub fn memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?, None)
    }
    fn initialize(conn: Connection, lock: Option<File>) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY; PRAGMA secure_delete=ON;")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 2 {
            return Err(Error::Invalid("profile needs a newer application"));
        }
        conn.execute_batch("
          BEGIN IMMEDIATE;
          CREATE TABLE IF NOT EXISTS notebooks(
            id TEXT PRIMARY KEY, parent_id TEXT REFERENCES notebooks(id), title TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS notes(
            id TEXT PRIMARY KEY, notebook_id TEXT NOT NULL REFERENCES notebooks(id),
            title TEXT NOT NULL, body TEXT NOT NULL, is_todo INTEGER NOT NULL DEFAULT 0,
            completed INTEGER NOT NULL DEFAULT 0, due INTEGER NOT NULL DEFAULT 0,
            created INTEGER NOT NULL, updated INTEGER NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0, version INTEGER NOT NULL DEFAULT 0,
            extra TEXT NOT NULL DEFAULT '{}');
          CREATE INDEX IF NOT EXISTS notes_list ON notes(deleted, notebook_id, updated DESC);
          CREATE TABLE IF NOT EXISTS tags(id TEXT PRIMARY KEY, title TEXT NOT NULL UNIQUE);
          CREATE TABLE IF NOT EXISTS note_tags(
            note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
            tag_id TEXT NOT NULL REFERENCES tags(id), PRIMARY KEY(note_id,tag_id));
          CREATE TABLE IF NOT EXISTS revisions(
            seq INTEGER PRIMARY KEY, note_id TEXT NOT NULL REFERENCES notes(id),
            at INTEGER NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS revision_note ON revisions(note_id,seq DESC);
          CREATE TABLE IF NOT EXISTS resources(
            id TEXT PRIMARY KEY, title TEXT NOT NULL, extension TEXT NOT NULL, mime TEXT NOT NULL,
            data BLOB NOT NULL, extra TEXT NOT NULL DEFAULT '{}');
          CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
          CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(note_id UNINDEXED,title,body,tokenize='unicode61');
          CREATE TABLE IF NOT EXISTS drafts(note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,data TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS item_meta(id TEXT PRIMARY KEY,props TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS relation_ids(id TEXT PRIMARY KEY,note_id TEXT NOT NULL,tag_id TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS sync_baseline(path TEXT PRIMARY KEY,hash TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS sync_conflicts(path TEXT PRIMARY KEY,local BLOB,remote BLOB,local_hash TEXT NOT NULL,remote_hash TEXT NOT NULL);
          PRAGMA user_version=2;
          COMMIT;
        ")?;
        Ok(Self {
            conn,
            _lock: lock,
            key: None,
            path: None,
        })
    }
    pub fn notebooks(&self) -> Result<Vec<Notebook>> {
        let mut s = self
            .conn
            .prepare("SELECT id,parent_id,title FROM notebooks ORDER BY title COLLATE NOCASE")?;
        let books: Vec<Notebook> = s
            .query_map([], |r| {
                Ok(Notebook {
                    id: r.get(0)?,
                    parent_id: r.get(1)?,
                    title: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        // Iterative tree walk keeps children below their parent without recursive stack growth.
        let mut children: std::collections::HashMap<Option<&str>, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, b) in books.iter().enumerate() {
            children.entry(b.parent_id.as_deref()).or_default().push(i);
        }
        let mut stack: Vec<usize> = children
            .get(&None)
            .into_iter()
            .flatten()
            .rev()
            .copied()
            .collect();
        let mut visited = std::collections::HashSet::new();
        let mut out = Vec::with_capacity(books.len());
        while let Some(i) = stack.pop() {
            let b = &books[i];
            if !visited.insert(b.id.as_str()) {
                continue;
            }
            out.push(b.clone());
            if let Some(indices) = children.get(&Some(b.id.as_str())) {
                stack.extend(indices.iter().rev().copied());
            }
        }
        // Keep orphaned/corrupt notebook records visible if a profile was edited externally.
        out.extend(
            books
                .iter()
                .filter(|b| !visited.contains(b.id.as_str()))
                .cloned(),
        );
        Ok(out)
    }
    pub fn create_notebook(&self, title: &str, parent: Option<&str>) -> Result<String> {
        if title.trim().is_empty() || title.len() > 1024 || title.contains(['\n', '\r', '\0']) {
            return Err(Error::Invalid("notebook title"));
        }
        let id = id();
        self.conn.execute(
            "INSERT INTO notebooks VALUES(?1,?2,?3)",
            params![id, parent, title.trim()],
        )?;
        self.conn.execute("INSERT INTO item_meta VALUES(?1,?2)",params![id,serde_json::json!({"created_time":crate::exchange::iso(now())?,"updated_time":crate::exchange::iso(now())?}).to_string()])?;
        Ok(id)
    }
    pub fn rename_notebook(&self, id: &str, title: &str) -> Result<()> {
        if title.trim().is_empty() || title.len() > 1024 || title.contains(['\n', '\r', '\0']) {
            return Err(Error::Invalid("notebook title"));
        }
        self.conn.execute(
            "UPDATE notebooks SET title=?2 WHERE id=?1",
            params![id, title.trim()],
        )?;
        self.touch_notebook(id)?;
        Ok(())
    }
    pub fn create_note(&mut self, notebook: &str, title: &str, todo: bool) -> Result<Note> {
        let n = Note {
            id: id(),
            notebook_id: notebook.into(),
            title: title.into(),
            body: String::new(),
            is_todo: todo,
            completed: false,
            due: 0,
            created: now(),
            updated: now(),
            deleted: false,
            version: 0,
            extra: "{}".into(),
        };
        validate_note(&n)?;
        let tx = self.conn.transaction()?;
        Self::insert_note(&tx, &n)?;
        tx.commit()?;
        Ok(n)
    }
    pub(crate) fn insert_note(c: &Connection, n: &Note) -> Result<()> {
        validate_note(n)?;
        c.execute(
            "INSERT INTO notes VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                n.id,
                n.notebook_id,
                n.title,
                n.body,
                n.is_todo,
                n.completed,
                n.due,
                n.created,
                n.updated,
                n.deleted,
                n.version,
                n.extra
            ],
        )?;
        c.execute(
            "INSERT INTO search VALUES(?1,?2,?3)",
            params![n.id, n.title, n.body],
        )?;
        Ok(())
    }
    pub fn note(&self, id: &str) -> Result<Note> {
        Ok(self
            .conn
            .query_row("SELECT * FROM notes WHERE id=?1", [id], note_row)?)
    }
    pub fn save(&mut self, n: &mut Note, tags: &str) -> Result<()> {
        validate_note(n)?;
        let titles: Vec<_> = tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if titles.len() > 64
            || titles
                .iter()
                .any(|s| s.len() > 256 || s.contains(['\n', '\r', '\0']))
        {
            return Err(Error::Invalid("tags"));
        }
        let updated = now();
        let tx = self.conn.transaction()?;
        let old = tx.query_row("SELECT * FROM notes WHERE id=?1", [&n.id], note_row)?;
        if old.version != n.version {
            return Err(Error::Conflict);
        }
        if old.title != n.title || old.body != n.body {
            tx.execute(
                "INSERT INTO revisions(note_id,at,title,body) VALUES(?1,?2,?3,?4)",
                params![n.id, updated, old.title, old.body],
            )?;
            // Bounded history: latest 100 snapshots per note, never silently unbounded.
            tx.execute("DELETE FROM revisions WHERE note_id=?1 AND seq NOT IN (SELECT seq FROM revisions WHERE note_id=?1 ORDER BY seq DESC LIMIT 100)",[&n.id])?;
        }
        tx.execute("UPDATE notes SET notebook_id=?2,title=?3,body=?4,is_todo=?5,completed=?6,due=?7,updated=?8,deleted=?9,extra=?10,version=version+1 WHERE id=?1",
            params![n.id,n.notebook_id,n.title,n.body,n.is_todo,n.completed,n.due,updated,n.deleted,n.extra])?;
        tx.execute("DELETE FROM search WHERE note_id=?1", [&n.id])?;
        tx.execute(
            "INSERT INTO search VALUES(?1,?2,?3)",
            params![n.id, n.title, n.body],
        )?;
        tx.execute("DELETE FROM note_tags WHERE note_id=?1", [&n.id])?;
        for title in titles {
            tx.execute(
                "INSERT OR IGNORE INTO tags VALUES(?1,?2)",
                params![id(), title],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO note_tags SELECT ?1,id FROM tags WHERE title=?2",
                params![n.id, title],
            )?;
        }
        tx.execute("DELETE FROM drafts WHERE note_id=?1", [&n.id])?;
        tx.commit()?;
        n.updated = updated;
        n.version += 1;
        Ok(())
    }
    pub fn tags(&self, note: &str) -> Result<String> {
        let mut s=self.conn.prepare("SELECT t.title FROM tags t JOIN note_tags nt ON nt.tag_id=t.id WHERE nt.note_id=?1 ORDER BY t.title")?;
        Ok(s.query_map([note], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .join(", "))
    }
    pub fn all_tags(&self) -> Result<Vec<String>> {
        let mut s = self
            .conn
            .prepare("SELECT title FROM tags ORDER BY title COLLATE NOCASE")?;
        Ok(s.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn list(
        &self,
        notebook: Option<&str>,
        query: &str,
        trash: bool,
        todo: bool,
        tag: Option<&str>,
        offset: usize,
    ) -> Result<Vec<NoteSummary>> {
        if query.len() > 1024 {
            return Err(Error::Invalid("search too long"));
        }
        // Literal substring fallback covers Chinese/Japanese without whitespace and punctuation.
        // Bound parameters mean quotes, SQL fragments and wildcard characters remain data.
        let mut s=self.conn.prepare("SELECT n.id,n.title,n.is_todo,n.completed,n.updated FROM notes n
          WHERE deleted=?1 AND (?2 IS NULL OR notebook_id=?2)
          AND (?3='' OR instr(lower(n.title),lower(?3))>0 OR instr(lower(n.body),lower(?3))>0)
          AND (?4=0 OR is_todo=1) AND (?5 IS NULL OR EXISTS(
            SELECT 1 FROM note_tags nt JOIN tags t ON t.id=nt.tag_id WHERE nt.note_id=n.id AND t.title=?5))
          ORDER BY n.updated DESC,n.id LIMIT ?6 OFFSET ?7")?;
        Ok(s.query_map(
            params![
                trash,
                notebook,
                query,
                todo,
                tag,
                MAX_RESULTS as i64,
                offset as i64
            ],
            |r| {
                Ok(NoteSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    is_todo: r.get(2)?,
                    completed: r.get(3)?,
                    updated: r.get(4)?,
                })
            },
        )?
        .collect::<rusqlite::Result<_>>()?)
    }
    /// FTS query language is deliberate here; the desktop uses literal search above.
    pub fn full_text(&self, query: &str) -> Result<Vec<String>> {
        if query.len() > 1024 {
            return Err(Error::Invalid("search too long"));
        }
        let mut s=self.conn.prepare("SELECT search.note_id FROM search JOIN notes n ON n.id=search.note_id WHERE search MATCH ?1 AND n.deleted=0 ORDER BY rank LIMIT 500")?;
        Ok(s.query_map([query], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn revisions(&self, note: &str) -> Result<Vec<Revision>> {
        let mut s = self.conn.prepare(
            "SELECT seq,at,title,body FROM revisions WHERE note_id=?1 ORDER BY seq DESC LIMIT 100",
        )?;
        Ok(s.query_map([note], |r| {
            Ok(Revision {
                seq: r.get(0)?,
                at: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    /// Small headers for UI lists; never loads 100 full note bodies each frame.
    pub fn revision_headers(&self, note: &str) -> Result<Vec<Revision>> {
        let mut s = self.conn.prepare(
            "SELECT seq,at,title FROM revisions WHERE note_id=?1 ORDER BY seq DESC LIMIT 100",
        )?;
        Ok(s.query_map([note], |r| {
            Ok(Revision {
                seq: r.get(0)?,
                at: r.get(1)?,
                title: r.get(2)?,
                body: String::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn revision(&self, note: &str, seq: i64) -> Result<Revision> {
        Ok(self.conn.query_row(
            "SELECT seq,at,title,body FROM revisions WHERE note_id=?1 AND seq=?2",
            params![note, seq],
            |r| {
                Ok(Revision {
                    seq: r.get(0)?,
                    at: r.get(1)?,
                    title: r.get(2)?,
                    body: r.get(3)?,
                })
            },
        )?)
    }
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute("INSERT INTO settings VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value])?;
        Ok(())
    }
    pub fn add_attachment(&self, title: &str, data: &[u8]) -> Result<String> {
        if data.len() as u64 > MAX_ATTACHMENT || title.len() > 1024 {
            return Err(Error::Invalid("attachment too large"));
        }
        let extension = Path::new(title)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if extension.len() > 32 || !extension.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::Invalid("attachment extension"));
        }
        let id = id();
        let at = crate::exchange::iso(crate::now())?;
        let extra = serde_json::json!({ "created_time": at, "updated_time": at, "blob_updated_time": crate::now().to_string() }).to_string();
        self.conn.execute(
            "INSERT INTO resources VALUES(?1,?2,?3,'application/octet-stream',?4,?5)",
            params![id, title, extension, data, extra],
        )?;
        Ok(id)
    }
    pub fn attachments(&self, body: &str) -> Result<Vec<Attachment>> {
        let mut s = self.conn.prepare(
            "SELECT id,title,extension,mime,data,extra FROM resources WHERE instr(?1,':/' || id)>0",
        )?;
        Ok(s.query_map([body], |r| {
            Ok(Attachment {
                id: r.get(0)?,
                title: r.get(1)?,
                extension: r.get(2)?,
                mime: r.get(3)?,
                data: r.get(4)?,
                extra: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn attachment_headers(&self, body: &str) -> Result<Vec<Attachment>> {
        let mut s = self.conn.prepare(
            "SELECT id,title,extension,mime,extra FROM resources WHERE instr(?1,':/' || id)>0",
        )?;
        Ok(s.query_map([body], |r| {
            Ok(Attachment {
                id: r.get(0)?,
                title: r.get(1)?,
                extension: r.get(2)?,
                mime: r.get(3)?,
                data: vec![],
                extra: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn attachment_data(&self, id: &str) -> Result<Vec<u8>> {
        Ok(self
            .conn
            .query_row("SELECT data FROM resources WHERE id=?1", [id], |r| r.get(0))?)
    }
    pub fn backup(&self, destination: &Path) -> Result<()> {
        let key = self.key.as_ref().ok_or(Error::EncryptionRequired)?;
        let parent = destination
            .parent()
            .ok_or(Error::Invalid("backup destination"))?;
        let temp = crate::vault::encrypted_copy(&self.conn, parent, key.as_str())?;
        temp.persist_noclobber(destination)
            .map_err(|e| Error::Io(e.error))?;
        Ok(())
    }
    pub fn backup_with_password(&self, destination: &Path, password: &str) -> Result<()> {
        if !self.is_encrypted() {
            return Err(Error::EncryptionRequired);
        }
        crate::vault::validate_new_password(password)?;
        let parent = destination
            .parent()
            .ok_or(Error::Invalid("backup destination"))?;
        let temp = crate::vault::encrypted_copy(&self.conn, parent, password)?;
        temp.persist_noclobber(destination)
            .map_err(|e| Error::Io(e.error))?;
        Ok(())
    }
    pub fn change_password(&mut self, current: &str, new: &str) -> Result<()> {
        crate::vault::validate_new_password(new)?;
        let path = self.path.as_ref().ok_or(Error::EncryptionRequired)?;
        let check = crate::vault::open_database(path, current)?;
        drop(check);
        crate::vault::checkpoint(&self.conn)?;
        crate::vault::integrity(&self.conn)?;
        crate::vault::set_password_pragma(&self.conn, "rekey", new)?;
        self.key = Some(crate::vault::Secret::new(new));
        crate::vault::integrity(&self.conn)?;
        self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(())
    }
    pub fn restore_backup(&mut self, source: &Path, password: &str) -> Result<()> {
        let path = self.path.as_ref().ok_or(Error::EncryptionRequired)?.clone();
        let key = self.key.as_ref().ok_or(Error::EncryptionRequired)?;
        let input = crate::vault::open_database(source, password)?;
        crate::vault::integrity(&input)?;
        let version: i64 = input.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if !(1..=2).contains(&version) {
            return Err(Error::Invalid("unsupported backup version"));
        }
        crate::vault::validate_schema(&input)?;
        let temp = crate::vault::encrypted_copy_from_path(
            source,
            password,
            path.parent().ok_or(Error::Invalid("profile path"))?,
            key.as_str(),
        )?;
        drop(input);
        // Keep an encrypted recovery copy, never a plaintext rollback file.
        let recovery = path.with_file_name(format!("recovery-{}.sqlite3", crate::id()));
        self.backup(&recovery)?;
        crate::vault::checkpoint(&self.conn)?;
        let previous = std::mem::replace(&mut self.conn, Connection::open_in_memory()?);
        drop(previous);
        let result = temp.persist(&path).map_err(|e| Error::Io(e.error));
        let conn = Connection::open(&path)?;
        crate::vault::key_connection(&conn, key.as_str())?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY; PRAGMA secure_delete=ON;")?;
        self.conn = Self::initialize(conn, None)?.conn;
        result?;
        Ok(())
    }
    pub fn save_recovery_draft(&self, note: &Note, tags: &str, due: &str) -> Result<()> {
        let data = zeroize::Zeroizing::new(
            serde_json::to_string(&(note, tags, due))
                .map_err(|_| Error::Invalid("draft serialization"))?,
        );
        if data.len() > 32 * 1024 * 1024 {
            return Err(Error::Invalid("draft too large"));
        }
        self.conn.execute("INSERT INTO drafts VALUES(?1,?2) ON CONFLICT(note_id) DO UPDATE SET data=excluded.data",params![note.id,data.as_str()])?;
        Ok(())
    }
    pub fn recovery_draft(&self, id: &str) -> Result<Option<(Note, String, String)>> {
        let data: Option<String> = self
            .conn
            .query_row("SELECT data FROM drafts WHERE note_id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        data.map(|d| serde_json::from_str(&d).map_err(|_| Error::Invalid("recovery draft")))
            .transpose()
    }
    pub fn first_draft_id(&self) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT note_id FROM drafts ORDER BY rowid LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub(crate) fn recovery_snapshot(&self) -> Result<()> {
        let path = self.path.as_ref().ok_or(Error::EncryptionRequired)?;
        self.backup(&path.with_file_name(format!("recovery-{}.sqlite3", crate::id())))
    }
    pub(crate) fn touch_notebook(&self, id: &str) -> Result<()> {
        let stored: Option<String> = self
            .conn
            .query_row("SELECT props FROM item_meta WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let mut props: std::collections::BTreeMap<String, String> = stored
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        props.insert("updated_time".into(), crate::exchange::iso(now())?);
        self.conn.execute("INSERT INTO item_meta VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET props=excluded.props",params![id,serde_json::to_string(&props).map_err(|_|Error::Invalid("notebook metadata"))?])?;
        Ok(())
    }
    pub(crate) fn all_notes(&self) -> Result<Vec<Note>> {
        let mut s = self.conn.prepare("SELECT * FROM notes ORDER BY id")?;
        Ok(s.query_map([], note_row)?
            .collect::<rusqlite::Result<_>>()?)
    }
}
