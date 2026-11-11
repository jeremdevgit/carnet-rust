//! Conservative sync-format v3 client. Never disables encryption on an existing target.
//! Local SQLCipher encryption is distinct from Joplin's E2EE protocol.
use crate::{
    Error, Result, e2ee,
    exchange::{self, Item},
    id, now,
    store::{MAX_ATTACHMENT, Note, Store, valid_id, validate_note},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};
const MAX_TOTAL: usize = 256 * 1024 * 1024;
const ABSENT: &str = "absent";
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    #[default]
    JoplinServer,
    Nextcloud,
    WebDav,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub target: Target,
    #[serde(default)]
    pub automatic: bool,
    pub server: String,
    pub email: String,
    pub password: String,
    pub client_id: String,
    /// Joplin end-to-end encryption password (master password). Empty: plain target only.
    #[serde(default)]
    pub e2ee_password: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            target: Target::Nextcloud,
            automatic: true,
            server: String::new(),
            email: String::new(),
            password: String::new(),
            client_id: id(),
            e2ee_password: String::new(),
        }
    }
}
impl Drop for Config {
    fn drop(&mut self) {
        self.password.zeroize();
        self.email.zeroize();
        self.e2ee_password.zeroize();
    }
}
#[derive(Debug)]
pub enum Outcome {
    Complete {
        uploaded: usize,
        downloaded: usize,
        deleted: usize,
    },
    Conflicts(usize),
}
#[derive(Clone, Copy)]
pub enum Resolution {
    Local,
    Remote,
}
#[derive(Debug)]
pub struct Conflict {
    pub path: String,
    pub local_title: String,
    pub remote_title: String,
}
#[derive(Clone)]
struct Record {
    data: Zeroizing<Vec<u8>>,
    hash: String,
    kind: i64,
}
fn record(path: &str, data: Vec<u8>) -> Result<Record> {
    let (bytes, kind) = if path.ends_with(".md") {
        let item = parse(&data)?;
        if path != format!("{}.md", item.get("id")) {
            return Err(Error::Invalid("sync item filename mismatch"));
        }
        let bytes = serde_json::to_vec(&(&item.title, &item.body, &item.props))
            .map_err(|_| Error::Invalid("sync record"))?;
        (bytes, item.number("type_")?)
    } else {
        (data.clone(), 0)
    };
    Ok(Record {
        data: Zeroizing::new(data),
        hash: hex::encode(Sha256::digest(&bytes)),
        kind,
    })
}
fn parse(data: &[u8]) -> Result<Item> {
    exchange::parse_item(
        std::str::from_utf8(data).map_err(|_| Error::Invalid("sync item is not UTF-8"))?,
    )
}
fn hash(record: Option<&Record>) -> &str {
    record.map_or(ABSENT, |r| r.hash.as_str())
}
pub fn validate_server(server: &str) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(server).map_err(|_| Error::Invalid("server URL"))?;
    let host = url.host_str().ok_or(Error::Invalid("server host"))?;
    let local = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Invalid(
            "HTTPS required (HTTP is allowed only for local testing)",
        ));
    }
    if !url.path().ends_with('/') {
        let p = format!("{}/", url.path());
        url.set_path(&p);
    }
    Ok(url)
}
/// Nextcloud accepts a server URL or an explicit DAV directory URL.
pub fn endpoint(config: &Config) -> Result<reqwest::Url> {
    let mut url = validate_server(&config.server)?;
    if config.target == Target::Nextcloud && !url.path().contains("/remote.php/") {
        if config.email.is_empty()
            || config.email.contains(['/', '\\', '\0'])
            || matches!(config.email.as_str(), "." | "..")
        {
            return Err(Error::Invalid("Nextcloud username"));
        }
        url.path_segments_mut()
            .map_err(|_| Error::Invalid("Nextcloud URL"))?
            .pop_if_empty()
            .extend([
                "remote.php",
                "dav",
                "files",
                config.email.as_str(),
                "Joplin",
                "",
            ]);
    }
    if config.target != Target::JoplinServer && url.path().trim_matches('/').is_empty() {
        return Err(Error::Invalid("choose a dedicated WebDAV notes directory"));
    }
    Ok(url)
}
pub fn save_config(store: &Store, config: &Config) -> Result<()> {
    if !store.is_encrypted() {
        return Err(Error::EncryptionRequired);
    }
    endpoint(config)?;
    if config.email.is_empty()
        || config.email.len() > 1024
        || config.password.len() > 4096
        || config.e2ee_password.len() > 4096
        || !valid_id(&config.client_id)
    {
        return Err(Error::Invalid("sync account settings"));
    }
    let text = Zeroizing::new(
        serde_json::to_string(config).map_err(|_| Error::Invalid("sync configuration"))?,
    );
    store.set_setting("sync_account", text.as_str())
}
pub fn load_config(store: &Store) -> Result<Config> {
    match store.setting("sync_account")? {
        Some(text) => {
            let text = Zeroizing::new(text);
            serde_json::from_str(text.as_str()).map_err(|_| Error::Invalid("sync configuration"))
        }
        None => Ok(Config::default()),
    }
}
pub fn conflicts(store: &Store) -> Result<Vec<Conflict>> {
    let mut s = store
        .conn
        .prepare("SELECT path,local,remote FROM sync_conflicts ORDER BY path")?;
    Ok(s.query_map([], |r| {
        let path: String = r.get(0)?;
        let local: Option<Vec<u8>> = r.get(1)?;
        let remote: Option<Vec<u8>> = r.get(2)?;
        let title = |bytes: Option<Vec<u8>>| {
            bytes
                .and_then(|b| parse(&b).ok())
                .map_or(String::new(), |i| i.title)
        };
        Ok(Conflict {
            path,
            local_title: title(local),
            remote_title: title(remote),
        })
    })?
    .collect::<rusqlite::Result<_>>()?)
}
pub fn resolve(store: &mut Store, path: &str, choice: Resolution) -> Result<()> {
    let tx = store.conn.transaction()?;
    let (local, remote): (String, String) = tx.query_row(
        "SELECT local_hash,remote_hash FROM sync_conflicts WHERE path=?1",
        [path],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let baseline = match choice {
        Resolution::Local => remote,
        Resolution::Remote => local,
    };
    tx.execute("INSERT INTO sync_baseline VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET hash=excluded.hash",params![path,baseline])?;
    tx.execute("DELETE FROM sync_conflicts WHERE path=?1", [path])?;
    tx.commit()?;
    Ok(())
}
fn meta(store: &Store, id: &str) -> Result<BTreeMap<String, String>> {
    let raw: Option<String> = store
        .conn
        .query_row("SELECT props FROM item_meta WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let mut props: BTreeMap<String, String> = raw.map_or(Ok(BTreeMap::new()), |s| {
        serde_json::from_str(&s).map_err(|_| Error::Invalid("item metadata"))
    })?;
    props
        .entry("created_time".into())
        .or_insert(exchange::iso(0)?);
    props
        .entry("updated_time".into())
        .or_insert(exchange::iso(0)?);
    Ok(props)
}

fn put(map: &mut BTreeMap<String, Record>, path: String, data: Vec<u8>) -> Result<()> {
    let r = record(&path, data)?;
    map.insert(path, r);
    Ok(())
}
fn snapshot(store: &Store) -> Result<BTreeMap<String, Record>> {
    let mut map = BTreeMap::new();
    for book in store.notebooks()? {
        let mut p = meta(store, &book.id)?;
        p.entry("created_time".into()).or_insert(exchange::iso(0)?);
        p.entry("updated_time".into()).or_insert(exchange::iso(0)?);
        p.extend(BTreeMap::from([
            ("id".into(), book.id.clone()),
            ("type_".into(), "2".into()),
            ("parent_id".into(), book.parent_id.unwrap_or_default()),
        ]));
        put(
            &mut map,
            format!("{}.md", book.id),
            exchange::serialize(&book.title, "", p)?,
        )?;
    }
    for n in store.all_notes()? {
        let mut p: BTreeMap<String, String> =
            serde_json::from_str(&n.extra).map_err(|_| Error::Invalid("note metadata"))?;
        let markup = match p.get("markup_language").map(String::as_str) {
            Some("2") => "2",
            _ => "1",
        };
        let completed = if n.completed {
            p.get("todo_completed")
                .and_then(|n| n.parse::<i64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(n.updated)
        } else {
            0
        };
        p.extend(BTreeMap::from([
            ("id".into(), n.id.clone()),
            ("parent_id".into(), n.notebook_id.clone()),
            ("type_".into(), "1".into()),
            ("created_time".into(), exchange::iso(n.created)?),
            ("updated_time".into(), exchange::iso(n.updated)?),
            ("is_todo".into(), i32::from(n.is_todo).to_string()),
            ("todo_completed".into(), completed.to_string()),
            ("todo_due".into(), n.due.to_string()),
            (
                "deleted_time".into(),
                if n.deleted {
                    p.get("deleted_time")
                        .and_then(|n| n.parse::<i64>().ok())
                        .filter(|n| *n > 0)
                        .unwrap_or(n.updated)
                } else {
                    0
                }
                .to_string(),
            ),
            ("encryption_applied".into(), "0".into()),
            ("encryption_cipher_text".into(), String::new()),
            ("markup_language".into(), markup.into()),
        ]));
        put(
            &mut map,
            format!("{}.md", n.id),
            exchange::serialize(&n.title, &n.body, p)?,
        )?;
    }
    let mut s = store
        .conn
        .prepare("SELECT id,title,extension,mime,data,extra FROM resources ORDER BY id")?;
    let mut rows = s.query([])?;
    while let Some(r) = rows.next()? {
        let id: String = r.get(0)?;
        let title: String = r.get(1)?;
        let ext: String = r.get(2)?;
        let mime: String = r.get(3)?;
        let data: Vec<u8> = r.get(4)?;
        let extra: String = r.get(5)?;
        let mut p: BTreeMap<String, String> =
            serde_json::from_str(&extra).map_err(|_| Error::Invalid("resource metadata"))?;
        p.entry("created_time".into()).or_insert(exchange::iso(0)?);
        p.entry("updated_time".into()).or_insert(exchange::iso(0)?);
        p.extend(BTreeMap::from([
            ("id".into(), id.clone()),
            ("type_".into(), "4".into()),
            ("file_extension".into(), ext),
            ("mime".into(), mime),
            ("size".into(), data.len().to_string()),
            ("encryption_blob_encrypted".into(), "0".into()),
            ("encryption_applied".into(), "0".into()),
            ("encryption_cipher_text".into(), String::new()),
        ]));
        put(
            &mut map,
            format!("{id}.md"),
            exchange::serialize(&title, "", p)?,
        )?;
        put(&mut map, format!(".resource/{id}"), data)?;
    }
    let mut s = store
        .conn
        .prepare("SELECT id,title FROM tags ORDER BY id")?;
    for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (id, title) = r?;
        let mut p = meta(store, &id)?;
        p.extend(BTreeMap::from([
            ("id".into(), id.clone()),
            ("type_".into(), "5".into()),
        ]));
        put(
            &mut map,
            format!("{id}.md"),
            exchange::serialize(&title, "", p)?,
        )?;
    }
    store.conn.execute("DELETE FROM relation_ids WHERE NOT EXISTS(SELECT 1 FROM note_tags nt WHERE nt.note_id=relation_ids.note_id AND nt.tag_id=relation_ids.tag_id)",[])?;
    let mut s=store.conn.prepare("SELECT nt.note_id,nt.tag_id FROM note_tags nt WHERE NOT EXISTS(SELECT 1 FROM relation_ids ri WHERE ri.note_id=nt.note_id AND ri.tag_id=nt.tag_id)")?;
    let missing = s
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (note, tag) in missing {
        store.conn.execute(
            "INSERT INTO relation_ids VALUES(?1,?2,?3)",
            params![id(), note, tag],
        )?;
    }
    let mut s = store
        .conn
        .prepare("SELECT id,note_id,tag_id FROM relation_ids ORDER BY id")?;
    for r in s.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, note, tag) = r?;
        let mut p = meta(store, &id)?;
        p.extend(BTreeMap::from([
            ("id".into(), id.clone()),
            ("type_".into(), "6".into()),
            ("note_id".into(), note),
            ("tag_id".into(), tag),
        ]));
        put(
            &mut map,
            format!("{id}.md"),
            exchange::serialize("", "", p)?,
        )?;
    }
    if map.len() > 10000 || map.values().map(|r| r.data.len()).sum::<usize>() > MAX_TOTAL {
        return Err(Error::Invalid("sync snapshot exceeds limits"));
    }
    Ok(map)
}
struct Api {
    base: reqwest::Url,
    client: reqwest::blocking::Client,
    token: Zeroizing<String>,
    user_id: String,
    client_id: String,
    cancel: Arc<AtomicBool>,
    start: Instant,
    renewed: Instant,
    dav: Option<crate::webdav::Dav>,
}
#[derive(Serialize, Deserialize)]
struct SessionCache {
    binding: String,
    token: String,
    user_id: String,
}
impl Drop for SessionCache {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}
impl Api {
    fn new(store: &Store, config: &Config, cancel: Arc<AtomicBool>) -> Result<Self> {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Invalid("synchronisation cancelled"));
        }
        static TLS: std::sync::Once = std::sync::Once::new();
        TLS.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(concat!("CarnetDeNotes/", env!("CARGO_PKG_VERSION")))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .build()
            .map_err(|_| Error::Invalid("TLS client"))?;
        let mut api = Self {
            base: endpoint(config)?,
            client,
            token: Zeroizing::new(String::new()),
            user_id: String::new(),
            client_id: config.client_id.clone(),
            cancel,
            start: Instant::now(),
            renewed: Instant::now(),
            dav: None,
        };
        if config.target != Target::JoplinServer {
            api.user_id = config.email.clone();
            api.dav = Some(crate::webdav::Dav::new(
                api.client.clone(),
                config,
                api.cancel.clone(),
            )?);
            return Ok(api);
        }
        let serialized = Zeroizing::new(
            serde_json::to_vec(&(&config.server, &config.email, &config.password))
                .map_err(|_| Error::Invalid("session binding"))?,
        );
        let binding = hex::encode(Sha256::digest(serialized.as_slice()));
        if let Some(cached) = store.setting("sync_session")? {
            let cached = Zeroizing::new(cached);
            if let Ok(mut cached) = serde_json::from_str::<SessionCache>(cached.as_str())
                && cached.binding == binding
                && !cached.token.is_empty()
            {
                api.token = Zeroizing::new(std::mem::take(&mut cached.token));
                api.user_id = std::mem::take(&mut cached.user_id);
                return Ok(api);
            }
        }
        let body=Zeroizing::new(serde_json::to_vec(&json!({"email":config.email,"password":config.password,"clientName":"Carnet de notes","clientType":1})).map_err(|_|Error::Invalid("session request"))?);
        let data = Zeroizing::new(
            api.request("POST", "api/sessions", Some(body.as_slice()), false)?
                .ok_or(Error::Invalid("session response"))?,
        );
        let mut response: Value =
            serde_json::from_slice(&data).map_err(|_| Error::Invalid("session response"))?;
        api.user_id = response["user_id"]
            .as_str()
            .ok_or(Error::Invalid("account identity"))?
            .to_string();
        let token = match response.get_mut("id").map(Value::take) {
            Some(Value::String(s)) => s,
            _ => return Err(Error::Invalid("session token")),
        };
        if token.is_empty() || token.len() > 1024 {
            return Err(Error::Invalid("session token"));
        }
        api.token = Zeroizing::new(token);
        let cached = SessionCache {
            binding,
            token: api.token.to_string(),
            user_id: api.user_id.clone(),
        };
        let serialized = Zeroizing::new(
            serde_json::to_string(&cached).map_err(|_| Error::Invalid("session cache"))?,
        );
        store.set_setting("sync_session", serialized.as_str())?;
        Ok(api)
    }
    fn check(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(Error::Invalid("synchronisation cancelled"));
        }
        if self.start.elapsed() > Duration::from_secs(120) {
            return Err(Error::Invalid("sync time limit; retry to continue"));
        }
        Ok(())
    }
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        allow_missing: bool,
    ) -> Result<Option<Vec<u8>>> {
        let url = self
            .base
            .join(path)
            .map_err(|_| Error::Invalid("API path"))?;
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| Error::Invalid("HTTP method"))?;
        let mut request = self.client.request(method, url);
        if !self.token.is_empty() {
            let mut h = reqwest::header::HeaderValue::from_str(self.token.as_str())
                .map_err(|_| Error::Invalid("session header"))?;
            h.set_sensitive(true);
            request = request.header("X-API-AUTH", h);
        }
        if let Some(body) = body {
            request = request
                .header(
                    "Content-Type",
                    if path == "api/sessions" || path == "api/locks" {
                        "application/json"
                    } else {
                        "application/octet-stream"
                    },
                )
                .body(body.to_vec());
        }
        let response = request
            .send()
            .map_err(|_| Error::Invalid("network or TLS request failed"))?;
        let status = response.status();
        if allow_missing && status.as_u16() == 404 {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(Error::SyncStatus(status.as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|s| s > MAX_ATTACHMENT + 65536)
        {
            return Err(Error::Invalid("server response too large"));
        }
        let mut data = Vec::new();
        response
            .take(MAX_ATTACHMENT + 65537)
            .read_to_end(&mut data)?;
        if data.len() as u64 > MAX_ATTACHMENT + 65536 {
            return Err(Error::Invalid("server response too large"));
        }
        Ok(Some(data))
    }
    fn lock(&mut self) -> Result<()> {
        if let Some(dav) = &mut self.dav {
            return dav.lock();
        }
        let body = serde_json::to_vec(&json!({"type":1,"clientType":1,"clientId":self.client_id}))
            .map_err(|_| Error::Invalid("lock request"))?;
        self.request("POST", "api/locks", Some(&body), false)?;
        self.renewed = Instant::now();
        Ok(())
    }
    fn tick(&mut self) -> Result<()> {
        self.check()?;
        if let Some(dav) = &mut self.dav {
            return dav.tick();
        }
        if self.renewed.elapsed() > Duration::from_secs(30) {
            self.lock()?;
        }
        Ok(())
    }
    fn get(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        self.tick()?;
        if let Some(dav) = &mut self.dav {
            return dav.get(path);
        }
        self.request(
            "GET",
            &format!("api/items/root:/{path}:/content"),
            None,
            true,
        )
    }
    fn put(&mut self, path: &str, data: &[u8]) -> Result<()> {
        self.tick()?;
        if let Some(dav) = &mut self.dav {
            return dav.put(path, data);
        }
        self.request(
            "PUT",
            &format!("api/items/root:/{path}:/content"),
            Some(data),
            false,
        )?;
        Ok(())
    }
    fn delete(&mut self, path: &str) -> Result<()> {
        self.tick()?;
        if let Some(dav) = &mut self.dav {
            return dav.delete(path);
        }
        self.request("DELETE", &format!("api/items/root:/{path}:"), None, true)?;
        Ok(())
    }
    fn list(&mut self) -> Result<Vec<String>> {
        self.tick()?;
        if let Some(dav) = &mut self.dav {
            return dav.list("");
        }
        let mut names = Vec::new();
        let mut cursor = None::<String>;
        let mut seen = BTreeSet::new();
        loop {
            self.tick()?;
            let mut url = self
                .base
                .join("api/items/root:/:/children")
                .map_err(|_| Error::Invalid("API path"))?;
            if let Some(c) = &cursor {
                url.query_pairs_mut().append_pair("cursor", c);
            }
            let relative = url
                .as_str()
                .strip_prefix(self.base.as_str())
                .ok_or(Error::Invalid("API origin"))?;
            let data = self
                .request("GET", relative, None, false)?
                .ok_or(Error::Invalid("server listing"))?;
            let response: Value =
                serde_json::from_slice(&data).map_err(|_| Error::Invalid("server listing"))?;
            for item in response["items"]
                .as_array()
                .ok_or(Error::Invalid("server listing"))?
            {
                let name = item["name"].as_str().ok_or(Error::Invalid("remote name"))?;
                if !seen.insert(name.to_string()) {
                    return Err(Error::Invalid("duplicate remote name"));
                }
                names.push(name.to_string());
                if names.len() > 10000 {
                    return Err(Error::Invalid("remote item limit"));
                }
            }
            if response["has_more"] != true {
                break;
            }
            let next = response["cursor"]
                .as_str()
                .ok_or(Error::Invalid("pagination cursor"))?
                .to_string();
            if cursor.as_ref() == Some(&next) {
                return Err(Error::Invalid("repeated pagination cursor"));
            }
            cursor = Some(next);
        }
        Ok(names)
    }
}
fn safe_remote_name(path: &str) -> bool {
    path.strip_suffix(".md").is_some_and(valid_id)
        || path.strip_prefix(".resource/").is_some_and(valid_id)
}
pub fn run(store: &mut Store, config: &Config, cancel: Arc<AtomicBool>) -> Result<Outcome> {
    if !store.is_encrypted() {
        return Err(Error::EncryptionRequired);
    }
    let result = (|| {
        let mut api = Api::new(store, config, cancel)?;
        api.lock()?;
        let result = run_locked(store, &mut api, &config.e2ee_password);
        if let Some(dav) = &mut api.dav {
            let _ = dav.release();
        } else {
            let _ = api.request(
                "DELETE",
                &format!("api/locks/1_1_{}", api.client_id),
                None,
                true,
            );
        }
        result
    })();
    if matches!(result, Err(Error::SyncStatus(401))) {
        store
            .conn
            .execute("DELETE FROM settings WHERE key='sync_session'", [])?;
    }
    result
}

fn run_locked(store: &mut Store, api: &mut Api, e2ee_password: &str) -> Result<Outcome> {
    let names = api.list()?;
    let info = api.get("info.json")?;
    let new_target = info.is_none();
    let mut keys: Option<e2ee::Keys> = None;
    let mut new_master_key = None;
    if let Some(info) = info {
        let info: Value =
            serde_json::from_slice(&info).map_err(|_| Error::Invalid("sync target metadata"))?;
        if info["version"] != 3 {
            return Err(Error::Invalid("unsupported Joplin sync version"));
        }
        match info["e2ee"]["value"].as_bool() {
            Some(false) => {}
            Some(true) => {
                if e2ee_password.is_empty() {
                    return Err(Error::Invalid(
                        "this Joplin target uses end-to-end encryption: enter its master password",
                    ));
                }
                keys = Some(
                    e2ee::Keys::unlock(&info, e2ee_password).map_err(|e| match e {
                        Error::Unlock => {
                            Error::Invalid("wrong Joplin end-to-end encryption password")
                        }
                        other => other,
                    })?,
                );
            }
            None => {
                return Err(Error::Invalid(
                    "invalid target encryption metadata; sync cancelled",
                ));
            }
        }
    } else if names.iter().any(|n| {
        !["locks", "temp", ".resource"].contains(&n.as_str())
            && !n.starts_with("locks/")
            && !n.starts_with("temp/")
    }) {
        return Err(Error::Invalid("missing sync metadata on non-empty target"));
    }
    if new_target
        && let Some(dav) = &api.dav
        && !dav.list(".resource")?.is_empty()
    {
        return Err(Error::Invalid(
            "missing sync metadata on non-empty resource directory",
        ));
    }
    let fingerprint = hex::encode(Sha256::digest(
        format!("{}:{}", api.base, api.user_id).as_bytes(),
    ));
    if store
        .setting("sync_identity")?
        .is_some_and(|s| s != fingerprint)
    {
        return Err(Error::Invalid(
            "different sync account: use a separate profile",
        ));
    }
    store.set_setting("sync_identity", &fingerprint)?;
    if new_target && !e2ee_password.is_empty() {
        if e2ee_password.chars().count() < 12 {
            return Err(Error::PasswordPolicy);
        }
        let (k, entity) = e2ee::Keys::create(e2ee_password)?;
        keys = Some(k);
        new_master_key = Some(entity);
    }
    let local = snapshot(store)?;
    let mut remote = BTreeMap::new();
    let mut total = 0usize;
    for name in names.iter().filter(|n| safe_remote_name(n)) {
        let data = api
            .get(name)?
            .ok_or(Error::Invalid("remote item disappeared during sync"))?;
        total += data.len();
        if total > MAX_TOTAL {
            return Err(Error::Invalid("remote snapshot exceeds limits"));
        }
        let data = if name.ends_with(".md") {
            match decrypt_item(keys.as_ref(), data)? {
                Some(d) => d,
                None => continue,
            }
        } else {
            data
        };
        let r = record(name, data)?;
        if [0, 1, 2, 4, 5, 6].contains(&r.kind) {
            remote.insert(name.clone(), r);
        }
    }
    let resources: Vec<String> = remote
        .iter()
        .filter(|(_, r)| r.kind == 4)
        .map(|(p, _)| p.trim_end_matches(".md").to_string())
        .collect();
    for id in &resources {
        let path = format!(".resource/{id}");
        if !remote.contains_key(&path) {
            let data = api
                .get(&path)?
                .ok_or(Error::Invalid("missing remote attachment"))?;
            total += data.len();
            if total > MAX_TOTAL {
                return Err(Error::Invalid("remote snapshot exceeds limits"));
            }
            put(&mut remote, path, data)?;
        }
    }
    decrypt_blobs(keys.as_ref(), &mut remote)?;
    remote.retain(|p, r| {
        r.kind != 0
            || p.strip_prefix(".resource/")
                .is_some_and(|id| resources.iter().any(|r| r == id))
    });
    let mut s = store.conn.prepare("SELECT path,hash FROM sync_baseline")?;
    let baseline = s
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    drop(s);
    if new_target && baseline.values().any(|h| h != ABSENT) {
        return Err(Error::Invalid(
            "sync failsafe: metadata disappeared from a previously used target",
        ));
    }
    let paths: BTreeSet<String> = local
        .keys()
        .chain(remote.keys())
        .chain(baseline.keys())
        .cloned()
        .collect();
    let mut incoming = Vec::new();
    let mut outgoing = Vec::new();
    let mut conflict_paths = Vec::new();
    for path in &paths {
        let l = hash(local.get(path));
        let r = hash(remote.get(path));
        let b = baseline.get(path).map_or(ABSENT, String::as_str);
        if l == r {
            continue;
        }
        if l != b && r != b {
            conflict_paths.push(path.clone());
        } else if l == b {
            incoming.push(path.clone());
        } else {
            outgoing.push(path.clone());
        }
    }
    let removals = incoming
        .iter()
        .filter(|p| local.contains_key(*p) && !remote.contains_key(*p))
        .count();
    if removals > 0 && removals.saturating_mul(100) >= local.len().saturating_mul(90) {
        return Err(Error::Invalid(
            "sync failsafe: at least 90 percent of local records would be deleted",
        ));
    }
    if !conflict_paths.is_empty() {
        let tx = store.conn.transaction()?;
        tx.execute("DELETE FROM sync_conflicts", [])?;
        for path in &conflict_paths {
            tx.execute(
                "INSERT INTO sync_conflicts VALUES(?1,?2,?3,?4,?5)",
                params![
                    path,
                    local.get(path).map(|r| r.data.as_slice()),
                    remote.get(path).map(|r| r.data.as_slice()),
                    hash(local.get(path)),
                    hash(remote.get(path))
                ],
            )?;
        }
        tx.commit()?;
        return Ok(Outcome::Conflicts(conflict_paths.len()));
    }
    api.check()?;
    if !incoming.is_empty() {
        store.recovery_snapshot()?;
        apply_remote(store, &local, &remote, &incoming)?;
    }
    if new_target {
        let (enabled, active, master_keys, at) = match (&keys, new_master_key) {
            (Some(k), Some(mk)) => (true, k.active.clone(), vec![mk], now()),
            _ => (false, String::new(), vec![], 0),
        };
        let info = json!({"version":3,"e2ee":{"value":enabled,"updatedTime":at},"activeMasterKeyId":{"value":active,"updatedTime":at},"masterKeys":master_keys,"ppk":{"value":null,"updatedTime":0},"appMinVersion":"3.7.0","revisionServiceEnabled":{"value":true,"updatedTime":0},"revisionServiceTtlDays":{"value":90,"updatedTime":0}});
        api.put(
            "info.json",
            &serde_json::to_vec(&info).map_err(|_| Error::Invalid("sync metadata"))?,
        )?;
    }
    // Commit baselines per successful operation; a disconnect can be retried safely.
    for path in &incoming {
        baseline_set(store, path, hash(remote.get(path)))?;
    }
    let mut deleted = 0;
    // Upload payloads first, then descriptors and notes, before any deletion.
    outgoing.sort_by_key(|p| (!p.starts_with(".resource/"), p.clone()));
    for path in &outgoing {
        if let Some(r) = local.get(path) {
            match &keys {
                Some(k) => api.put(path, &encrypt_for_upload(k, path, &r.data)?)?,
                None => api.put(path, &r.data)?,
            }
        } else {
            api.delete(path)?;
            deleted += 1;
        }
        baseline_set(store, path, hash(local.get(path)))?;
    }
    for path in &paths {
        if hash(local.get(path)) == hash(remote.get(path)) {
            baseline_set(store, path, hash(local.get(path)))?;
        }
    }
    store.conn.execute("DELETE FROM sync_conflicts", [])?;
    store.set_setting("sync_last_success", &now().to_string())?;
    Ok(Outcome::Complete {
        uploaded: outgoing.len() - deleted,
        downloaded: incoming.len(),
        deleted,
    })
}
/// Keys kept in clear by Joplin on encrypted items (`BaseItem.serializeForSync`).
const CLEAR_KEYS: [&str; 10] = [
    "id",
    "note_id",
    "tag_id",
    "parent_id",
    "share_id",
    "updated_time",
    "deleted_time",
    "type_",
    "is_locked",
    "extracted_resource_ids",
];

// Joplin 3.7 table columns (title and body are written separately).
const NOTE_FIELDS: &[&str] = &[
    "id",
    "parent_id",
    "created_time",
    "updated_time",
    "is_conflict",
    "latitude",
    "longitude",
    "altitude",
    "author",
    "source_url",
    "is_todo",
    "todo_due",
    "todo_completed",
    "source",
    "source_application",
    "application_data",
    "order",
    "user_created_time",
    "user_updated_time",
    "encryption_cipher_text",
    "encryption_applied",
    "markup_language",
    "is_shared",
    "share_id",
    "conflict_original_id",
    "master_key_id",
    "user_data",
    "deleted_time",
    "is_locked",
    "extracted_resource_ids",
    "type_",
];
const FOLDER_FIELDS: &[&str] = &[
    "id",
    "created_time",
    "updated_time",
    "user_created_time",
    "user_updated_time",
    "encryption_cipher_text",
    "encryption_applied",
    "parent_id",
    "is_shared",
    "share_id",
    "master_key_id",
    "icon",
    "user_data",
    "deleted_time",
    "type_",
];
const RESOURCE_FIELDS: &[&str] = &[
    "id",
    "mime",
    "filename",
    "created_time",
    "updated_time",
    "user_created_time",
    "user_updated_time",
    "file_extension",
    "encryption_cipher_text",
    "encryption_applied",
    "encryption_blob_encrypted",
    "size",
    "is_shared",
    "share_id",
    "master_key_id",
    "user_data",
    "blob_updated_time",
    "ocr_text",
    "ocr_details",
    "ocr_status",
    "ocr_error",
    "ocr_driver_id",
    "is_locked",
    "type_",
];
const TAG_FIELDS: &[&str] = &[
    "id",
    "created_time",
    "updated_time",
    "user_created_time",
    "user_updated_time",
    "encryption_cipher_text",
    "encryption_applied",
    "is_shared",
    "parent_id",
    "user_data",
    "type_",
];
const NOTE_TAG_FIELDS: &[&str] = &[
    "id",
    "note_id",
    "tag_id",
    "created_time",
    "updated_time",
    "user_created_time",
    "user_updated_time",
    "encryption_cipher_text",
    "encryption_applied",
    "is_shared",
    "type_",
];

/// Returns the plain serialised form of a remote item, decrypting it when needed.
/// Item types other than notes, notebooks, resources, tags and tag links are ignored (`None`).
fn decrypt_item(keys: Option<&e2ee::Keys>, data: Vec<u8>) -> Result<Option<Vec<u8>>> {
    let text = std::str::from_utf8(&data).map_err(|_| Error::Invalid("sync item is not UTF-8"))?;
    let outer = exchange::parse_item_raw(text)?;
    if !["1", "2", "4", "5", "6"].contains(&outer.get("type_")) {
        return Ok(None);
    }
    if outer.number("encryption_applied")? == 0 {
        return Ok(Some(data));
    }
    let keys = keys.ok_or(Error::Invalid(
        "encrypted item on a target without end-to-end encryption",
    ))?;
    let plain = e2ee::decrypt_string(keys, outer.get("encryption_cipher_text"))?;
    let mut item = exchange::parse_item_raw(plain.as_str())?;
    if item.get("id") != outer.get("id") {
        return Err(Error::Invalid("encrypted item identifier mismatch"));
    }
    item.props
        .insert("updated_time".into(), outer.get("updated_time").into());
    // The plain text was serialised before encryption; only reset fields it actually carries.
    for (k, v) in [("encryption_applied", "0"), ("encryption_cipher_text", "")] {
        if let Some(p) = item.props.get_mut(k) {
            *p = v.into();
        }
    }
    exchange::serialize(&item.title, &item.body, item.props).map(Some)
}

/// Decrypts attachment payloads whose (already decrypted) descriptor says the blob is encrypted.
fn decrypt_blobs(keys: Option<&e2ee::Keys>, remote: &mut BTreeMap<String, Record>) -> Result<()> {
    let encrypted: Vec<String> = remote
        .iter()
        .filter(|(_, r)| r.kind == 4)
        .filter_map(|(p, r)| parse(&r.data).ok().map(|i| (p, i)))
        .filter(|(_, i)| i.get("encryption_blob_encrypted") == "1")
        .map(|(p, _)| p.trim_end_matches(".md").to_string())
        .collect();
    for id in encrypted {
        let keys = keys.ok_or(Error::Invalid(
            "encrypted attachment on a target without end-to-end encryption",
        ))?;
        let blob_path = format!(".resource/{id}");
        if let Some(blob) = remote.get(&blob_path) {
            let plain = e2ee::decrypt_file(keys, &blob.data)?;
            put(remote, blob_path, plain.to_vec())?;
        }
        let path = format!("{id}.md");
        let mut item = parse(&remote[&path].data)?;
        item.props
            .insert("encryption_blob_encrypted".into(), "0".into());
        if let Some(blob) = remote.get(&format!(".resource/{id}")) {
            item.props
                .insert("size".into(), blob.data.len().to_string());
        }
        let data = exchange::serialize(&item.title, &item.body, item.props)?;
        put(remote, path, data)?;
    }
    Ok(())
}

/// Produces what Joplin uploads when end-to-end encryption is enabled.
fn encrypt_for_upload(keys: &e2ee::Keys, path: &str, data: &[u8]) -> Result<Vec<u8>> {
    if path.starts_with(".resource/") {
        return e2ee::encrypt_file(keys, data);
    }
    let mut item = parse(data)?;
    if item.get("type_") == "4" {
        item.props
            .insert("encryption_blob_encrypted".into(), "1".into());
    }
    let plain = Zeroizing::new(exchange::serialize(
        &item.title,
        &item.body,
        item.props.clone(),
    )?);
    let text = std::str::from_utf8(&plain).map_err(|_| Error::Invalid("sync item"))?;
    let cipher = e2ee::encrypt_string(keys, text)?;
    // Joplin serialises every column of the item type, empty unless kept in clear; its
    // database rejects encrypted items whose NOT NULL columns (mime, ...) are missing.
    let fields = match item.get("type_") {
        "1" => NOTE_FIELDS,
        "2" => FOLDER_FIELDS,
        "4" => RESOURCE_FIELDS,
        "5" => TAG_FIELDS,
        "6" => NOTE_TAG_FIELDS,
        _ => return Err(Error::Invalid("sync item type")),
    };
    let mut reduced: BTreeMap<String, String> = fields
        .iter()
        .map(|k| (k.to_string(), String::new()))
        .collect();
    for k in CLEAR_KEYS {
        if let Some(v) = item.props.get(k) {
            reduced.insert(k.to_string(), v.clone());
        }
    }
    reduced.insert("encryption_applied".into(), "1".into());
    reduced.insert("encryption_cipher_text".into(), cipher);
    exchange::serialize("", "", reduced)
}

fn baseline_set(store: &Store, path: &str, hash: &str) -> Result<()> {
    store.conn.execute("INSERT INTO sync_baseline VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET hash=excluded.hash",params![path,hash])?;
    Ok(())
}
fn apply_remote(
    store: &mut Store,
    local: &BTreeMap<String, Record>,
    remote: &BTreeMap<String, Record>,
    incoming: &[String],
) -> Result<()> {
    let tx = store.conn.transaction()?;
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;")?;
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let mut resource_ids = BTreeSet::new();
    for path in incoming {
        match remote.get(path) {
            Some(record) => {
                if record.kind == 0 {
                    resource_ids.insert(path.trim_start_matches(".resource/").to_string());
                } else {
                    // Items of notebooks shared through Joplin Server keep their share_id in props.
                    changed.push(parse(&record.data)?);
                }
            }
            None => {
                if let Some(record) = local.get(path) {
                    removed.push((path.clone(), record.kind));
                }
            }
        }
    }
    for item in changed.iter().filter(|i| i.get("type_") == "2") {
        if item.title.trim().is_empty()
            || item.title.len() > 1024
            || item.title.contains(['\n', '\r', '\0'])
        {
            return Err(Error::Invalid("remote notebook title"));
        }
        let parent = if item.get("parent_id").is_empty() {
            None
        } else {
            Some(item.get("parent_id"))
        };
        if parent.is_some_and(|p| !valid_id(p)) {
            return Err(Error::Invalid("remote parent notebook"));
        }
        tx.execute("INSERT INTO notebooks VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET parent_id=excluded.parent_id,title=excluded.title",params![item.get("id"),parent,item.title])?;
    }
    for item in changed.iter().filter(|i| i.get("type_") == "5") {
        if item.title.trim().is_empty()
            || item.title.len() > 256
            || item.title.contains([',', '\n', '\r', '\0'])
        {
            return Err(Error::Invalid("remote tag title"));
        }
        tx.execute(
            "INSERT INTO tags VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET title=excluded.title",
            params![item.get("id"), item.title],
        )?;
    }
    for item in changed.iter().filter(|i| i.get("type_") == "1") {
        if item.number("markup_language")? > 2 {
            return Err(Error::Invalid("unknown note format; sync cancelled"));
        }
        let n = Note {
            id: item.get("id").into(),
            notebook_id: item.get("parent_id").into(),
            title: item.title.clone(),
            body: item.body.clone(),
            is_todo: item.number("is_todo")? != 0,
            completed: item.number("todo_completed")? > 0,
            due: item.timestamp("todo_due")?,
            created: item.timestamp("created_time")?,
            updated: item.timestamp("updated_time")?,
            deleted: item.timestamp("deleted_time")? > 0,
            version: 0,
            extra: serde_json::to_string(&item.props)
                .map_err(|_| Error::Invalid("remote metadata"))?,
        };
        validate_note(&n)?;
        let previous: Option<(String, String)> = tx
            .query_row("SELECT title,body FROM notes WHERE id=?1", [&n.id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?;
        if let Some((title, body)) = previous {
            if title != n.title || body != n.body {
                tx.execute(
                    "INSERT INTO revisions(note_id,at,title,body) VALUES(?1,?2,?3,?4)",
                    params![n.id, now(), title, body],
                )?;
            }
            tx.execute("UPDATE notes SET notebook_id=?2,title=?3,body=?4,is_todo=?5,completed=?6,due=?7,created=?8,updated=?9,deleted=?10,version=version+1,extra=?11 WHERE id=?1",params![n.id,n.notebook_id,n.title,n.body,n.is_todo,n.completed,n.due,n.created,n.updated,n.deleted,n.extra])?;
            tx.execute("DELETE FROM search WHERE note_id=?1", [&n.id])?;
            tx.execute(
                "INSERT INTO search VALUES(?1,?2,?3)",
                params![n.id, n.title, n.body],
            )?;
        } else {
            Store::insert_note(&tx, &n)?;
        }
    }
    for item in changed.iter().filter(|i| i.get("type_") == "4") {
        resource_ids.insert(item.get("id").to_string());
    }
    for id in resource_ids {
        if !valid_id(&id) {
            return Err(Error::Invalid("resource identifier"));
        }
        let descriptor = remote
            .get(&format!("{id}.md"))
            .ok_or(Error::Invalid("missing resource metadata"))?;
        let item = parse(&descriptor.data)?;
        let ext = item.get("file_extension");
        if item.number("encryption_blob_encrypted")? != 0 {
            return Err(Error::Invalid(
                "encrypted Joplin attachments are not supported yet",
            ));
        }
        if ext.len() > 32 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::Invalid("resource extension"));
        }
        let data = &remote
            .get(&format!(".resource/{id}"))
            .ok_or(Error::Invalid("missing resource payload"))?
            .data;
        if data.len() as u64 > MAX_ATTACHMENT {
            return Err(Error::Invalid("remote attachment too large"));
        }
        tx.execute("INSERT INTO resources VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET title=excluded.title,extension=excluded.extension,mime=excluded.mime,data=excluded.data,extra=excluded.extra",params![id,item.title,ext,item.get("mime"),data.as_slice(),serde_json::to_string(&item.props).map_err(|_|Error::Invalid("resource metadata"))?])?;
    }
    for item in changed.iter().filter(|i| i.get("type_") == "6") {
        if !valid_id(item.get("note_id")) || !valid_id(item.get("tag_id")) {
            return Err(Error::Invalid("tag relationship"));
        }
        tx.execute(
            "INSERT OR IGNORE INTO note_tags VALUES(?1,?2)",
            params![item.get("note_id"), item.get("tag_id")],
        )?;
        tx.execute("INSERT INTO relation_ids VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET note_id=excluded.note_id,tag_id=excluded.tag_id",params![item.get("id"),item.get("note_id"),item.get("tag_id")])?;
    }
    // Relations first, then notes, notebooks, tags, and resource descriptors.
    removed.sort_by_key(|(_, kind)| match kind {
        6 => 0,
        1 => 1,
        2 => 2,
        5 => 3,
        4 => 4,
        _ => 5,
    });
    for (path, kind) in &removed {
        let id = path.trim_end_matches(".md");
        match kind {
            6 => {
                tx.execute("DELETE FROM relation_ids WHERE id=?1", [id])?;
                tx.execute("DELETE FROM note_tags WHERE NOT EXISTS(SELECT 1 FROM relation_ids r WHERE r.note_id=note_tags.note_id AND r.tag_id=note_tags.tag_id)",[])?;
            }
            1 => {
                for table in ["revisions", "drafts", "relation_ids", "search"] {
                    tx.execute(&format!("DELETE FROM {table} WHERE note_id=?1"), [id])?;
                }
                tx.execute("DELETE FROM notes WHERE id=?1", [id])?;
            }
            2 => {
                tx.execute("DELETE FROM notebooks WHERE id=?1", [id])?;
            }
            5 => {
                tx.execute("DELETE FROM tags WHERE id=?1", [id])?;
            }
            4 => {
                let refs: i64 = tx.query_row(
                    "SELECT count(*) FROM notes WHERE instr(body,?1)>0",
                    [id],
                    |r| r.get(0),
                )?;
                if refs > 0 {
                    return Err(Error::Invalid(
                        "server removed an attachment that is still referenced",
                    ));
                }
                tx.execute("DELETE FROM resources WHERE id=?1", [id])?;
            }
            _ => {}
        }
        if *kind != 0 {
            tx.execute("DELETE FROM item_meta WHERE id=?1", [id])?;
        }
    }
    for item in changed
        .iter()
        .filter(|i| ["2", "5", "6"].contains(&i.get("type_")))
    {
        tx.execute("INSERT INTO item_meta VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET props=excluded.props",params![item.get("id"),serde_json::to_string(&item.props).map_err(|_|Error::Invalid("item metadata"))?])?;
    }
    let mut s = tx.prepare("SELECT id,parent_id FROM notebooks")?;
    let parents = s
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
    drop(s);
    let mut checked = BTreeSet::new();
    for id in parents.keys() {
        let mut chain = BTreeSet::new();
        let mut p = Some(id.as_str());
        while let Some(id) = p {
            if checked.contains(id) {
                break;
            }
            if !chain.insert(id) {
                return Err(Error::Invalid("remote notebook cycle"));
            }
            p = parents
                .get(id)
                .ok_or(Error::Invalid("missing remote parent"))?
                .as_deref();
        }
        checked.extend(chain);
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tls_and_origin_validation() {
        for url in [
            "http://example.com",
            "https://user:secret@example.com",
            "https://example.com/?token=secret",
            "file:///tmp/database",
            "https://example.com/#fragment",
        ] {
            assert!(validate_server(url).is_err());
        }
        assert!(
            validate_server("https://joplin.example.com/prefix")
                .unwrap()
                .as_str()
                .ends_with("prefix/")
        );
        assert!(validate_server("http://127.0.0.1:22300").is_ok());
    }
    #[test]
    fn nextcloud_endpoint_is_generic_and_encodes_username() {
        let mut c = Config {
            server: "https://cloud.example.org/nextcloud".into(),
            email: "alice@example.org".into(),
            target: Target::Nextcloud,
            automatic: false,
            password: String::new(),
            client_id: id(),
            e2ee_password: String::new(),
        };
        let url = endpoint(&c).unwrap();
        assert_eq!(
            url.as_str(),
            "https://cloud.example.org/nextcloud/remote.php/dav/files/alice@example.org/Joplin/"
        );
        c.email = "private user".into();
        assert!(endpoint(&c).unwrap().as_str().contains("private%20user"));
        c.email = "../other".into();
        assert!(endpoint(&c).is_err());
        c.server = "https://cloud.example.org/remote.php/dav/files/custom/Joplin/".into();
        c.email = "alice".into();
        assert_eq!(endpoint(&c).unwrap().as_str(), c.server);
        let old:Config=serde_json::from_str(r#"{"server":"https://example.org","email":"a","password":"b","client_id":"0123456789abcdef0123456789abcdef"}"#).unwrap();
        assert_eq!(old.target, Target::JoplinServer);
        assert!(!old.automatic);
    }
    #[test]
    fn snapshot_identifiers_and_metadata_are_stable() {
        let mut s = Store::memory().unwrap();
        let b = s.create_notebook("Example", None).unwrap();
        let mut n = s.create_note(&b, "Example", false).unwrap();
        n.body = "Body".into();
        s.save(&mut n, "work").unwrap();
        let first = snapshot(&s).unwrap();
        let second = snapshot(&s).unwrap();
        assert_eq!(
            first.keys().collect::<Vec<_>>(),
            second.keys().collect::<Vec<_>>()
        );
        for (p, r) in first {
            assert_eq!(r.hash, second[&p].hash);
        }
    }
    #[test]
    fn sync_credentials_are_only_written_inside_sqlcipher() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), &id()).unwrap();
        let config = Config {
            target: Target::JoplinServer,
            automatic: false,
            server: "https://example.org".into(),
            email: "private-mail-sentinel@example.org".into(),
            password: "Private account credential sentinel".into(),
            client_id: id(),
            e2ee_password: String::new(),
        };
        save_config(&store, &config).unwrap();
        assert_eq!(load_config(&store).unwrap().email, config.email);
        for f in std::fs::read_dir(dir.path()).unwrap() {
            let p = f.unwrap().path();
            if p.is_file() {
                // These control files are not encrypted content and carry
                // active byte-range locks on Windows.
                let name = p.file_name().unwrap().to_string_lossy();
                if name == "profile.lock" || name.ends_with("-shm") {
                    continue;
                }
                let data = std::fs::read(&p)
                    .unwrap_or_else(|e| panic!("cannot inspect {}: {e}", p.display()));
                for value in [&config.email, &config.password] {
                    assert!(!data.windows(value.len()).any(|b| b == value.as_bytes()));
                }
            }
        }
    }
    #[test]
    #[ignore = "requires isolated Joplin Server at CARNET_TEST_SERVER; never use a real account"]
    fn real_server_roundtrip_conflict_and_delete() {
        let server = std::env::var("CARNET_TEST_SERVER").expect("isolated test server URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        roundtrip_scenario(
            server,
            Target::JoplinServer,
            "admin@localhost".into(),
            std::env::var("CARNET_TEST_SERVER_PASSWORD").expect("isolated Joplin Server password"),
        );
    }
    #[test]
    #[ignore = "requires isolated demo WebDAV server at CARNET_TEST_DAV; never use a real account"]
    fn webdav_roundtrip_conflict_delete_and_e2ee_refusal() {
        let server = std::env::var("CARNET_TEST_DAV").expect("isolated demo DAV URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        roundtrip_scenario(
            server,
            Target::WebDav,
            "demo".into(),
            std::env::var("CARNET_TEST_DAV_PASSWORD").expect("isolated WebDAV password"),
        );
    }
    #[test]
    #[ignore = "requires isolated Nextcloud at CARNET_TEST_NEXTCLOUD; never use a real account"]
    fn nextcloud_account_url_roundtrip() {
        let server =
            std::env::var("CARNET_TEST_NEXTCLOUD").expect("isolated demo Nextcloud server URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        roundtrip_scenario(
            server,
            Target::Nextcloud,
            "demo".into(),
            std::env::var("CARNET_TEST_DAV_PASSWORD").expect("isolated Nextcloud password"),
        );
    }
    #[test]
    #[ignore = "requires isolated WebDAV at CARNET_TEST_DAV; intended for native Joplin client interop"]
    fn native_joplin_reads_carnet_upload() {
        let server = std::env::var("CARNET_TEST_DAV").expect("isolated demo DAV URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path(), &id()).unwrap();
        let book = store.create_notebook("CarnetWriteInterop", None).unwrap();
        let mut note = store
            .create_note(&book, "CarnetWrittenNote", false)
            .unwrap();
        note.body = "RustToJoplinBody".into();
        store.save(&mut note, "interop").unwrap();
        let config = Config {
            target: Target::WebDav,
            automatic: false,
            server,
            email: "demo".into(),
            password: std::env::var("CARNET_TEST_DAV_PASSWORD").expect("isolated WebDAV password"),
            client_id: id(),
            e2ee_password: String::new(),
        };
        assert!(matches!(
            run(&mut store, &config, Arc::new(AtomicBool::new(false))).unwrap(),
            Outcome::Complete { .. }
        ));
    }
    /// Carnet creates a new end-to-end encrypted target; the Joplin client must then open it.
    #[test]
    #[ignore = "requires an empty isolated WebDAV directory"]
    fn new_e2ee_target_for_joplin() {
        let server = std::env::var("CARNET_TEST_DAV").expect("isolated DAV URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("a"), &id()).unwrap();
        let config = Config {
            target: Target::WebDav,
            automatic: false,
            server,
            email: "demo".into(),
            password: std::env::var("CARNET_TEST_DAV_PASSWORD").unwrap(),
            client_id: id(),
            e2ee_password: std::env::var("CARNET_TEST_E2EE_PASSWORD").unwrap(),
        };
        let book = store.create_notebook("FromCarnet", None).unwrap();
        let mut note = store.create_note(&book, "SecretFromCarnet", false).unwrap();
        note.body = "Encrypted by Carnet".into();
        store.save(&mut note, "interop").unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        assert!(matches!(
            run(&mut store, &config, cancel.clone()).unwrap(),
            Outcome::Complete { .. }
        ));
        let api = Api::new(&store, &config, cancel.clone()).unwrap();
        let mut api = api;
        let info: Value = serde_json::from_slice(&api.get("info.json").unwrap().unwrap()).unwrap();
        assert_eq!(info["e2ee"]["value"], true);
        let raw = String::from_utf8(api.get(&format!("{}.md", note.id)).unwrap().unwrap()).unwrap();
        assert!(!raw.contains("Encrypted by Carnet") && raw.contains("encryption_applied: 1"));
    }

    /// Two-way check with the official Joplin client on an end-to-end encrypted WebDAV target.
    /// The target must already contain a note titled "JoplinE2EE" with body "BodyFromJoplin" and
    /// one attachment; afterwards the Joplin client must find "CarnetE2EE".
    #[test]
    #[ignore = "requires an isolated WebDAV target prepared by the Joplin client"]
    fn joplin_e2ee_webdav_interop() {
        let server = std::env::var("CARNET_TEST_DAV").expect("isolated DAV URL");
        assert!(server.starts_with("http://127.0.0.1:"));
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("a"), &id()).unwrap();
        let mut config = Config {
            target: Target::WebDav,
            automatic: false,
            server,
            email: "demo".into(),
            password: std::env::var("CARNET_TEST_DAV_PASSWORD").unwrap(),
            client_id: id(),
            e2ee_password: crate::e2ee::tests::random_password(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        assert!(matches!(
            run(&mut store, &config, cancel.clone()),
            Err(Error::Invalid(
                "wrong Joplin end-to-end encryption password"
            ))
        ));
        config.e2ee_password = std::env::var("CARNET_TEST_E2EE_PASSWORD").unwrap();
        assert!(matches!(
            run(&mut store, &config, cancel.clone()).unwrap(),
            Outcome::Complete { .. }
        ));
        let notes = store.all_notes().unwrap();
        let from_joplin = notes
            .iter()
            .find(|n| n.title == "JoplinE2EE")
            .expect("decrypted Joplin note");
        assert!(
            from_joplin.body.contains("BodyFromJoplin"),
            "{}",
            from_joplin.body
        );
        let attachments: i64 = store
            .conn
            .query_row("SELECT count(*) FROM resources", [], |r| r.get(0))
            .unwrap();
        assert!(attachments >= 1);
        let book = store.notebooks().unwrap()[0].id.clone();
        let mut note = store.create_note(&book, "CarnetE2EE", false).unwrap();
        let resource = store
            .add_attachment("from-carnet.txt", b"Attachment written by Carnet")
            .unwrap();
        note.body = format!("BodyFromCarnet é漢🙂\n\n[Attachment](:/{resource})");
        store.save(&mut note, "interop").unwrap();
        assert!(
            matches!(run(&mut store, &config, cancel.clone()).unwrap(), Outcome::Complete { uploaded, .. } if uploaded >= 3)
        );
        // A second pass must be a no-op: decrypted remote items hash like the local ones.
        assert!(matches!(
            run(&mut store, &config, cancel).unwrap(),
            Outcome::Complete {
                uploaded: 0,
                downloaded: 0,
                deleted: 0
            }
        ));
    }
    fn roundtrip_scenario(server: String, target: Target, email: String, password: String) {
        let dir = tempfile::tempdir().unwrap();
        let vault_secret = id();
        let mut a = Store::open(&dir.path().join("a"), &vault_secret).unwrap();
        let mut b = Store::open(&dir.path().join("b"), &vault_secret).unwrap();
        let mut config = Config {
            target,
            automatic: false,
            server,
            email,
            password,
            client_id: id(),
            e2ee_password: String::new(),
        };
        let book = a.create_notebook("Demonstration", None).unwrap();
        let mut note = a
            .create_note(&book, "Server integration example", true)
            .unwrap();
        let resource = a
            .add_attachment("example.txt", b"An integration test attachment")
            .unwrap();
        note.body = format!("Hello from Rust\n\n[Attachment](:/{resource})");
        a.save(&mut note, "integration").unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        assert!(matches!(
            run(&mut a, &config, cancel.clone()).unwrap(),
            Outcome::Complete { .. }
        ));
        if std::env::var_os("CARNET_TEST_NATIVE_JOPLIN_SEED").is_some() {
            assert!(
                a.notebooks()
                    .unwrap()
                    .iter()
                    .any(|book| book.title == "NativeJoplinInterop")
            );
            assert!(a.all_notes().unwrap().iter().any(|note| {
                note.title == "NativeJoplinNote" && note.body == "NativeBodyInterop"
            }));
        }
        config.client_id = id();
        if target != Target::JoplinServer {
            let mut api = Api::new(&a, &config, cancel.clone()).unwrap();
            let other_sync = format!("locks/1_1_{}.json", id());
            api.put(&other_sync, b"{\"type\":1,\"clientType\":1}")
                .unwrap();
            // An ordinary Joplin sync lock must not block another client.
            assert!(matches!(
                run(&mut b, &config, cancel.clone()).unwrap(),
                Outcome::Complete { .. }
            ));
            api.get(&other_sync).unwrap();
            api.delete(&other_sync).unwrap();
            let exclusive = format!("locks/2_1_{}.json", id());
            api.put(&exclusive, b"{\"type\":2,\"clientType\":1}")
                .unwrap();
            let blocked = run(&mut b, &config, cancel.clone()).unwrap_err();
            assert!(matches!(
                blocked,
                Error::Invalid("another Joplin client holds an exclusive lock; retry later")
            ));
            api.get(&exclusive).unwrap();
            api.delete(&exclusive).unwrap();
        } else {
            let api = Api::new(&a, &config, cancel.clone()).unwrap();
            let foreign_id = id();
            let lock = serde_json::to_vec(&json!({"type":1,"clientType":1,"clientId":foreign_id}))
                .unwrap();
            api.request("POST", "api/locks", Some(&lock), false)
                .unwrap();
            // Joplin Server also permits simultaneous ordinary sync locks.
            assert!(matches!(
                run(&mut b, &config, cancel.clone()).unwrap(),
                Outcome::Complete { .. }
            ));
            api.request("DELETE", &format!("api/locks/1_1_{foreign_id}"), None, true)
                .unwrap();
            let exclusive_id = id();
            let exclusive =
                serde_json::to_vec(&json!({"type":2,"clientType":1,"clientId":exclusive_id}))
                    .unwrap();
            api.request("POST", "api/locks", Some(&exclusive), false)
                .unwrap();
            let blocked = run(&mut b, &config, cancel.clone()).unwrap_err();
            assert!(matches!(blocked, Error::SyncStatus(409 | 423)));
            api.request(
                "DELETE",
                &format!("api/locks/2_1_{exclusive_id}"),
                None,
                true,
            )
            .unwrap();
        }
        assert!(matches!(
            run(&mut b, &config, cancel.clone()).unwrap(),
            Outcome::Complete { .. }
        ));
        assert_eq!(b.note(&note.id).unwrap().body, note.body);
        assert_eq!(b.tags(&note.id).unwrap(), "integration");
        assert_eq!(
            b.attachments(&note.body).unwrap()[0].data,
            b"An integration test attachment"
        );
        // Canonical server serialisation may add defaults on the first echo.
        run(&mut a, &config, cancel.clone()).unwrap();
        run(&mut b, &config, cancel.clone()).unwrap();
        let mut left = a.note(&note.id).unwrap();
        left.body = "Local version".into();
        a.save(&mut left, "integration").unwrap();
        let mut right = b.note(&note.id).unwrap();
        right.body = "Remote version".into();
        b.save(&mut right, "integration").unwrap();
        run(&mut b, &config, cancel.clone()).unwrap();
        assert!(matches!(
            run(&mut a, &config, cancel.clone()).unwrap(),
            Outcome::Conflicts(_)
        ));
        assert_eq!(a.note(&note.id).unwrap().body, "Local version");
        resolve(&mut a, &format!("{}.md", note.id), Resolution::Remote).unwrap();
        run(&mut a, &config, cancel.clone()).unwrap();
        assert_eq!(a.note(&note.id).unwrap().body, "Remote version");
        let mut deleted = a.note(&note.id).unwrap();
        deleted.deleted = true;
        a.save(&mut deleted, "integration").unwrap();
        run(&mut a, &config, cancel.clone()).unwrap();
        run(&mut b, &config, cancel.clone()).unwrap();
        assert!(b.note(&note.id).unwrap().deleted);
        a.purge_trash().unwrap();
        run(&mut a, &config, cancel.clone()).unwrap();
        run(&mut b, &config, cancel.clone()).unwrap();
        assert!(b.note(&note.id).is_err());
        let mut api = Api::new(&a, &config, cancel.clone()).unwrap();
        if target != Target::JoplinServer {
            api.put("guard.bin", b"First version").unwrap();
            api.get("guard.bin").unwrap();
            let mut other = Api::new(&a, &config, cancel.clone()).unwrap();
            other.get("guard.bin").unwrap();
            other.put("guard.bin", b"Changed concurrently").unwrap();
            assert!(matches!(
                api.put("guard.bin", b"Do not overwrite"),
                Err(Error::SyncStatus(412))
            ));
            api.get("guard.bin").unwrap();
            api.delete("guard.bin").unwrap();
        }
        let info = api.get("info.json").unwrap().unwrap();
        let mut encrypted: Value = serde_json::from_slice(&info).unwrap();
        encrypted["e2ee"]["value"] = json!(true);
        api.put("info.json", &serde_json::to_vec(&encrypted).unwrap())
            .unwrap();
        let withheld = a
            .create_note(&book, "Do not upload to an E2EE target", false)
            .unwrap();
        assert!(run(&mut a, &config, cancel.clone()).is_err());
        assert!(api.get(&format!("{}.md", withheld.id)).unwrap().is_none());
        api.get("info.json").unwrap();
        api.put("info.json", &info).unwrap();
        for path in api
            .list()
            .unwrap()
            .into_iter()
            .filter(|p| safe_remote_name(p))
        {
            api.get(&path).unwrap();
            api.delete(&path).unwrap();
        }
        if let Some(dav) = &api.dav {
            for name in dav.list(".resource").unwrap() {
                assert!(valid_id(&name));
                let path = format!(".resource/{name}");
                api.get(&path).unwrap();
                api.delete(&path).unwrap();
            }
        }
        api.get("info.json").unwrap();
        api.delete("info.json").unwrap();
        assert!(run(&mut a, &config, cancel).is_err());
        assert!(a.note(&withheld.id).is_ok());
    }
}
