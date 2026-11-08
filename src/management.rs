use crate::{Error, Result, id, now, store::Store};
use rusqlite::{OptionalExtension, params};

impl Store {
    pub fn move_notebook(&self, book: &str, parent: Option<&str>) -> Result<()> {
        let books = self.notebooks()?;
        if !books.iter().any(|b| b.id == book) {
            return Err(Error::Invalid("notebook"));
        }
        let mut cursor = parent;
        let mut visited = std::collections::HashSet::new();
        while let Some(p) = cursor {
            if p == book || !visited.insert(p) {
                return Err(Error::Invalid("notebook cycle"));
            }
            cursor = books
                .iter()
                .find(|b| b.id == p)
                .ok_or(Error::Invalid("parent notebook"))?
                .parent_id
                .as_deref();
        }
        self.conn.execute(
            "UPDATE notebooks SET parent_id=?2 WHERE id=?1",
            params![book, parent],
        )?;
        self.touch_notebook(book)?;
        Ok(())
    }
    /// Removes only the selected notebook; notes and child notebooks are preserved.
    pub fn remove_notebook(&mut self, book: &str, destination: &str) -> Result<()> {
        if book == destination {
            return Err(Error::Invalid("destination notebook"));
        }
        let books = self.notebooks()?;
        if !books.iter().any(|b| b.id == book) || !books.iter().any(|b| b.id == destination) {
            return Err(Error::Invalid("notebook"));
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE notes SET notebook_id=?2,version=version+1,updated=?3 WHERE notebook_id=?1",
            params![book, destination, now()],
        )?;
        tx.execute("UPDATE notebooks SET parent_id=(SELECT parent_id FROM notebooks WHERE id=?1) WHERE parent_id=?1",[book])?;
        tx.execute("DELETE FROM notebooks WHERE id=?1", [book])?;
        tx.commit()?;
        Ok(())
    }
    pub fn rename_tag(&mut self, old: &str, new: &str) -> Result<()> {
        let new = new.trim();
        if new.is_empty() || new.len() > 256 || new.contains([',', '\n', '\r', '\0']) {
            return Err(Error::Invalid("tag title"));
        }
        if old == new {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        let old_id: Option<String> = tx
            .query_row("SELECT id FROM tags WHERE title=?1", [old], |r| r.get(0))
            .optional()?;
        let old_id = old_id.ok_or(Error::Invalid("tag"))?;
        tx.execute(
            "INSERT OR IGNORE INTO tags VALUES(?1,?2)",
            params![id(), new],
        )?;
        let new_id: String =
            tx.query_row("SELECT id FROM tags WHERE title=?1", [new], |r| r.get(0))?;
        tx.execute("UPDATE notes SET version=version+1,updated=?2 WHERE id IN (SELECT note_id FROM note_tags WHERE tag_id=?1)",params![old_id,now()])?;
        tx.execute(
            "INSERT OR IGNORE INTO note_tags SELECT note_id,?2 FROM note_tags WHERE tag_id=?1",
            params![old_id, new_id],
        )?;
        tx.execute("DELETE FROM note_tags WHERE tag_id=?1", [&old_id])?;
        tx.execute("DELETE FROM tags WHERE id=?1", [old_id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn remove_tag(&mut self, title: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("UPDATE notes SET version=version+1,updated=?2 WHERE id IN (SELECT nt.note_id FROM note_tags nt JOIN tags t ON t.id=nt.tag_id WHERE t.title=?1)",params![title,now()])?;
        tx.execute(
            "DELETE FROM note_tags WHERE tag_id IN (SELECT id FROM tags WHERE title=?1)",
            [title],
        )?;
        tx.execute("DELETE FROM tags WHERE title=?1", [title])?;
        tx.commit()?;
        Ok(())
    }
    pub fn purge_trash(&mut self) -> Result<usize> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM revisions WHERE note_id IN (SELECT id FROM notes WHERE deleted=1)",
            [],
        )?;
        tx.execute(
            "DELETE FROM drafts WHERE note_id IN (SELECT id FROM notes WHERE deleted=1)",
            [],
        )?;
        tx.execute(
            "DELETE FROM search WHERE note_id IN (SELECT id FROM notes WHERE deleted=1)",
            [],
        )?;
        let count = tx.execute("DELETE FROM notes WHERE deleted=1", [])?;
        tx.commit()?;
        Ok(count)
    }
    /// Conservative reachability: protects attachment references in note metadata,
    /// revision snapshots and unsaved recovery drafts, including trash.
    pub fn clean_unreferenced_attachments(&self) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM resources WHERE
            NOT EXISTS (SELECT 1 FROM notes n WHERE instr(n.body,resources.id)>0 OR instr(n.extra,resources.id)>0)
            AND NOT EXISTS (SELECT 1 FROM revisions r WHERE instr(r.body,resources.id)>0)
            AND NOT EXISTS (SELECT 1 FROM drafts d WHERE instr(d.data,resources.id)>0)",[])?)
    }
}
