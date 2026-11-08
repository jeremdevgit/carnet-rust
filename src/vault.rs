//! SQLCipher 4 defaults, no application-specific encryption format or key derivation.
//! Passphrases are never saved in settings, environment variables or command arguments.
use crate::{Error, Result};
use rusqlite::{Connection, OpenFlags, params};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileState {
    New,
    Plaintext,
    Encrypted,
}

pub fn profile_state(dir: &Path) -> Result<ProfileState> {
    let path = dir.join("notes.sqlite3");
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            check_orphaned_journal(dir)?;
            return Ok(ProfileState::New);
        }
        Err(e) => return Err(e.into()),
    };
    if f.metadata()?.len() == 0 {
        check_orphaned_journal(dir)?;
        return Ok(ProfileState::New);
    }
    let mut header = [0; 16];
    f.read_exact(&mut header)?;
    Ok(if &header == b"SQLite format 3\0" {
        ProfileState::Plaintext
    } else {
        ProfileState::Encrypted
    })
}
fn check_orphaned_journal(dir: &Path) -> Result<()> {
    for name in ["notes.sqlite3-wal", "notes.sqlite3-journal"] {
        match std::fs::metadata(dir.join(name)) {
            Ok(m) if m.len() > 0 => {
                return Err(Error::Invalid(
                    "database is missing but a recovery journal exists; restore a backup",
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub fn validate_new_password(password: &str) -> Result<()> {
    if password.chars().count() < 12 || password.len() > 1024 || password.contains('\0') {
        return Err(Error::PasswordPolicy);
    }
    Ok(())
}
pub(crate) fn key_connection(conn: &Connection, password: &str) -> Result<()> {
    if password.is_empty() || password.len() > 1024 || password.contains('\0') {
        return Err(Error::Unlock);
    }
    let version: String = conn.query_row("PRAGMA cipher_version", [], |r| r.get(0))?;
    if !version.starts_with("4.") {
        return Err(Error::Invalid("SQLCipher 4 required"));
    }
    conn.execute_batch(
        "PRAGMA cipher_memory_security=ON; PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF;",
    )?;
    set_password_pragma(conn, "key", password)?;
    conn.execute_batch("PRAGMA cipher_compatibility=4; PRAGMA cipher_memory_security=ON; PRAGMA temp_store=MEMORY;")?;
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|_| Error::Unlock)?;
    Ok(())
}
pub(crate) fn set_password_pragma(conn: &Connection, name: &str, password: &str) -> Result<()> {
    if !matches!(name, "key" | "rekey") {
        return Err(Error::Invalid("key operation"));
    }
    // PRAGMA does not support bound parameters. Escape into a zeroized buffer,
    // rather than leave a second passphrase allocation in rusqlite's SQL builder.
    let mut sql = Zeroizing::new(format!("PRAGMA {name}='"));
    for c in password.chars() {
        sql.push(c);
        if c == '\'' {
            sql.push('\'');
        }
    }
    sql.push_str("';");
    conn.execute_batch(sql.as_str())?;
    Ok(())
}
pub fn open_database(path: &Path, password: &str) -> Result<Connection> {
    // Used to read exports/backups: never create a missing source by accident.
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    key_connection(&conn, password)?;
    Ok(conn)
}
pub(crate) fn integrity(conn: &Connection) -> Result<()> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if result != "ok" {
        return Err(Error::Invalid("database integrity"));
    }
    let mut statement = conn.prepare("PRAGMA cipher_integrity_check")?;
    // SQLCipher returns no rows when every encrypted page passes authentication.
    if statement.query([])?.next()?.is_some() {
        return Err(Error::Invalid("encrypted page integrity"));
    }
    Ok(())
}
fn path_text(path: &Path) -> Result<&str> {
    path.to_str().ok_or(Error::Invalid("path is not UTF-8"))
}

/// Export into an encrypted temporary file. SQLCipher supports mixed-mode migration
/// through sqlcipher_export, whereas SQLite's backup API cannot convert plaintext.
pub(crate) fn encrypted_copy(
    source: &Connection,
    parent: &Path,
    password: &str,
) -> Result<tempfile::NamedTempFile> {
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let version: i64 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
    source.execute(
        "ATTACH DATABASE ?1 AS vault_copy KEY ?2",
        params![path_text(temp.path())?, password],
    )?;
    let result: Result<()> = (|| {
        source.query_row("SELECT sqlcipher_export('vault_copy')", [], |_| Ok(()))?;
        source.pragma_update(Some("vault_copy"), "user_version", version)?;
        Ok(())
    })();
    let detached = source.execute_batch("DETACH DATABASE vault_copy");
    result?;
    detached?;
    let check = open_database(temp.path(), password)?;
    integrity(&check)?;
    drop(check);
    temp.as_file().sync_all()?;
    Ok(temp)
}
pub(crate) fn migrate_plaintext(path: &Path, password: &str) -> Result<()> {
    validate_new_password(password)?;
    let source = Connection::open(path)?;
    source.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF;")?;
    // Wrong schema must not be replaced: only this application's own profile.
    let version: i64 = source.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if !(1..=2).contains(&version) {
        return Err(Error::Invalid("unrecognized legacy profile"));
    }
    validate_schema(&source)?;
    let temp = encrypted_copy(
        &source,
        path.parent().ok_or(Error::Invalid("profile path"))?,
        password,
    )?;
    checkpoint(&source)?;
    drop(source);
    // Atomic replacement, with no plaintext backup/temp copy. On any earlier error
    // the original database remains the only active profile.
    temp.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
pub(crate) fn validate_schema(conn: &Connection) -> Result<()> {
    for (table, columns) in [
        ("notebooks", "id,parent_id,title"),
        (
            "notes",
            "id,notebook_id,title,body,is_todo,completed,due,created,updated,deleted,version,extra",
        ),
        ("resources", "id,title,extension,mime,data,extra"),
        ("tags", "id,title"),
        ("note_tags", "note_id,tag_id"),
        ("revisions", "seq,note_id,at,title,body"),
        ("settings", "key,value"),
        ("search", "note_id,title,body"),
    ] {
        let kind: Option<String> = conn
            .query_row(
                "SELECT type FROM sqlite_master WHERE name=?1",
                [table],
                |r| r.get(0),
            )
            .optional()?;
        if kind.as_deref() != Some("table") {
            return Err(Error::Invalid("unrecognized profile schema"));
        }
        conn.prepare(&format!("SELECT {columns} FROM {table} LIMIT 0"))?;
    }
    Ok(())
}
pub(crate) fn encrypted_copy_from_path(
    source: &Path,
    source_password: &str,
    parent: &Path,
    password: &str,
) -> Result<tempfile::NamedTempFile> {
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    let target = Connection::open(temp.path())?;
    key_connection(&target, password)?;
    target.execute(
        "ATTACH DATABASE ?1 AS restore_source KEY ?2",
        params![path_text(source)?, source_password],
    )?;
    let result: Result<()> = (|| {
        let version: i64 =
            target.pragma_query_value(Some("restore_source"), "user_version", |r| r.get(0))?;
        target.query_row(
            "SELECT sqlcipher_export('main','restore_source')",
            [],
            |_| Ok(()),
        )?;
        target.pragma_update(None, "user_version", version)?;
        Ok(())
    })();
    let detached = target.execute_batch("DETACH DATABASE restore_source");
    result?;
    detached?;
    integrity(&target)?;
    drop(target);
    temp.as_file().sync_all()?;
    Ok(temp)
}
pub(crate) fn checkpoint(conn: &Connection) -> Result<()> {
    let busy: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
    if busy != 0 {
        return Err(Error::Invalid("profile is busy; no file was replaced"));
    }
    conn.execute_batch("PRAGMA journal_mode=DELETE;")?;
    Ok(())
}
use rusqlite::OptionalExtension;

/// Secret is zeroized when the Store is dropped (manual or idle lock).
pub(crate) struct Secret(pub Zeroizing<String>);
impl Secret {
    pub fn new(password: &str) -> Self {
        Self(Zeroizing::new(password.to_string()))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
pub(crate) fn database_path(dir: &Path) -> PathBuf {
    dir.join("notes.sqlite3")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    const PASSWORD: &str = "Demonstration passphrase ' 日本語";
    const OTHER: &str = "Another demonstration passphrase";
    fn create(dir: &Path) -> (Store, String) {
        let mut store = Store::open(dir, PASSWORD).unwrap();
        let book = store
            .create_notebook("Private notebook sentinel", None)
            .unwrap();
        let mut note = store
            .create_note(&book, "Private title sentinel", false)
            .unwrap();
        note.body = "Confidential body sentinel".into();
        store.save(&mut note, "private tag sentinel").unwrap();
        store
            .add_attachment(
                "private-attachment.txt",
                b"Confidential attachment sentinel",
            )
            .unwrap();
        store
            .save_recovery_draft(&note, "private draft tag", "2030-12-25")
            .unwrap();
        (store, note.id)
    }
    fn no_cleartext(dir: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if !p.is_file() {
                continue;
            }
            // Windows applies byte-range locks to SQLite's shared-memory file.
            // The encrypted database and WAL remain readable for this assertion.
            if cfg!(windows) && p.to_string_lossy().ends_with("-shm") {
                continue;
            }
            let data = std::fs::read(&p).unwrap();
            for secret in [
                "Private notebook sentinel",
                "Private title sentinel",
                "Confidential body sentinel",
                "Confidential attachment sentinel",
                "private draft tag",
                PASSWORD,
            ] {
                assert!(
                    !data.windows(secret.len()).any(|b| b == secret.as_bytes()),
                    "plaintext leaked to {}",
                    p.file_name().unwrap().to_string_lossy()
                );
            }
        }
    }
    #[test]
    fn notes_attachments_search_history_drafts_and_wal_are_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let (store, id) = create(dir.path());
        assert!(store.is_encrypted());
        no_cleartext(dir.path());
        let bytes = std::fs::read(database_path(dir.path())).unwrap();
        assert_ne!(&bytes[..16], b"SQLite format 3\0");
        assert!(
            Connection::open(database_path(dir.path()))
                .unwrap()
                .query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .is_err()
        );
        drop(store);
        let store = Store::open(dir.path(), PASSWORD).unwrap();
        assert_eq!(store.note(&id).unwrap().body, "Confidential body sentinel");
        assert_eq!(store.full_text("Confidential").unwrap(), vec![id]);
        no_cleartext(dir.path());
    }
    #[test]
    fn wrong_password_does_not_change_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = create(dir.path());
        drop(store);
        let path = database_path(dir.path());
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(Store::open(dir.path(), OTHER), Err(Error::Unlock)));
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert!(Store::open(dir.path(), PASSWORD).is_ok());
    }
    #[test]
    fn authenticated_corruption_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = create(dir.path());
        drop(store);
        let path = database_path(dir.path());
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[120] ^= 0x40;
        std::fs::write(&path, bytes).unwrap();
        assert!(Store::open(dir.path(), PASSWORD).is_err());
    }
    #[test]
    fn password_change_invalidates_old_key_but_keeps_contents() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, id) = create(dir.path());
        assert!(
            store
                .change_password(OTHER, "Wrong current password must not work")
                .is_err()
        );
        assert!(store.change_password(PASSWORD, "short").is_err());
        store.change_password(PASSWORD, OTHER).unwrap();
        drop(store);
        assert!(Store::open(dir.path(), PASSWORD).is_err());
        let store = Store::open(dir.path(), OTHER).unwrap();
        assert_eq!(store.note(&id).unwrap().title, "Private title sentinel");
    }
    #[test]
    fn backups_are_encrypted_and_restored_with_the_destination_key() {
        let dir = tempfile::tempdir().unwrap();
        let (source, id) = create(&dir.path().join("source"));
        let path = dir.path().join("backup.sqlite3");
        source.backup(&path).unwrap();
        assert!(open_database(&path, OTHER).is_err());
        let check = open_database(&path, PASSWORD).unwrap();
        integrity(&check).unwrap();
        drop(check);
        let destdir = dir.path().join("destination");
        let mut dest = Store::open(&destdir, OTHER).unwrap();
        let original = dest.create_notebook("Before restore", None).unwrap();
        assert!(dest.restore_backup(&path, OTHER).is_err());
        assert_eq!(dest.notebooks().unwrap()[0].id, original);
        dest.restore_backup(&path, PASSWORD).unwrap();
        assert_eq!(dest.note(&id).unwrap().body, "Confidential body sentinel");
        drop(dest);
        assert!(Store::open(&destdir, PASSWORD).is_err());
        let dest = Store::open(&destdir, OTHER).unwrap();
        assert!(dest.note(&id).is_ok());
        let copies: Vec<_> = std::fs::read_dir(&destdir)
            .unwrap()
            .filter_map(|f| {
                let p = f.unwrap().path();
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("recovery-")
                    .then_some(p)
            })
            .collect();
        assert_eq!(copies.len(), 1);
        let recovery = open_database(&copies[0], OTHER).unwrap();
        let count: i64 = recovery
            .query_row("SELECT count(*) FROM notebooks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        no_cleartext(&destdir);
    }
    fn legacy(dir: &Path) -> String {
        std::fs::create_dir_all(dir).unwrap();
        let mut source = Store::memory().unwrap();
        let book = source.create_notebook("Legacy notebook", None).unwrap();
        let mut note = source.create_note(&book, "Legacy note", false).unwrap();
        note.body = "Confidential body sentinel".into();
        source.save(&mut note, "legacy").unwrap();
        source
            .conn
            .execute(
                "ATTACH DATABASE ?1 AS legacy KEY ''",
                [database_path(dir).to_str().unwrap()],
            )
            .unwrap();
        source
            .conn
            .query_row("SELECT sqlcipher_export('legacy')", [], |_| Ok(()))
            .unwrap();
        source
            .conn
            .execute_batch("PRAGMA legacy.user_version=1; DETACH DATABASE legacy;")
            .unwrap();
        note.id
    }
    #[test]
    fn plaintext_migration_preserves_notes_and_leaves_no_plaintext_copy() {
        let dir = tempfile::tempdir().unwrap();
        let id = legacy(dir.path());
        assert_eq!(profile_state(dir.path()).unwrap(), ProfileState::Plaintext);
        let store = Store::open(dir.path(), PASSWORD).unwrap();
        assert!(store.is_encrypted());
        assert_eq!(store.note(&id).unwrap().body, "Confidential body sentinel");
        assert_eq!(store.tags(&id).unwrap(), "legacy");
        drop(store);
        assert_eq!(profile_state(dir.path()).unwrap(), ProfileState::Encrypted);
        no_cleartext(dir.path());
        assert_eq!(
            std::fs::read_dir(dir.path())
                .unwrap()
                .filter(|f| f
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .ends_with("sqlite3"))
                .count(),
            1
        );
    }
    #[test]
    fn failed_migration_does_not_replace_plaintext_original() {
        let dir = tempfile::tempdir().unwrap();
        let id = legacy(dir.path());
        let before = std::fs::read(database_path(dir.path())).unwrap();
        assert!(Store::open(dir.path(), "short").is_err());
        assert_eq!(before, std::fs::read(database_path(dir.path())).unwrap());
        assert!(Store::open(dir.path(), PASSWORD).unwrap().note(&id).is_ok());
    }
    #[test]
    fn incompatible_plaintext_database_is_not_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(dir.path());
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE unrelated(data TEXT); INSERT INTO unrelated VALUES('Keep me');",
        )
        .unwrap();
        drop(db);
        let before = std::fs::read(&path).unwrap();
        assert!(Store::open(dir.path(), PASSWORD).is_err());
        assert_eq!(before, std::fs::read(path).unwrap());
    }
    #[test]
    fn passphrase_boundaries_and_unicode_are_supported() {
        assert!(validate_new_password("12345678901").is_err());
        assert!(validate_new_password(&"x".repeat(1025)).is_err());
        assert!(validate_new_password("abcdefghijkl\0").is_err());
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), PASSWORD).unwrap();
        assert!(store.is_encrypted());
    }
    #[test]
    fn malformed_legacy_schema_is_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = database_path(dir.path());
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE notes(id TEXT); PRAGMA user_version=1;")
            .unwrap();
        drop(db);
        let before = std::fs::read(&path).unwrap();
        assert!(Store::open(dir.path(), PASSWORD).is_err());
        assert_eq!(before, std::fs::read(path).unwrap());
    }
    #[test]
    fn orphaned_journal_is_never_overwritten_by_new_vault_creation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("notes.sqlite3-wal"),
            b"Recovery journal sentinel",
        )
        .unwrap();
        assert!(profile_state(dir.path()).is_err());
        assert!(Store::open(dir.path(), PASSWORD).is_err());
        assert!(!database_path(dir.path()).exists());
    }
}
