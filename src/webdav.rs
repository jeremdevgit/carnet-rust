//! Bounded WebDAV transport for Joplin format 3. Only a user-selected notes directory is touched.
use crate::{
    Error, Result, now,
    store::{MAX_ATTACHMENT, valid_id},
    sync::{Config, endpoint},
};
use quick_xml::{Reader, events::Event};
use reqwest::{
    Url,
    blocking::{Client, Response},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub(crate) struct Dav {
    base: Url,
    client: Client,
    username: Zeroizing<String>,
    password: Zeroizing<String>,
    client_id: String,
    etags: BTreeMap<String, Option<String>>,
    renewed: Instant,
    started: Instant,
    cancel: Arc<AtomicBool>,
    held: bool,
}
impl Dav {
    pub(crate) fn new(client: Client, config: &Config, cancel: Arc<AtomicBool>) -> Result<Self> {
        let base = endpoint(config)?;
        // Never initialise a server/account root as a notes collection.
        if base.path().trim_matches('/').is_empty() {
            return Err(Error::Invalid(
                "choose a dedicated WebDAV notes directory, e.g. Joplin",
            ));
        }
        if config.email.is_empty() || !valid_id(&config.client_id) {
            return Err(Error::Invalid("WebDAV account settings"));
        }
        Ok(Self {
            base,
            client,
            username: Zeroizing::new(config.email.clone()),
            password: Zeroizing::new(config.password.clone()),
            client_id: config.client_id.clone(),
            etags: BTreeMap::new(),
            renewed: Instant::now(),
            started: Instant::now(),
            cancel,
            held: false,
        })
    }
    fn url(&self, path: &str) -> Result<Url> {
        let mut url = self.base.clone();
        if !path.is_empty() {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| Error::Invalid("WebDAV URL"))?;
            segments.pop_if_empty();
            for segment in path.split('/') {
                if segment.is_empty()
                    || segment == "."
                    || segment == ".."
                    || segment.contains(['\\', '\0'])
                {
                    return Err(Error::Invalid("WebDAV path"));
                }
                segments.push(segment);
            }
        }
        Ok(url)
    }
    fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        conditional: bool,
    ) -> Result<Response> {
        if (self.started.elapsed() > Duration::from_secs(120)
            || self.cancel.load(Ordering::Relaxed))
            && !(method == "DELETE" && path == self.lock_path())
        {
            return Err(Error::Invalid("sync time limit; retry to continue"));
        }
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| Error::Invalid("WebDAV method"))?;
        let mut request = self
            .client
            .request(method, self.url(path)?)
            .basic_auth(self.username.as_str(), Some(self.password.as_str()));
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/octet-stream")
                .body(body.to_vec());
        }
        if conditional {
            match self.etags.get(path) {
                Some(Some(tag)) if !tag.starts_with("W/") => {
                    request = request.header("If-Match", tag);
                }
                Some(Some(_)) => {
                    return Err(Error::Invalid(
                        "weak WebDAV ETag: refusing unsafe overwrite",
                    ));
                }
                Some(None) | None => {
                    request = request.header("If-None-Match", "*");
                }
            }
        }
        request
            .send()
            .map_err(|_| Error::Invalid("WebDAV network or TLS request failed"))
    }
    fn read(response: Response) -> Result<Vec<u8>> {
        if !response.status().is_success() {
            return Err(Error::SyncStatus(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_ATTACHMENT + 65536)
        {
            return Err(Error::Invalid("WebDAV response too large"));
        }
        let mut data = Vec::new();
        response
            .take(MAX_ATTACHMENT + 65537)
            .read_to_end(&mut data)?;
        if data.len() as u64 > MAX_ATTACHMENT + 65536 {
            return Err(Error::Invalid("WebDAV response too large"));
        }
        Ok(data)
    }
    pub(crate) fn get(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        let response = self.send("GET", path, None, false)?;
        if response.status().as_u16() == 404 {
            self.etags.insert(path.into(), None);
            return Ok(None);
        }
        let tag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(str::to_owned);
        let data = Self::read(response)?;
        // If content exists but has no ETag, reads are allowed, mutations are not.
        self.etags.insert(
            path.into(),
            Some(tag.unwrap_or_else(|| "W/unsupported".into())),
        );
        Ok(Some(data))
    }
    pub(crate) fn put(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let response = self.send("PUT", path, Some(data), true)?;
        Self::read(response)?;
        Ok(())
    }
    pub(crate) fn delete(&mut self, path: &str) -> Result<()> {
        if matches!(self.etags.get(path), Some(None)) {
            return Ok(());
        }
        if !self.etags.contains_key(path) {
            return Err(Error::Invalid("WebDAV delete requires a verified ETag"));
        }
        let response = self.send("DELETE", path, None, true)?;
        if response.status().as_u16() == 404 {
            return Ok(());
        }
        Self::read(response)?;
        Ok(())
    }
    pub(crate) fn list(&self, path: &str) -> Result<Vec<String>> {
        if self.started.elapsed() > Duration::from_secs(120) || self.cancel.load(Ordering::Relaxed)
        {
            return Err(Error::Invalid("sync time limit; retry to continue"));
        }
        let response=self.client.request(reqwest::Method::from_bytes(b"PROPFIND").expect("method"),self.url(path)?)
            .basic_auth(self.username.as_str(),Some(self.password.as_str()))
            .header("Depth","1").header("Content-Type","application/xml; charset=utf-8")
            .body("<?xml version=\"1.0\"?><d:propfind xmlns:d=\"DAV:\"><d:prop><d:resourcetype/></d:prop></d:propfind>")
            .send().map_err(|_|Error::Invalid("WebDAV listing failed"))?;
        let bytes = Self::read(response)?;
        list_names(&bytes, &self.url(path)?)
    }
    fn mkdir(&self, path: &str) -> Result<()> {
        let response = self.send("MKCOL", path, None, false)?;
        if response.status().is_success() || response.status().as_u16() == 405 {
            Ok(())
        } else {
            Err(Error::SyncStatus(response.status().as_u16()))
        }
    }
    fn lock_path(&self) -> String {
        format!("locks/2_1_{}.json", self.client_id)
    }
    fn check_other_locks(&self) -> Result<()> {
        for name in self.list("locks")? {
            if !name.ends_with(".json") || !(name.starts_with("1_") || name.starts_with("2_")) {
                continue;
            }
            if name == format!("2_1_{}.json", self.client_id) {
                continue;
            }
            let response = self.send("GET", &format!("locks/{name}"), None, false)?;
            if response.status().as_u16() == 404 {
                continue;
            }
            let data = Self::read(response)?;
            let value: serde_json::Value = serde_json::from_slice(&data)
                .map_err(|_| Error::Invalid("malformed WebDAV lock"))?;
            let updated = value["updatedTime"]
                .as_i64()
                .ok_or(Error::Invalid("WebDAV lock timestamp"))?;
            if now().saturating_sub(updated) < 180_000 {
                return Err(Error::Invalid(
                    "another Joplin client holds a synchronisation lock; retry later",
                ));
            }
        }
        Ok(())
    }
    fn write_lock(&self) -> Result<()> {
        let data=serde_json::to_vec(&serde_json::json!({"type":2,"clientType":1,"clientId":self.client_id,"updatedTime":now()})).map_err(|_|Error::Invalid("lock"))?;
        Self::read(self.send("PUT", &self.lock_path(), Some(&data), false)?)?;
        Ok(())
    }
    pub(crate) fn lock(&mut self) -> Result<()> {
        // Verify the selected collection is accessible. Do not create a root or account folder.
        self.list("")?;
        for path in ["locks", ".resource", "temp"] {
            self.mkdir(path)?;
        }
        self.check_other_locks()?;
        self.write_lock()?;
        self.held = true;
        // Recheck after publishing our lock, so competing clients cannot both proceed unnoticed.
        std::thread::sleep(Duration::from_millis(120));
        if let Err(e) = self.check_other_locks() {
            let _ = self.release();
            return Err(e);
        }
        self.renewed = Instant::now();
        Ok(())
    }
    pub(crate) fn tick(&mut self) -> Result<()> {
        if self.held && self.renewed.elapsed() > Duration::from_secs(30) {
            self.check_other_locks()?;
            self.write_lock()?;
            self.renewed = Instant::now();
        }
        Ok(())
    }
    pub(crate) fn release(&mut self) -> Result<()> {
        if !self.held {
            return Ok(());
        }
        self.held = false;
        let response = self.send("DELETE", &self.lock_path(), None, false)?;
        if response.status().as_u16() != 404 {
            Self::read(response)?;
        }
        Ok(())
    }
}
// Only immediate children of the selected collection may enter the sync engine.
fn list_names(xml: &[u8], base: &Url) -> Result<Vec<String>> {
    if xml.len() > 8 * 1024 * 1024 {
        return Err(Error::Invalid("WebDAV listing too large"));
    }
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut in_href = false;
    let mut href = String::new();
    let mut names = BTreeSet::new();
    loop {
        match reader
            .read_event()
            .map_err(|_| Error::Invalid("WebDAV XML"))?
        {
            Event::DocType(_) => return Err(Error::Invalid("WebDAV DTD refused")),
            Event::Start(e) if e.local_name().as_ref() == "href" => {
                in_href = true;
                href.clear();
            }
            Event::Text(e) if in_href => {
                href.push_str(
                    &quick_xml::escape::unescape(e.as_ref())
                        .map_err(|_| Error::Invalid("WebDAV href"))?,
                );
            }
            Event::GeneralRef(e) if in_href => {
                if let Some(ch) = e
                    .resolve_char_ref()
                    .map_err(|_| Error::Invalid("WebDAV character reference"))?
                {
                    href.push(ch);
                } else if let Some(value) = quick_xml::escape::resolve_predefined_entity(e.as_ref())
                {
                    href.push_str(value);
                } else {
                    return Err(Error::Invalid("WebDAV entity reference"));
                }
            }
            Event::End(e) if e.local_name().as_ref() == "href" => {
                in_href = false;
                let url = base
                    .join(&href)
                    .map_err(|_| Error::Invalid("WebDAV href URL"))?;
                if url.origin() != base.origin()
                    || url.query().is_some()
                    || url.fragment().is_some()
                {
                    return Err(Error::Invalid("WebDAV listing escaped selected origin"));
                }
                let basepath = base.path().trim_end_matches('/');
                let path = url.path().trim_end_matches('/');
                if path == basepath {
                    continue;
                }
                let prefix = format!("{basepath}/");
                let child = path
                    .strip_prefix(&prefix)
                    .ok_or(Error::Invalid("WebDAV listing escaped selected directory"))?;
                if child.contains('/')
                    || child.contains('%')
                    || child.contains('\\')
                    || child == ".."
                    || child.is_empty()
                {
                    return Err(Error::Invalid("unsafe WebDAV child"));
                }
                if !names.insert(child.to_string()) {
                    return Err(Error::Invalid("duplicate WebDAV child"));
                }
                if names.len() > 10000 {
                    return Err(Error::Invalid("WebDAV item limit"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(names.into_iter().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn listing_never_escapes_account_directory() {
        let base = Url::parse("https://example.org/remote.php/dav/files/user/Joplin/").unwrap();
        let xml=b"<d:multistatus xmlns:d='DAV:'><d:response><d:href>/remote.php/dav/files/user/Joplin/</d:href></d:response><d:response><d:href>/remote.php/dav/files/user/Joplin/abc.md</d:href></d:response></d:multistatus>";
        assert_eq!(list_names(xml, &base).unwrap(), vec!["abc.md"]);
        for path in [
            "https://evil.example/abc.md",
            "/remote.php/dav/files/other/Joplin/abc.md",
            "../abc.md",
            "%2e%2e%2fsecret",
            "a/b",
        ] {
            let xml =
                format!("<multistatus><response><href>{path}</href></response></multistatus>");
            assert!(list_names(xml.as_bytes(), &base).is_err(), "{path}");
        }
        assert!(list_names(b"<!DOCTYPE a><multistatus/>", &base).is_err());
    }
}
