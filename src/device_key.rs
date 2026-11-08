//! Device-local SQLCipher key. No account password is used as a database encryption key.
//! Native OS stores only; unsupported platforms fail closed instead of using a mock/file store.
use crate::{
    Error, Result, id,
    store::Store,
    vault::{self, ProfileState},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;
const SERVICE: &str = "CarnetRust.LocalVault.v1";
const MARKER: &str = "device-key-id";

trait Secrets {
    fn get(&self, id: &str) -> Result<Option<Zeroizing<String>>>;
    fn set(&self, id: &str, key: &str) -> Result<()>;
    fn delete(&self, id: &str) -> Result<()>;
}
struct Native;
impl Secrets for Native {
    fn get(&self, id: &str) -> Result<Option<Zeroizing<String>>> {
        supported()?;
        let entry = keyring::Entry::new(SERVICE, id)
            .map_err(|_| Error::Invalid("OS credential store unavailable"))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(Error::Invalid(
                "OS credential store is locked or unavailable",
            )),
        }
    }
    fn set(&self, id: &str, key: &str) -> Result<()> {
        supported()?;
        keyring::Entry::new(SERVICE, id)
            .and_then(|e| e.set_password(key))
            .map_err(|_| Error::Invalid("could not save key in OS credential store"))
    }
    fn delete(&self, id: &str) -> Result<()> {
        supported()?;
        match keyring::Entry::new(SERVICE, id).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(Error::Invalid("could not remove OS vault key")),
        }
    }
}
fn supported() -> Result<()> {
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        Ok(())
    } else {
        Err(Error::Invalid(
            "device encryption requires Windows Credential Manager or macOS Keychain; choose a master password",
        ))
    }
}
pub fn is_device_managed(dir: &Path) -> bool {
    dir.join(MARKER).is_file()
}
fn identity(dir: &Path, create: bool) -> Result<String> {
    let path = dir.join(MARKER);
    if path.exists() {
        let data = std::fs::read(&path)?;
        let value =
            std::str::from_utf8(&data).map_err(|_| Error::Invalid("device key identity"))?;
        if !crate::store::valid_id(value) {
            return Err(Error::Invalid("device key identity"));
        }
        return Ok(value.to_owned());
    }
    if !create {
        return Err(Error::Invalid("device key identity missing"));
    }
    std::fs::create_dir_all(dir)?;
    let value = id();
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(value.as_bytes())?;
    file.sync_all()?;
    Ok(value)
}
fn generate() -> Zeroizing<String> {
    Zeroizing::new(format!("{}{}{}", id(), id(), id()))
}
fn read_key(dir: &Path, secrets: &impl Secrets, create: bool) -> Result<Zeroizing<String>> {
    let identity = identity(dir, create)?;
    if let Some(key) = secrets.get(&identity)? {
        vault::validate_new_password(key.as_str())?;
        return Ok(key);
    }
    // Never generate a replacement key for an existing encrypted database.
    if !create || vault::profile_state(dir)? == ProfileState::Encrypted {
        return Err(Error::Invalid(
            "this device has no vault key; restore a portable backup or use the existing master password",
        ));
    }
    let key = generate();
    secrets.set(&identity, key.as_str())?;
    let check = secrets
        .get(&identity)?
        .ok_or(Error::Invalid("OS vault key could not be verified"))?;
    if check.as_str() != key.as_str() {
        return Err(Error::Invalid("OS vault key verification failed"));
    }
    Ok(key)
}
pub fn open(dir: &Path) -> Result<Store> {
    open_with(dir, &Native)
}
fn open_with(dir: &Path, secrets: &impl Secrets) -> Result<Store> {
    let create = vault::profile_state(dir)? != ProfileState::Encrypted;
    let key = read_key(dir, secrets, create)?;
    let store = Store::open(dir, key.as_str())?;
    if create {
        store.set_setting("auto_lock_minutes", "0")?;
    }
    Ok(store)
}
pub fn enable(store: &mut Store, current: &str) -> Result<()> {
    enable_with(store, current, &Native)
}
fn enable_with(store: &mut Store, current: &str, secrets: &impl Secrets) -> Result<()> {
    let dir = store
        .profile_dir()
        .ok_or(Error::EncryptionRequired)?
        .to_owned();
    if is_device_managed(&dir) {
        return Err(Error::Invalid("device encryption is already enabled"));
    }
    drop(vault::open_database(&vault::database_path(&dir), current)?);
    let identity = identity(&dir, true)?;
    let key = generate();
    let change = (|| {
        secrets.set(&identity, key.as_str())?;
        let check = secrets
            .get(&identity)?
            .ok_or(Error::Invalid("OS vault key missing"))?;
        if check.as_str() != key.as_str() {
            return Err(Error::Invalid("OS vault key verification failed"));
        }
        store.change_password(current, key.as_str())
    })();
    if let Err(e) = change {
        // This entry and marker were created by this operation, before changing the key.
        let _ = std::fs::remove_file(dir.join(MARKER));
        let _ = secrets.delete(&identity);
        return Err(e);
    }
    store.set_setting("auto_lock_minutes", "0")?;
    Ok(())
}
pub fn set_master(store: &mut Store, current: &str, new: &str) -> Result<()> {
    set_master_with(store, current, new, &Native)
}
fn set_master_with(
    store: &mut Store,
    current: &str,
    new: &str,
    secrets: &impl Secrets,
) -> Result<()> {
    let dir: PathBuf = store.profile_dir().ok_or(Error::EncryptionRequired)?.into();
    if is_device_managed(&dir) {
        let key = read_key(&dir, secrets, false)?;
        let identity = identity(&dir, false)?;
        // Keep a portable snapshot before retiring a device-only key.
        vault::validate_new_password(new)?;
        let recovery = dir.join(format!("recovery-master-{}.sqlite3", id()));
        store.backup_with_password(&recovery, new)?;
        store.change_password(key.as_str(), new)?;
        if let Err(e) = std::fs::remove_file(dir.join(MARKER)) {
            store.change_password(new, key.as_str())?;
            return Err(Error::Io(e));
        }
        let _ = secrets.delete(&identity);
    } else {
        store.change_password(current, new)?;
    }
    store.set_setting("auto_lock_minutes", "5")?;
    Ok(())
}
/// Portable backup of a device-managed profile, protected by an explicit export password.
pub fn portable_backup(store: &Store, dest: &Path, password: &str) -> Result<()> {
    vault::validate_new_password(password)?;
    store.backup_with_password(dest, password)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap};
    #[derive(Default)]
    struct Fake(RefCell<BTreeMap<String, String>>);
    impl Secrets for Fake {
        fn get(&self, id: &str) -> Result<Option<Zeroizing<String>>> {
            Ok(self.0.borrow().get(id).cloned().map(Zeroizing::new))
        }
        fn set(&self, id: &str, key: &str) -> Result<()> {
            self.0.borrow_mut().insert(id.into(), key.into());
            Ok(())
        }
        fn delete(&self, id: &str) -> Result<()> {
            self.0.borrow_mut().remove(id);
            Ok(())
        }
    }
    #[test]
    fn device_key_persists_offline_and_missing_key_never_replaces_data() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Fake::default();
        let mut store = open_with(dir.path(), &secrets).unwrap();
        let book = store.create_notebook("Private", None).unwrap();
        let n = store.create_note(&book, "Offline note", false).unwrap();
        drop(store);
        let store = open_with(dir.path(), &secrets).unwrap();
        assert_eq!(store.note(&n.id).unwrap().title, "Offline note");
        drop(store);
        let path = vault::database_path(dir.path());
        let before = std::fs::read(&path).unwrap();
        assert!(open_with(dir.path(), &Fake::default()).is_err());
        assert_eq!(std::fs::read(path).unwrap(), before);
        for f in std::fs::read_dir(dir.path()).unwrap() {
            let data = std::fs::read(f.unwrap().path()).unwrap();
            for key in secrets.0.borrow().values() {
                assert!(!data.windows(key.len()).any(|b| b == key.as_bytes()));
            }
        }
    }
    #[test]
    fn portable_backup_can_be_restored_on_another_device_without_os_key() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Fake::default();
        let mut store = open_with(&dir.path().join("device"), &secrets).unwrap();
        let book = store.create_notebook("Private", None).unwrap();
        let n = store.create_note(&book, "Portable note", false).unwrap();
        let backup = dir.path().join("portable.sqlite3");
        portable_backup(&store, &backup, "Portable backup passphrase").unwrap();
        let mut other = Store::open(&dir.path().join("other"), "Other device passphrase").unwrap();
        other
            .restore_backup(&backup, "Portable backup passphrase")
            .unwrap();
        assert_eq!(other.note(&n.id).unwrap().title, "Portable note");
        assert!(
            other
                .restore_backup(&backup, "Incorrect passphrase")
                .is_err()
        );
    }
    #[test]
    fn master_system_master_transitions_preserve_notes_and_reject_wrong_password() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Fake::default();
        let mut store = Store::open(dir.path(), "Original master passphrase").unwrap();
        let book = store.create_notebook("Private", None).unwrap();
        let note = store.create_note(&book, "Keep this note", false).unwrap();
        assert!(enable_with(&mut store, "Incorrect current password", &secrets).is_err());
        assert!(!is_device_managed(dir.path()));
        enable_with(&mut store, "Original master passphrase", &secrets).unwrap();
        assert!(is_device_managed(dir.path()));
        drop(store);
        let mut store = open_with(dir.path(), &secrets).unwrap();
        assert_eq!(store.note(&note.id).unwrap().title, "Keep this note");
        set_master_with(&mut store, "", "New master passphrase", &secrets).unwrap();
        assert!(!is_device_managed(dir.path()));
        drop(store);
        assert!(Store::open(dir.path(), "Original master passphrase").is_err());
        let store = Store::open(dir.path(), "New master passphrase").unwrap();
        assert_eq!(store.note(&note.id).unwrap().title, "Keep this note");
    }
}
