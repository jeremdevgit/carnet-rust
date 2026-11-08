use carnet_rust::{
    Error, exchange, i18n,
    store::{MAX_ATTACHMENT, MAX_BODY, Store},
};
use std::{fs, path::Path};

fn sample() -> (Store, String) {
    let s = Store::memory().unwrap();
    let id = s.create_notebook("Demonstration", None).unwrap();
    (s, id)
}
fn archive(path: &Path, files: &[(&str, &[u8])]) {
    let f = fs::File::create(path).unwrap();
    let mut t = tar::Builder::new(f);
    for (p, b) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(b.len() as u64);
        h.set_mode(0o600);
        h.set_cksum();
        t.append_data(&mut h, p, *b).unwrap();
    }
    t.finish().unwrap();
}
#[test]
fn save_search_tags_history_and_trash() {
    let (mut s, b) = sample();
    let mut n = s.create_note(&b, "Planning", true).unwrap();
    n.body = "A durable note, 中文 日本語 ' OR 1=1 --".into();
    n.completed = true;
    n.due = 1_800_000_000_000;
    s.save(&mut n, "work, urgent, work").unwrap();
    assert_eq!(s.tags(&n.id).unwrap(), "urgent, work");
    assert_eq!(
        s.list(None, "日本語", false, false, None, 0).unwrap().len(),
        1
    );
    assert!(s.list(None, "%", false, false, None, 0).unwrap().is_empty());
    assert_eq!(s.full_text("durable").unwrap(), vec![n.id.clone()]);
    assert_eq!(
        s.list(None, "", false, true, Some("urgent"), 0)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(s.revisions(&n.id).unwrap().len(), 1);
    let saved = s.note(&n.id).unwrap();
    assert_eq!(saved, n);
    n.deleted = true;
    s.save(&mut n, "work").unwrap();
    assert!(s.list(None, "", false, false, None, 0).unwrap().is_empty());
    assert!(s.full_text("durable").unwrap().is_empty());
    assert_eq!(s.list(None, "", true, false, None, 0).unwrap().len(), 1);
    n.deleted = false;
    s.save(&mut n, "work").unwrap();
    assert_eq!(s.list(None, "", false, false, None, 0).unwrap().len(), 1);
}
#[test]
fn stale_draft_cannot_overwrite_newer_edit() {
    let (mut s, b) = sample();
    let mut a = s.create_note(&b, "Example", false).unwrap();
    let mut stale = a.clone();
    a.body = "Newer edit".into();
    s.save(&mut a, "").unwrap();
    stale.body = "Old draft".into();
    assert!(matches!(s.save(&mut stale, ""), Err(Error::Conflict)));
    assert_eq!(s.note(&a.id).unwrap().body, "Newer edit");
    assert_eq!(stale.body, "Old draft");
}
#[test]
fn failed_validation_keeps_previous_note_and_tags() {
    let (mut s, b) = sample();
    let mut n = s.create_note(&b, "Example", false).unwrap();
    s.save(&mut n, "before").unwrap();
    let old = s.note(&n.id).unwrap();
    n.body = "x".repeat(MAX_BODY + 1);
    assert!(s.save(&mut n, "after").is_err());
    assert_eq!(s.note(&n.id).unwrap(), old);
    assert_eq!(s.tags(&n.id).unwrap(), "before");
}
#[test]
fn missing_notebook_rolls_back_revision_and_search() {
    let (mut s, b) = sample();
    let mut n = s.create_note(&b, "Example", false).unwrap();
    let original = n.clone();
    n.body = "must not persist".into();
    n.notebook_id = "f".repeat(32);
    assert!(s.save(&mut n, "").is_err());
    assert_eq!(s.note(&n.id).unwrap(), original);
    assert!(s.revisions(&n.id).unwrap().is_empty());
    assert!(s.full_text("persist").unwrap().is_empty());
}
#[test]
fn database_lock_and_restart_persistence() {
    let d = tempfile::tempdir().unwrap();
    let id;
    {
        let mut s = Store::open(d.path(), "Demo passphrase for tests").unwrap();
        assert!(Store::open(d.path(), "Demo passphrase for tests").is_err());
        let b = s.create_notebook("Example", None).unwrap();
        let n = s.create_note(&b, "Persisted", false).unwrap();
        id = n.id;
    }
    let s = Store::open(d.path(), "Demo passphrase for tests").unwrap();
    assert_eq!(s.note(&id).unwrap().title, "Persisted");
}
#[test]
fn backup_is_consistent_and_never_clobbers_source() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::open(&d.path().join("profile"), "Demo passphrase for tests").unwrap();
    let b = s.create_notebook("Example", None).unwrap();
    let n = s.create_note(&b, "Preserved", false).unwrap();
    let p = d.path().join("backup.sqlite3");
    s.backup(&p).unwrap();
    let db = carnet_rust::vault::open_database(&p, "Demo passphrase for tests").unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(s.backup(&p).is_err());
    assert!(s.backup(&d.path().join("profile/notes.sqlite3")).is_err());
    assert_eq!(s.note(&n.id).unwrap().title, "Preserved");
}
#[test]
fn jex_round_trip_with_nested_notebooks_tags_todo_attachment_and_unknown_fields() {
    let (mut s, root) = sample();
    let child = s.create_notebook("Child", Some(&root)).unwrap();
    let mut note = s.create_note(&child, "日本語 Example", true).unwrap();
    let resource = s
        .add_attachment("example.txt", b"Example attachment")
        .unwrap();
    note.body =
        format!("## A heading\n\n[Attachment](:/{resource})\n\nmetadata-like text\ntype_: 99\n\n");
    note.extra=r#"{"source_url":"https://example.org","application_data":"{\"demo\":true}","markup_language":"1"}"#.into();
    note.completed = true;
    note.due = 1_800_000_000_000;
    s.save(&mut note, "demo, sample").unwrap();
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("export.jex");
    exchange::export_jex(&s, &p).unwrap();
    assert!(exchange::export_jex(&s, &p).is_err());
    let mut dest = Store::memory().unwrap();
    assert_eq!(exchange::import_jex(&mut dest, &p, "Imported").unwrap(), 1);
    assert_eq!(dest.notebooks().unwrap().len(), 2);
    let id = dest.list(None, "", false, false, None, 0).unwrap()[0]
        .id
        .clone();
    let n = dest.note(&id).unwrap();
    assert_ne!(n.id, note.id);
    assert_eq!(n.title, note.title);
    assert!(n.completed);
    assert_eq!(n.due, note.due);
    assert_eq!(n.created, note.created);
    assert_eq!(n.updated, note.updated);
    assert!(n.body.ends_with("type_: 99\n\n"));
    let attachments = dest.attachments(&n.body).unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].data, b"Example attachment");
    assert_eq!(dest.tags(&id).unwrap(), "demo, sample");
    assert!(n.extra.contains("source_url"));
    // A second import must duplicate notes safely and keep internal links within the new import.
    assert_eq!(exchange::import_jex(&mut dest, &p, "Imported").unwrap(), 1);
    assert_eq!(dest.list(None, "", false, false, None, 0).unwrap().len(), 2);
}
#[test]
fn jex_note_without_folder_goes_to_new_imported_notebook() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("note.jex");
    let id = "a".repeat(32);
    let text = format!(
        "Example\n\nBody\n\nid: {id}\nparent_id: {}\ntype_: 1",
        "b".repeat(32)
    );
    archive(&p, &[(&format!("{id}.md"), text.as_bytes())]);
    let mut s = Store::memory().unwrap();
    assert_eq!(exchange::import_jex(&mut s, &p, "Imported").unwrap(), 1);
    assert_eq!(s.notebooks().unwrap()[0].title, "Imported");
}
#[test]
fn malformed_jex_rolls_back_everything() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("bad.jex");
    let a = "a".repeat(32);
    let b = "b".repeat(32);
    let folder = format!("Example\n\nid: {a}\ntype_: 2");
    let broken = format!(
        "Too large\n\n{}\n\nid: {b}\nparent_id: {a}\ntype_: 1",
        "x".repeat(MAX_BODY + 1)
    );
    archive(
        &p,
        &[
            (&format!("{a}.md"), folder.as_bytes()),
            (&format!("{b}.md"), broken.as_bytes()),
        ],
    );
    let mut s = Store::memory().unwrap();
    assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
    assert!(s.notebooks().unwrap().is_empty());
    assert!(s.list(None, "", false, false, None, 0).unwrap().is_empty());
}
#[test]
fn encrypted_and_unsupported_items_are_not_silently_dropped() {
    let d = tempfile::tempdir().unwrap();
    let a = "a".repeat(32);
    for extra in [
        "type_: 1\nencryption_applied: 1\nencryption_cipher_text: Secret",
        "type_: 13",
    ] {
        let p = d.path().join("bad.jex");
        let text = format!("Example\n\nid: {a}\n{extra}");
        archive(&p, &[(&format!("{a}.md"), text.as_bytes())]);
        let mut s = Store::memory().unwrap();
        assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
        assert!(s.notebooks().unwrap().is_empty());
    }
}
#[test]
fn jex_symlink_is_rejected_without_extraction() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("link.jex");
    let mut t = tar::Builder::new(fs::File::create(&p).unwrap());
    let mut h = tar::Header::new_gnu();
    h.set_entry_type(tar::EntryType::Symlink);
    h.set_size(0);
    h.set_mode(0o600);
    h.set_cksum();
    t.append_link(&mut h, "escape", "../outside").unwrap();
    t.finish().unwrap();
    drop(t);
    let mut s = Store::memory().unwrap();
    assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
    assert!(!d.path().join("outside").exists());
}
#[test]
fn windows_path_and_duplicate_archive_entries_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    for files in [
        vec![("C:\\outside.md", b"ignored".as_slice())],
        vec![("same.md", b"x".as_slice()), ("same.md", b"y".as_slice())],
    ] {
        let p = d.path().join("bad.jex");
        archive(&p, &files);
        let mut s = Store::memory().unwrap();
        assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
    }
}
#[test]
fn missing_resource_payload_rolls_back_notes_and_notebooks() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("missing.jex");
    let a = "a".repeat(32);
    let b = "b".repeat(32);
    let note = format!("Example\n\n[File](:/{b})\n\nid: {a}\ntype_: 1");
    let resource = format!("File\n\nid: {b}\nfile_extension: txt\ntype_: 4");
    archive(
        &p,
        &[
            (&format!("{a}.md"), note.as_bytes()),
            (&format!("{b}.md"), resource.as_bytes()),
        ],
    );
    let mut s = Store::memory().unwrap();
    assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
    assert!(s.notebooks().unwrap().is_empty());
}
#[test]
fn cycle_in_notebooks_cannot_hang_import() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("cycle.jex");
    let a = "a".repeat(32);
    let b = "b".repeat(32);
    let one = format!("One\n\nid: {a}\nparent_id: {b}\ntype_: 2");
    let two = format!("Two\n\nid: {b}\nparent_id: {a}\ntype_: 2");
    archive(
        &p,
        &[
            (&format!("{a}.md"), one.as_bytes()),
            (&format!("{b}.md"), two.as_bytes()),
        ],
    );
    let mut s = Store::memory().unwrap();
    assert!(exchange::import_jex(&mut s, &p, "Imported").is_err());
    assert!(s.notebooks().unwrap().is_empty());
}
#[test]
fn markdown_import_is_atomic_and_utf8_only() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("Example.md");
    let (mut s, b) = sample();
    fs::write(&p, b"# Example\n\nBody").unwrap();
    let n = exchange::import_markdown(&mut s, &p, &b).unwrap();
    assert_eq!(n.title, "Example");
    fs::write(&p, [0xff, 0xfe]).unwrap();
    assert!(exchange::import_markdown(&mut s, &p, &b).is_err());
    assert_eq!(s.list(None, "", false, false, None, 0).unwrap().len(), 1);
}
#[test]
fn export_and_attachment_limits() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("existing");
    fs::write(&p, b"before").unwrap();
    assert!(exchange::write_new(&p, b"after").is_err());
    assert_eq!(fs::read(&p).unwrap(), b"before");
    let (s, _) = sample();
    assert!(
        s.add_attachment("x.txt", &vec![0; MAX_ATTACHMENT as usize + 1])
            .is_err()
    );
    assert!(s.add_attachment("x.unsafe/extension", b"x").is_ok()); // basename has no extension: stored as bytes only
}
#[test]
fn translations_are_complete_and_locale_fallbacks_work() {
    let mut keys = std::collections::HashSet::new();
    for row in i18n::TEXTS {
        assert!(keys.insert(row[0]));
        assert!(row.iter().all(|v| !v.trim().is_empty()));
    }
    assert_eq!(i18n::language("pt-BR"), 5);
    assert_eq!(i18n::language("ja_JP"), 7);
    assert_eq!(i18n::language("nl-NL"), 1);
}

#[test]
fn notebook_tree_keeps_children_below_their_parent() {
    let s = Store::memory().unwrap();
    let root = s.create_notebook("Studio", None).unwrap();
    let child = s.create_notebook("Alpha", Some(&root)).unwrap();
    let sibling = s.create_notebook("Zulu", None).unwrap();
    let books = s.notebooks().unwrap();
    assert_eq!(
        books.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
        vec![root.as_str(), child.as_str(), sibling.as_str()]
    );
}
#[test]
fn preview_never_forwards_image_paths_to_a_loader() {
    for body in [
        "![image](https://example.invalid/image.png)",
        "![image](file:///etc/passwd)",
        "![image](:/0123456789abcdef0123456789abcdef)",
        "![image][x]\n\n[x]: https://example.invalid/image.png",
    ] {
        let rendered = carnet_rust::preview_markdown(body);
        assert!(
            pulldown_cmark::Parser::new_ext(&rendered, pulldown_cmark::Options::all()).all(
                |e| !matches!(
                    e,
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { .. })
                )
            )
        );
    }
}

#[test]
fn moving_notebooks_cannot_create_a_cycle() {
    let mut s = Store::memory().unwrap();
    let root = s.create_notebook("Root", None).unwrap();
    let child = s.create_notebook("Child", Some(&root)).unwrap();
    assert!(s.move_notebook(&root, Some(&child)).is_err());
    assert!(s.move_notebook(&child, Some(&child)).is_err());
    let note = s.create_note(&child, "Preserved", false).unwrap();
    s.move_notebook(&child, None).unwrap();
    assert_eq!(s.note(&note.id).unwrap().notebook_id, child);
}
#[test]
fn removing_a_notebook_preserves_notes_and_child_notebooks() {
    let mut s = Store::memory().unwrap();
    let root = s.create_notebook("Root", None).unwrap();
    let child = s.create_notebook("Child", Some(&root)).unwrap();
    let target = s.create_notebook("Destination", None).unwrap();
    let note = s.create_note(&root, "Preserved", false).unwrap();
    assert!(s.remove_notebook(&root, &root).is_err());
    s.remove_notebook(&root, &target).unwrap();
    assert_eq!(s.note(&note.id).unwrap().notebook_id, target);
    assert!(
        s.notebooks()
            .unwrap()
            .iter()
            .find(|b| b.id == child)
            .unwrap()
            .parent_id
            .is_none()
    );
}
#[test]
fn merging_and_removing_tags_preserves_notes_and_rejects_stale_edits() {
    let (mut s, book) = sample();
    let mut n = s.create_note(&book, "Example", false).unwrap();
    n.body = "Body".into();
    s.save(&mut n, "one, two").unwrap();
    s.rename_tag("one", "two").unwrap();
    assert_eq!(s.tags(&n.id).unwrap(), "two");
    assert!(s.save(&mut n, "one").is_err());
    s.remove_tag("two").unwrap();
    assert_eq!(s.tags(&n.id).unwrap(), "");
    assert_eq!(s.note(&n.id).unwrap().body, "Body");
}
#[test]
fn purge_removes_only_trash_and_cleanup_protects_history_and_drafts() {
    let (mut s, book) = sample();
    let old = s.add_attachment("old.txt", b"keep via history").unwrap();
    let orphan = s.add_attachment("unused.txt", b"remove").unwrap();
    let mut n = s.create_note(&book, "Keep", false).unwrap();
    n.body = format!("[file](:/{old})");
    s.save(&mut n, "").unwrap();
    n.body = "Current body".into();
    s.save(&mut n, "").unwrap();
    let draft_attachment = s
        .add_attachment("draft.txt", b"keep via recovery draft")
        .unwrap();
    let mut draft = n.clone();
    draft.body = format!(":/{draft_attachment}");
    s.save_recovery_draft(&draft, "", "").unwrap();
    assert_eq!(s.clean_unreferenced_attachments().unwrap(), 1);
    assert!(s.attachment_data(&orphan).is_err());
    assert!(s.attachment_data(&old).is_ok());
    assert!(s.attachment_data(&draft_attachment).is_ok());
    let mut trash = s.create_note(&book, "Trash", false).unwrap();
    trash.body = "Delete me".into();
    s.save(&mut trash, "tag").unwrap();
    trash.deleted = true;
    s.save(&mut trash, "tag").unwrap();
    assert_eq!(s.purge_trash().unwrap(), 1);
    assert!(s.note(&trash.id).is_err());
    assert!(s.note(&n.id).is_ok());
    assert!(s.full_text("Delete").unwrap().is_empty());
}
