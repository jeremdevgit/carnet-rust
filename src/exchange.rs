//! Bounded JEX reader; archive paths are never extracted to disk.
use crate::{
    Error, Result, id, now,
    store::{MAX_ATTACHMENT, MAX_BODY, Note, Store, valid_id, validate_note},
};
use rusqlite::params;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::{Read, Write},
    path::{Component, Path},
};

pub const MAX_ARCHIVE: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;

pub fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > max {
        return Err(Error::Invalid("file too large or not regular"));
    }
    let mut out = Vec::new();
    file.take(max + 1).read_to_end(&mut out)?;
    if out.len() as u64 > max {
        return Err(Error::Invalid("file too large"));
    }
    Ok(out)
}
pub fn write_new(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or(Error::Invalid("destination"))?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(data)?;
    f.as_file().sync_all()?;
    f.persist_noclobber(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
pub fn import_markdown(store: &mut Store, path: &Path, notebook: &str) -> Result<Note> {
    let bytes = read_bounded(path, MAX_BODY as u64)?;
    let body = String::from_utf8(bytes).map_err(|_| Error::Invalid("Markdown must be UTF-8"))?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or(Error::Invalid("filename"))?;
    let note = Note {
        id: id(),
        notebook_id: notebook.into(),
        title: title.into(),
        body,
        is_todo: false,
        completed: false,
        due: 0,
        created: now(),
        updated: now(),
        deleted: false,
        version: 0,
        extra: "{}".into(),
    };
    let tx = store.conn.transaction()?;
    Store::insert_note(&tx, &note)?;
    tx.commit()?;
    Ok(note)
}

#[derive(Debug)]
pub(crate) struct Item {
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) props: BTreeMap<String, String>,
}
impl Item {
    pub(crate) fn get(&self, k: &str) -> &str {
        self.props.get(k).map_or("", String::as_str)
    }
    pub(crate) fn number(&self, k: &str) -> Result<i64> {
        let s = self.get(k);
        if s.is_empty() {
            Ok(0)
        } else {
            s.parse().map_err(|_| Error::Invalid("numeric metadata"))
        }
    }
    pub(crate) fn timestamp(&self, k: &str) -> Result<i64> {
        let s = self.get(k);
        if s.is_empty() {
            return Ok(0);
        }
        if let Ok(n) = s.parse::<i64>() {
            return Ok(n);
        }
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|d| d.timestamp_millis())
            .map_err(|_| Error::Invalid("timestamp"))
    }
}
pub(crate) fn parse_item(text: &str) -> Result<Item> {
    // Joplin serialises the metadata block LAST, separated by a blank line.
    let normalized = text.replace("\r\n", "\n");
    let (front, props) = normalized
        .trim_end_matches('\n')
        .rsplit_once("\n\n")
        .unwrap_or(("", normalized.trim_end_matches('\n')));
    let mut map = BTreeMap::new();
    for line in props.lines() {
        let (k, v) = line.split_once(':').ok_or(Error::Invalid("JEX property"))?;
        if k.is_empty()
            || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || map.insert(k.into(), v.trim().into()).is_some()
        {
            return Err(Error::Invalid("JEX property name or duplicate"));
        }
    }
    let (title, body) = front.split_once("\n\n").unwrap_or((front, ""));
    let item = Item {
        title: title.into(),
        body: body.into(),
        props: map,
    };
    if !valid_id(item.get("id")) {
        return Err(Error::Invalid("JEX item identifier"));
    }
    if item.number("encryption_applied")? != 0 || !item.get("encryption_cipher_text").is_empty() {
        return Err(Error::Invalid(
            "encrypted JEX: export a decrypted JEX from Joplin first",
        ));
    }
    Ok(item)
}
fn safe_path(p: &Path) -> Result<String> {
    let s = p.to_str().ok_or(Error::Invalid("archive path encoding"))?;
    if s.contains(['\\', ':', '\0']) || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(Error::Invalid("archive path"));
    }
    Ok(s.into())
}

pub fn import_jex(store: &mut Store, path: &Path, fallback_title: &str) -> Result<usize> {
    let f = File::open(path)?;
    if !f.metadata()?.is_file() || f.metadata()?.len() > MAX_ARCHIVE {
        return Err(Error::Invalid("archive too large"));
    }
    let mut archive = tar::Archive::new(f.take(MAX_ARCHIVE + 1));
    let mut files = HashMap::new();
    let mut total = 0u64;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            return Err(Error::Invalid(
                "archive links and special files are forbidden",
            ));
        }
        let name = safe_path(&entry.path()?)?;
        let len = entry.size();
        total = total
            .checked_add(len)
            .ok_or(Error::Invalid("archive size overflow"))?;
        if len > MAX_ATTACHMENT || total > MAX_ARCHIVE || files.len() >= MAX_ENTRIES {
            return Err(Error::Invalid("archive limits"));
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        if files.insert(name, data).is_some() {
            return Err(Error::Invalid("duplicate archive path"));
        }
    }
    let mut items = Vec::new();
    let mut mapping = HashMap::new();
    let mut resource_paths = HashSet::new();
    for (name, data) in &files {
        if name.starts_with("resources/") {
            continue;
        }
        if name.contains('/') || !name.ends_with(".md") || data.len() > MAX_BODY + 16384 {
            return Err(Error::Invalid("unsupported JEX file"));
        }
        let item = parse_item(
            std::str::from_utf8(data).map_err(|_| Error::Invalid("JEX must be UTF-8"))?,
        )?;
        if name != &format!("{}.md", item.get("id")) {
            return Err(Error::Invalid("JEX filename / id mismatch"));
        }
        if mapping.insert(item.get("id").to_string(), id()).is_some() {
            return Err(Error::Invalid("duplicate JEX id"));
        }
        if ![1, 2, 4, 5, 6].contains(&item.number("type_")?) {
            return Err(Error::Invalid(
                "unsupported JEX item type; import cancelled without changes",
            ));
        }
        items.push(item);
    }
    if items.is_empty() {
        return Err(Error::Invalid("empty JEX"));
    }
    let tx = store.conn.transaction()?;
    let mut folder_ids = HashSet::new();
    let mut pending: Vec<_> = items.iter().filter(|i| i.get("type_") == "2").collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut next = Vec::new();
        for item in pending {
            let parent = item.get("parent_id");
            if !parent.is_empty() && !folder_ids.contains(parent) {
                next.push(item);
                continue;
            }
            if item.title.trim().is_empty()
                || item.title.len() > 1024
                || item.title.contains(['\n', '\r', '\0'])
            {
                return Err(Error::Invalid("notebook title"));
            }
            let parent = if parent.is_empty() {
                None
            } else {
                mapping.get(parent)
            };
            tx.execute(
                "INSERT INTO notebooks VALUES(?1,?2,?3)",
                params![mapping[item.get("id")], parent, item.title],
            )?;
            folder_ids.insert(item.get("id").to_string());
        }
        if next.len() == before {
            return Err(Error::Invalid("notebook cycle or missing parent"));
        }
        pending = next;
    }
    let orphan_id = id();
    let mut orphan_created = false;
    let mut count = 0;
    let note_ids: HashSet<_> = items
        .iter()
        .filter(|i| i.get("type_") == "1")
        .map(|i| i.get("id"))
        .collect();
    for item in items.iter().filter(|i| i.get("type_") == "1") {
        let notebook_id = if folder_ids.contains(item.get("parent_id")) {
            mapping[item.get("parent_id")].clone()
        } else {
            if !orphan_created {
                tx.execute(
                    "INSERT INTO notebooks VALUES(?1,NULL,?2)",
                    params![orphan_id, fallback_title],
                )?;
                orphan_created = true;
            }
            orphan_id.clone()
        };
        // References only use validated 32-character ids, never paths.
        let mut body = item.body.clone();
        for (old, new) in &mapping {
            body = body.replace(&format!(":/{old}"), &format!(":/{new}"));
        }
        let note = Note {
            id: mapping[item.get("id")].clone(),
            notebook_id,
            title: item.title.clone(),
            body,
            is_todo: item.number("is_todo")? != 0,
            completed: item.number("todo_completed")? > 0,
            due: item.timestamp("todo_due")?,
            created: item.timestamp("created_time")?,
            updated: item.timestamp("updated_time")?,
            deleted: item.timestamp("deleted_time")? > 0,
            version: 0,
            extra: serde_json::to_string(&item.props).map_err(|_| Error::Invalid("metadata"))?,
        };
        validate_note(&note)?;
        Store::insert_note(&tx, &note)?;
        count += 1;
    }
    for item in items.iter().filter(|i| i.get("type_") == "4") {
        let ext = item.get("file_extension");
        if ext.len() > 32 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::Invalid("resource extension"));
        }
        let resource_name = if ext.is_empty() {
            item.get("id").to_string()
        } else {
            format!("{}.{}", item.get("id"), ext)
        };
        let path = format!("resources/{resource_name}");
        let data = files
            .get(&path)
            .ok_or(Error::Invalid("missing resource payload"))?;
        resource_paths.insert(path);
        tx.execute(
            "INSERT INTO resources VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                mapping[item.get("id")],
                item.title,
                ext,
                item.get("mime"),
                data,
                serde_json::to_string(&item.props).map_err(|_| Error::Invalid("metadata"))?
            ],
        )?;
    }
    if files
        .keys()
        .any(|p| p.starts_with("resources/") && !resource_paths.contains(p))
    {
        return Err(Error::Invalid("unreferenced resource file"));
    }
    let mut tag_map = HashMap::new();
    for item in items.iter().filter(|i| i.get("type_") == "5") {
        if item.title.trim().is_empty()
            || item.title.len() > 256
            || item.title.contains([',', '\n', '\r', '\0'])
        {
            return Err(Error::Invalid("unsupported tag title"));
        }
        tx.execute(
            "INSERT OR IGNORE INTO tags VALUES(?1,?2)",
            params![mapping[item.get("id")], item.title],
        )?;
        let tag: String =
            tx.query_row("SELECT id FROM tags WHERE title=?1", [&item.title], |r| {
                r.get(0)
            })?;
        tag_map.insert(item.get("id"), tag);
    }
    for item in items.iter().filter(|i| i.get("type_") == "6") {
        if !note_ids.contains(item.get("note_id")) {
            return Err(Error::Invalid("missing tagged note"));
        }
        let tag = tag_map
            .get(item.get("tag_id"))
            .ok_or(Error::Invalid("missing tag"))?;
        tx.execute(
            "INSERT OR IGNORE INTO note_tags VALUES(?1,?2)",
            params![mapping[item.get("note_id")], tag],
        )?;
    }
    // Owning transaction is committed only after every item has been validated.
    tx.commit()?;
    Ok(count)
}

pub(crate) fn iso(ms: i64) -> Result<String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or(Error::Invalid("timestamp out of range"))
}
fn metadata(extra: &str) -> Result<BTreeMap<String, String>> {
    serde_json::from_str(extra).map_err(|_| Error::Invalid("stored metadata"))
}
pub(crate) fn serialize(
    title: &str,
    body: &str,
    props: BTreeMap<String, String>,
) -> Result<Vec<u8>> {
    let mut text = String::new();
    if !title.is_empty() {
        text.push_str(title);
        text.push_str("\n\n");
    }
    if !body.is_empty() {
        text.push_str(body);
        text.push_str("\n\n");
    }
    for (k, v) in props {
        if k.contains(['\n', '\r', ':']) || v.contains(['\n', '\r']) {
            return Err(Error::Invalid("metadata needs escaped newlines"));
        }
        text.push_str(&k);
        text.push_str(": ");
        text.push_str(&v);
        text.push('\n');
    }
    // Joplin reads the final metadata block backwards; a trailing blank line
    // would stop its parser before type_ and the other properties are reached.
    if text.ends_with('\n') {
        text.pop();
    }
    Ok(text.into_bytes())
}
fn append<W: Write>(tar: &mut tar::Builder<W>, name: &str, data: &[u8]) -> Result<()> {
    let mut h = tar::Header::new_gnu();
    h.set_size(data.len() as u64);
    h.set_mode(0o600);
    h.set_mtime(0);
    h.set_cksum();
    tar.append_data(&mut h, name, data)?;
    Ok(())
}
pub fn export_jex(store: &Store, path: &Path) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().ok_or(Error::Invalid("destination"))?)?;
    {
        let mut tar = tar::Builder::new(temp.as_file_mut());
        for n in store.notebooks()? {
            let props = BTreeMap::from([
                ("id".into(), n.id.clone()),
                ("parent_id".into(), n.parent_id.unwrap_or_default()),
                ("type_".into(), "2".into()),
                ("created_time".into(), iso(now())?),
                ("updated_time".into(), iso(now())?),
            ]);
            append(
                &mut tar,
                &format!("{}.md", n.id),
                &serialize(&n.title, "", props)?,
            )?;
        }
        for n in store.all_notes()? {
            let mut props = metadata(&n.extra)?;
            props.extend(BTreeMap::from([
                ("id".into(), n.id.clone()),
                ("parent_id".into(), n.notebook_id.clone()),
                ("type_".into(), "1".into()),
                ("created_time".into(), iso(n.created)?),
                ("updated_time".into(), iso(n.updated)?),
                ("is_todo".into(), i32::from(n.is_todo).to_string()),
                (
                    "todo_completed".into(),
                    if n.completed { n.updated } else { 0 }.to_string(),
                ),
                ("todo_due".into(), n.due.to_string()),
                (
                    "deleted_time".into(),
                    if n.deleted { n.updated } else { 0 }.to_string(),
                ),
                ("encryption_applied".into(), "0".into()),
                ("encryption_cipher_text".into(), String::new()),
            ]));
            append(
                &mut tar,
                &format!("{}.md", n.id),
                &serialize(&n.title, &n.body, props)?,
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
            let mut p = metadata(&extra)?;
            p.extend(BTreeMap::from([
                ("id".into(), id.clone()),
                ("type_".into(), "4".into()),
                ("file_extension".into(), ext.clone()),
                ("mime".into(), mime),
                ("size".into(), data.len().to_string()),
                ("encryption_applied".into(), "0".into()),
                ("encryption_cipher_text".into(), String::new()),
            ]));
            append(&mut tar, &format!("{id}.md"), &serialize(&title, "", p)?)?;
            let suffix = if ext.is_empty() {
                String::new()
            } else {
                format!(".{ext}")
            };
            append(&mut tar, &format!("resources/{id}{suffix}"), &data)?;
        }
        let mut s = store.conn.prepare("SELECT id,title FROM tags")?;
        for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, title) = r?;
            append(
                &mut tar,
                &format!("{id}.md"),
                &serialize(
                    &title,
                    "",
                    BTreeMap::from([("id".into(), id), ("type_".into(), "5".into())]),
                )?,
            )?;
        }
        let mut s = store.conn.prepare("SELECT note_id,tag_id FROM note_tags")?;
        for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (note, tag) = r?;
            let id = id();
            append(
                &mut tar,
                &format!("{id}.md"),
                &serialize(
                    "",
                    "",
                    BTreeMap::from([
                        ("id".into(), id),
                        ("type_".into(), "6".into()),
                        ("note_id".into(), note),
                        ("tag_id".into(), tag),
                    ]),
                )?,
            )?;
        }
        tar.finish()?;
    }
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path)
        .map_err(|e| Error::Io(e.error))?;
    Ok(())
}
