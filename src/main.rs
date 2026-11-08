// SPDX-License-Identifier: GPL-3.0-or-later — Copyright (C) 2026 jeremdevgit
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use carnet_rust::{
    Error, Result, exchange,
    i18n::{self, tr},
    store::{MAX_ATTACHMENT, MAX_RESULTS, Note, NoteSummary, Notebook, Store},
};
use eframe::egui;
use zeroize::{Zeroize, Zeroizing};
mod desktop;
mod management_ui;
mod media_ui;
mod sync_ui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

struct App {
    store: Store,
    lang: usize,
    dark: bool,
    notebooks: Vec<Notebook>,
    notes: Vec<NoteSummary>,
    active: Option<Note>,
    notebook: Option<String>,
    query: String,
    tag: Option<String>,
    tags: String,
    due: String,
    trash: bool,
    todo: bool,
    offset: usize,
    dirty: bool,
    changed: Instant,
    retry_save: bool,
    error: Option<String>,
    success: bool,
    preview: bool,
    cache: CommonMarkCache,
    media: media_ui::PreviewCache,
    settings: bool,
    about: bool,
    history: bool,
    confirm_trash: bool,
    notebook_dialog: bool,
    notebook_name: String,
    notebook_parent: Option<String>,
    rename: Option<String>,
    lock_requested: bool,
    auto_lock_minutes: u32,
    password_dialog: bool,
    current_password: Zeroizing<String>,
    new_password: Zeroizing<String>,
    confirm_password: Zeroizing<String>,
    restore_source: Option<PathBuf>,
    restore_password: Zeroizing<String>,
    export_confirm: Option<usize>,
    sync_task: Option<sync_ui::Task>,
    sync_config: carnet_rust::sync::Config,
    sync_edit: carnet_rust::sync::Config,
    sync_dialog: bool,
    conflicts_dialog: bool,
    sync_status: Option<String>,
    next_sync: Instant,
    backup_destination: Option<PathBuf>,
    backup_password: Zeroizing<String>,
    backup_confirm: Zeroizing<String>,
    manage_book: Option<String>,
    book_delete: bool,
    book_target: Option<String>,
    manage_tag: Option<String>,
    tag_delete: bool,
    tag_title: String,
    confirm_purge: bool,
    confirm_gc: bool,
}
impl App {
    fn from_store(store: Store, ctx: &egui::Context) -> Result<Self> {
        let lang = i18n::language(
            &store
                .setting("language")?
                .unwrap_or_else(|| sys_locale::get_locale().unwrap_or_else(|| "en".into())),
        );
        let dark = store.setting("dark")?.as_deref() != Some("false");
        configure_fonts(ctx);
        set_glass_style(ctx, dark);
        let sync_config = carnet_rust::sync::load_config(&store).unwrap_or_default();
        if store.notebooks()?.is_empty() && sync_config.server.is_empty() {
            store.create_notebook(tr("Carnets", lang), None)?;
        }
        let auto_lock_minutes = store
            .setting("auto_lock_minutes")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(5u32)
            .clamp(0, 60);
        let mut app = Self {
            store,
            lang,
            dark,
            notebooks: vec![],
            notes: vec![],
            active: None,
            notebook: None,
            query: String::new(),
            tag: None,
            tags: String::new(),
            due: String::new(),
            trash: false,
            todo: false,
            offset: 0,
            dirty: false,
            changed: Instant::now(),
            retry_save: true,
            error: None,
            success: false,
            preview: false,
            cache: CommonMarkCache::default(),
            media: media_ui::PreviewCache::default(),
            settings: false,
            about: false,
            history: false,
            confirm_trash: false,
            notebook_dialog: false,
            notebook_name: String::new(),
            notebook_parent: None,
            rename: None,
            lock_requested: false,
            auto_lock_minutes,
            password_dialog: false,
            current_password: Zeroizing::new(String::new()),
            new_password: Zeroizing::new(String::new()),
            confirm_password: Zeroizing::new(String::new()),
            restore_source: None,
            restore_password: Zeroizing::new(String::new()),
            export_confirm: None,
            sync_task: None,
            sync_edit: sync_config.clone(),
            sync_config,
            sync_dialog: false,
            conflicts_dialog: false,
            sync_status: None,
            next_sync: Instant::now() + Duration::from_secs(2),
            backup_destination: None,
            backup_password: Zeroizing::new(String::new()),
            backup_confirm: Zeroizing::new(String::new()),
            manage_book: None,
            book_delete: false,
            book_target: None,
            manage_tag: None,
            tag_delete: false,
            tag_title: String::new(),
            confirm_purge: false,
            confirm_gc: false,
        };
        app.refresh()?;
        if let Some(id) = app.store.first_draft_id()? {
            app.select(&id);
        }
        Ok(app)
    }
    fn refresh(&mut self) -> Result<()> {
        self.notebooks = self.store.notebooks()?;
        self.notes = self.store.list(
            self.notebook.as_deref(),
            &self.query,
            self.trash,
            self.todo,
            self.tag.as_deref(),
            self.offset,
        )?;
        Ok(())
    }
    fn fail(&mut self, e: impl std::fmt::Display) {
        self.error = Some(e.to_string());
        self.success = false;
        self.retry_save = false;
    }
    fn changed(&mut self) {
        self.dirty = true;
        self.changed = Instant::now();
        self.retry_save = true;
        self.success = false;
    }
    fn save_draft(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let due = if self.due.trim().is_empty() {
            0
        } else {
            match chrono::NaiveDate::parse_from_str(&self.due, "%Y-%m-%d") {
                Ok(d) => d
                    .and_hms_opt(12, 0, 0)
                    .map(|d| d.and_utc().timestamp_millis())
                    .unwrap_or(0),
                Err(_) => {
                    self.fail(tr("Échéance invalide", self.lang).to_string());
                    return false;
                }
            }
        };
        if let Some(n) = &mut self.active {
            n.due = due;
            if let Err(e) = self.store.save(n, &self.tags) {
                self.fail(e);
                return false;
            }
        }
        self.dirty = false;
        self.error = None;
        self.success = true;
        if let Err(e) = self.refresh() {
            self.fail(e);
        }
        true
    }
    fn select(&mut self, id: &str) {
        if !self.save_draft() {
            return;
        }
        match self
            .store
            .note(id)
            .and_then(|n| Ok((self.store.tags(id)?, n)))
        {
            Ok((tags, n)) => {
                self.tags = tags;
                self.due = if n.due == 0 {
                    String::new()
                } else {
                    chrono::DateTime::from_timestamp_millis(n.due)
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                };
                self.active = Some(n);
                match self.store.recovery_draft(id) {
                    Ok(Some((note, tags, due))) => {
                        self.active = Some(note);
                        self.tags = tags;
                        self.due = due;
                        self.dirty = true;
                        self.retry_save = false;
                    }
                    Ok(None) => {}
                    Err(e) => self.fail(e),
                }
                self.preview = false;
                self.cache = CommonMarkCache::default();
                self.media.key.clear();
            }
            Err(e) => self.fail(e),
        }
    }
    fn create_note(&mut self, todo: bool) {
        if !self.save_draft() {
            return;
        }
        if self.notebooks.is_empty()
            && let Err(e) = self
                .store
                .create_notebook(tr("Carnets", self.lang), None)
                .and_then(|_| self.refresh())
        {
            self.fail(e);
            return;
        }
        let notebook = self
            .notebook
            .clone()
            .or_else(|| self.notebooks.first().map(|n| n.id.clone()));
        if let Some(notebook) = notebook {
            match self.store.create_note(
                &notebook,
                tr(
                    if todo {
                        "Nouvelle tâche"
                    } else {
                        "Nouvelle note"
                    },
                    self.lang,
                ),
                todo,
            ) {
                Ok(n) => {
                    self.trash = false;
                    self.offset = 0;
                    self.query.clear();
                    self.tag = None;
                    self.todo = false;
                    self.notebook = Some(notebook);
                    self.active = Some(n);
                    self.tags.clear();
                    self.due.clear();
                    if let Err(e) = self.refresh() {
                        self.fail(e);
                    }
                }
                Err(e) => self.fail(e),
            }
        }
    }
    fn change_filter(
        &mut self,
        notebook: Option<String>,
        trash: bool,
        todo: bool,
        tag: Option<String>,
    ) {
        if !self.save_draft() {
            return;
        }
        self.notebook = notebook;
        self.trash = trash;
        self.todo = todo;
        self.tag = tag;
        self.offset = 0;
        if let Err(e) = self.refresh() {
            self.fail(e);
        }
    }
    fn run_file(&mut self, action: usize) {
        if !self.save_draft() {
            return;
        }
        let lang = self.lang;
        let result: Result<()> = (|| {
            match action {
                0 => {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Markdown", &["md", "markdown"])
                        .pick_file()
                    {
                        let book = self
                            .notebook
                            .clone()
                            .or_else(|| self.notebooks.first().map(|n| n.id.clone()))
                            .ok_or(Error::Invalid("notebook"))?;
                        let n = exchange::import_markdown(&mut self.store, &p, &book)?;
                        self.change_filter(Some(book), false, false, None);
                        self.select(&n.id);
                    }
                }
                1 => {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("JEX", &["jex"])
                        .pick_file()
                    {
                        exchange::import_jex(&mut self.store, &p, tr("Importé", lang))?;
                        self.change_filter(None, false, false, None);
                    }
                }
                2 => {
                    if let Some(n) = &self.active
                        && let Some(p)=rfd::FileDialog::new().set_title(tr("Choisissez un nouveau nom de fichier : les fichiers existants sont protégés.",lang)).set_file_name("note.md").add_filter("Markdown",&["md"]).save_file() {
                        exchange::write_new(&p,n.body.as_bytes())?;
                    }
                }
                3 => {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_file_name("carnet-export.jex")
                        .add_filter("JEX", &["jex"])
                        .save_file()
                    {
                        exchange::export_jex(&self.store, &p)?;
                    }
                }
                4 => {
                    if let Some(p) = rfd::FileDialog::new()
                        .set_file_name("carnet-backup.sqlite3")
                        .add_filter("SQLite", &["sqlite3"])
                        .save_file()
                    {
                        self.backup_destination = Some(p);
                    }
                }
                5 => {
                    if self.active.is_some()
                        && let Some(p) = rfd::FileDialog::new().pick_file() {
                            let data = exchange::read_bounded(&p, MAX_ATTACHMENT)?;
                            let title = p
                                .file_name()
                                .and_then(|n| n.to_str())
                                .ok_or(Error::Invalid("filename"))?;
                            let id = self.store.add_attachment(title, &data)?;
                            if let Some(n) = &mut self.active {
                                // Escape Markdown punctuation from a user-controlled filename.
                                let safe = title
                                    .replace('\\', "\\\\")
                                    .replace('[', "\\[")
                                    .replace(']', "\\]")
                                    .replace(['\n', '\r'], " ");
                                n.body.push_str(&format!("\n[{safe}](:/{id})\n"));
                                self.changed();
                            }
                        }
                }
                _ => {}
            }
            self.refresh()?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.success = true;
                self.error = None;
            }
            Err(e) => self.fail(e),
        }
    }
    fn top(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::top("toolbar")
            .frame(glass_frame(self.dark))
            .show(ui, |ui| {
                glass_reflection(ui, self.dark);
                ui.horizontal_wrapped(|ui| {
                    brand_logo(ui, 30.0);
                    ui.heading("Carnet de notes");
                    ui.menu_button(tr("Fichiers", lang), |ui| {
                        for (i, key) in [
                            "Importer Markdown",
                            "Importer JEX",
                            "Exporter Markdown",
                            "Exporter JEX",
                            "Sauvegarder la base",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if ui.button(tr(key, lang)).clicked() {
                                ui.close();
                                if matches!(i, 2 | 3) {
                                    self.export_confirm = Some(i);
                                } else {
                                    self.run_file(i);
                                }
                            }
                        }
                    });
                    if ui.button(tr("Nouvelle note", lang)).clicked() {
                        self.create_note(false);
                    }
                    if ui.button(tr("Nouvelle tâche", lang)).clicked() {
                        self.create_note(true);
                    }
                    if ui.button(tr("Paramètres", lang)).clicked() {
                        self.settings = true;
                    }
                    if self.store.is_encrypted() && ui.button(tr("Synchronisation", lang)).clicked()
                    {
                        self.sync_edit = self.sync_config.clone();
                        self.sync_dialog = true;
                    }
                    if self.store.is_encrypted() && ui.button(tr("Verrouiller", lang)).clicked() {
                        self.lock_requested = true;
                    }
                    if ui.button(tr("À propos", lang)).clicked() {
                        self.about = true;
                    }
                });
            });
        egui::Panel::bottom("status")
            .frame(glass_frame(self.dark).inner_margin(10))
            .show(ui, |ui| {
                if let Some(e) = self.error.clone() {
                    ui.colored_label(
                        egui::Color32::from_rgb(235, 110, 100),
                        tr("Opération impossible. Le brouillon est conservé.", lang),
                    );
                    ui.collapsing(tr("Détails techniques", lang), |ui| {
                        ui.label(e);
                    });
                } else {
                    ui.label(tr(
                        if self.dirty {
                            "Brouillon non enregistré"
                        } else if self.success {
                            "Enregistré"
                        } else {
                            "Version de développement : compatibilité Joplin incomplète."
                        },
                        lang,
                    ));
                }
            });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::Panel::left("notebooks")
            .default_size(210.0)
            .resizable(true)
            .show_separator_line(false)
            .frame(glass_frame(self.dark))
            .show(ui, |ui| {
                glass_reflection(ui, self.dark);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading(tr("Carnets", lang));
                    for (key, trash, todo) in [
                        ("Toutes les notes", false, false),
                        ("Tâches", false, true),
                        ("Corbeille", true, false),
                    ] {
                        if ui
                            .selectable_label(
                                self.notebook.is_none()
                                    && self.tag.is_none()
                                    && self.trash == trash
                                    && self.todo == todo,
                                tr(key, lang),
                            )
                            .clicked()
                        {
                            self.change_filter(None, trash, todo, None);
                        }
                    }
                    ui.separator();
                    for book in self.notebooks.clone() {
                        let mut depth = 0;
                        let mut parent = book.parent_id.clone();
                        while let Some(id) = parent {
                            depth += 1;
                            if depth >= 16 {
                                break;
                            }
                            parent = self
                                .notebooks
                                .iter()
                                .find(|b| b.id == id)
                                .and_then(|b| b.parent_id.clone());
                        }
                        let response = ui.selectable_label(
                            self.notebook.as_deref() == Some(&book.id),
                            format!("{}{}", "  ".repeat(depth), book.title),
                        );
                        if response.clicked() {
                            self.change_filter(Some(book.id.clone()), false, false, None);
                        }
                        response.context_menu(|ui| {
                            if ui.button(tr("Renommer", lang)).clicked() {
                                self.rename = Some(book.id.clone());
                                self.notebook_name = book.title.clone();
                                self.notebook_dialog = true;
                                ui.close();
                            }
                            if ui.button(tr("Déplacer le carnet", lang)).clicked()
                                && self.save_draft()
                            {
                                self.manage_book = Some(book.id.clone());
                                self.book_delete = false;
                                self.book_target = book.parent_id.clone();
                                ui.close();
                            }
                            if ui.button(tr("Supprimer le carnet", lang)).clicked()
                                && self.save_draft()
                            {
                                self.manage_book = Some(book.id.clone());
                                self.book_delete = true;
                                self.book_target = self
                                    .notebooks
                                    .iter()
                                    .find(|b| b.id != book.id)
                                    .map(|b| b.id.clone());
                                ui.close();
                            }
                            if ui.button(tr("Sous-carnet", lang)).clicked() {
                                self.rename = None;
                                self.notebook_parent = Some(book.id.clone());
                                self.notebook_name.clear();
                                self.notebook_dialog = true;
                                ui.close();
                            }
                        });
                    }
                    if ui.button(tr("Nouveau carnet", lang)).clicked() {
                        self.notebook_parent = None;
                        self.rename = None;
                        self.notebook_name.clear();
                        self.notebook_dialog = true;
                    }
                    ui.separator();
                    ui.strong(tr("Étiquettes", lang));
                    match self.store.all_tags() {
                        Ok(tags) => {
                            for tag in tags {
                                let response =
                                    ui.selectable_label(self.tag.as_deref() == Some(&tag), &tag);
                                if response.clicked() {
                                    self.change_filter(None, false, false, Some(tag.clone()));
                                }
                                response.context_menu(|ui| {
                                    if ui.button(tr("Renommer", lang)).clicked()
                                        && self.save_draft()
                                    {
                                        self.manage_tag = Some(tag.clone());
                                        self.tag_delete = false;
                                        self.tag_title = tag.clone();
                                        ui.close();
                                    }
                                    if ui.button(tr("Supprimer l’étiquette", lang)).clicked()
                                        && self.save_draft()
                                    {
                                        self.manage_tag = Some(tag.clone());
                                        self.tag_delete = true;
                                        ui.close();
                                    }
                                });
                            }
                        }
                        Err(e) => self.fail(e),
                    }
                });
            });
        egui::Panel::left("note-list")
            .default_size(270.0)
            .resizable(true)
            .show_separator_line(false)
            .frame(glass_frame(self.dark))
            .show(ui, |ui| {
                glass_reflection(ui, self.dark);
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(tr("Rechercher", lang)),
                    )
                    .changed()
                {
                    self.offset = 0;
                    if let Err(e) = self.refresh() {
                        self.fail(e);
                    }
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.offset > 0, egui::Button::new(tr("Précédent", lang)))
                        .clicked()
                    {
                        self.offset = self.offset.saturating_sub(MAX_RESULTS);
                        if let Err(e) = self.refresh() {
                            self.fail(e);
                        }
                    }
                    if ui
                        .add_enabled(
                            self.notes.len() == MAX_RESULTS,
                            egui::Button::new(tr("Suivant", lang)),
                        )
                        .clicked()
                    {
                        self.offset += MAX_RESULTS;
                        if let Err(e) = self.refresh() {
                            self.fail(e);
                        }
                    }
                });
                if self.trash && ui.button(tr("Vider la corbeille", lang)).clicked() {
                    self.confirm_purge = true;
                }
                ui.separator();
                if self.notes.is_empty() {
                    ui.label(tr("Aucune note", lang));
                }
                egui::ScrollArea::vertical().show_rows(ui, 32.0, self.notes.len(), |ui, range| {
                    for i in range {
                        let n = &self.notes[i];
                        let id = n.id.clone();
                        let prefix = if n.is_todo {
                            if n.completed { "[x] " } else { "[ ] " }
                        } else {
                            ""
                        };
                        if ui
                            .selectable_label(
                                self.active.as_ref().is_some_and(|a| a.id == id),
                                format!("{prefix}{}", n.title),
                            )
                            .clicked()
                        {
                            self.select(&id);
                        }
                    }
                });
            });
    }
    fn editor(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        egui::CentralPanel::default()
            .frame(glass_frame(self.dark))
            .show(ui, |ui| {
                glass_reflection(ui, self.dark);
                if self.active.is_none() {
                    ui.centered_and_justified(|ui| ui.label(tr("Sélectionnez une note", lang)));
                    return;
                }
                let mut save = false;
                let mut attach = false;
                let mut duplicate = false;
                let mut restore = false;
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Enregistrer", lang)).clicked() {
                        save = true;
                    }
                    ui.selectable_value(&mut self.preview, false, tr("Modifier", lang));
                    ui.selectable_value(&mut self.preview, true, tr("Aperçu", lang));
                    if ui.button(tr("Historique", lang)).clicked() {
                        self.history = true;
                    }
                    if ui.button(tr("Joindre un fichier", lang)).clicked() {
                        attach = true;
                    }
                    if ui.button(tr("Dupliquer", lang)).clicked() {
                        duplicate = true;
                    }
                    if self.active.as_ref().is_some_and(|n| n.deleted) {
                        if ui.button(tr("Restaurer", lang)).clicked() {
                            restore = true;
                        }
                    } else if ui.button(tr("Mettre à la corbeille", lang)).clicked() {
                        self.confirm_trash = true;
                    }
                    if ui.button(tr("Copier le lien de la note", lang)).clicked()
                        && let Some(n) = &self.active
                    {
                        ui.ctx().copy_text(format!(":/{}", n.id));
                    }
                });
                let mut edited = false;
                let mut navigate = None;
                if let Some(n) = &mut self.active {
                    let title_label = ui.label(tr("Titre", lang));
                    edited |= ui
                        .add(
                            egui::TextEdit::singleline(&mut n.title)
                                .font(egui::TextStyle::Heading)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(title_label.id)
                        .changed();
                    ui.horizontal(|ui| {
                        let title = self
                            .notebooks
                            .iter()
                            .find(|b| b.id == n.notebook_id)
                            .map_or("", |b| b.title.as_str());
                        let notebook_label = ui.label(tr("Carnets", lang));
                        egui::ComboBox::from_id_salt("destination")
                            .selected_text(title)
                            .show_ui(ui, |ui| {
                                for b in &self.notebooks {
                                    edited |= ui
                                        .selectable_value(
                                            &mut n.notebook_id,
                                            b.id.clone(),
                                            &b.title,
                                        )
                                        .changed();
                                }
                            })
                            .response
                            .labelled_by(notebook_label.id);
                        edited |= ui.checkbox(&mut n.is_todo, tr("Tâches", lang)).changed();
                        if n.is_todo {
                            edited |= ui.checkbox(&mut n.completed, tr("Terminé", lang)).changed();
                        }
                    });
                    if n.is_todo {
                        ui.horizontal(|ui| {
                            let due_label = ui.label(tr("Échéance (AAAA-MM-JJ)", lang));
                            edited |= ui
                                .text_edit_singleline(&mut self.due)
                                .labelled_by(due_label.id)
                                .changed();
                        });
                    }
                    edited |= ui
                        .add(
                            egui::TextEdit::singleline(&mut self.tags)
                                .hint_text(tr("Étiquettes (séparées par des virgules)", lang))
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                    ui.separator();
                    if self.preview {
                        ui.small(tr(
                            "Les liens externes et les images distantes sont désactivés.",
                            lang,
                        ));
                        // Intercept EVERY Markdown link. No arbitrary shell/URL handler is invoked.
                        let rendered = self.media.prepare(&self.store, ui.ctx(), &n.body);
                        self.cache.link_hooks_clear();
                        for e in pulldown_cmark::Parser::new_ext(
                            rendered,
                            pulldown_cmark::Options::all(),
                        ) {
                            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link {
                                dest_url,
                                ..
                            }) = e
                            {
                                self.cache.add_link_hook(dest_url.to_string());
                            }
                        }
                        egui::ScrollArea::vertical()
                            .id_salt("preview")
                            .show(ui, |ui| {
                                CommonMarkViewer::new()
                                    .explicit_image_uri_scheme(true)
                                    .max_image_width(Some(680))
                                    .show(ui, &mut self.cache, rendered);
                            });
                        for (link, clicked) in self.cache.link_hooks() {
                            if *clicked
                                && let Some(id) = link.strip_prefix(":/")
                                && carnet_rust::store::valid_id(id)
                            {
                                navigate = Some(id.to_string());
                            }
                        }
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for (key, insert) in [
                                ("Gras", "****"),
                                ("Italique", "**"),
                                ("Titre", "\n## "),
                                ("Liste de tâches", "\n- [ ] "),
                            ] {
                                if ui.button(tr(key, lang)).clicked() {
                                    n.body.push_str(insert);
                                    edited = true;
                                }
                            }
                        });
                        egui::ScrollArea::vertical()
                            .id_salt("source")
                            .show(ui, |ui| {
                                edited |= ui
                                    .add(
                                        egui::TextEdit::multiline(&mut n.body)
                                            .id(egui::Id::unique("note-body"))
                                            .char_limit(4 * 1024 * 1024)
                                            .hint_text(tr("Modifier", lang))
                                            .code_editor()
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(28),
                                    )
                                    .changed();
                            });
                    }
                }
                if edited {
                    self.changed();
                }
                if save {
                    self.save_draft();
                }
                if restore && let Some(n) = &mut self.active {
                    n.deleted = false;
                    self.changed();
                    self.save_draft();
                }
                if attach {
                    self.run_file(5);
                }
                if duplicate
                    && self.save_draft()
                    && let Some(n) = self.active.clone()
                {
                    match self.store.create_note(&n.notebook_id, &n.title, n.is_todo) {
                        Ok(mut copy) => {
                            copy.body = n.body;
                            copy.due = n.due;
                            copy.completed = n.completed;
                            match self.store.save(&mut copy, &self.tags) {
                                Ok(()) => self.select(&copy.id),
                                Err(e) => self.fail(e),
                            }
                        }
                        Err(e) => self.fail(e),
                    }
                    if let Err(e) = self.refresh() {
                        self.fail(e);
                    }
                }
                if let Some(id) = navigate {
                    self.select(&id);
                }
            });
    }
    fn windows(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        if self.settings {
            let mut open = true;
            egui::Window::new(tr("Paramètres", lang))
                .open(&mut open)
                .show(ctx, |ui| {
                    let language_label = ui.label(tr("Langue", lang));
                    let old = self.lang;
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(i18n::NAMES[self.lang])
                        .show_ui(ui, |ui| {
                            for (i, name) in i18n::NAMES.iter().enumerate() {
                                ui.selectable_value(&mut self.lang, i, *name);
                            }
                        })
                        .response
                        .labelled_by(language_label.id);
                    if self.lang != old
                        && let Err(e) = self.store.set_setting("language", i18n::CODES[self.lang])
                    {
                        self.fail(e);
                    }
                    if ui
                        .checkbox(&mut self.dark, tr("Thème sombre", self.lang))
                        .changed()
                    {
                        set_glass_style(ctx, self.dark);
                        if let Err(e) = self
                            .store
                            .set_setting("dark", if self.dark { "true" } else { "false" })
                        {
                            self.fail(e);
                        }
                    }
                    if self.store.is_encrypted() {
                        ui.separator();
                        if ui
                            .button(tr("Changer la phrase de passe", self.lang))
                            .clicked()
                        {
                            self.password_dialog = true;
                        }
                        if ui
                            .button(tr("Restaurer une sauvegarde chiffrée", self.lang))
                            .clicked()
                            && self.save_draft()
                        {
                            self.restore_source = rfd::FileDialog::new()
                                .add_filter("SQLCipher", &["sqlite3"])
                                .pick_file();
                        }
                        if ui
                            .button(tr("Nettoyer les pièces jointes inutilisées", self.lang))
                            .clicked()
                        {
                            self.confirm_gc = true;
                        }
                        ui.small(tr("0 : pas de verrouillage automatique.", self.lang));
                        let label = ui.label(tr("Verrouillage automatique (minutes)", self.lang));
                        if ui
                            .add(egui::Slider::new(&mut self.auto_lock_minutes, 0..=60))
                            .labelled_by(label.id)
                            .changed()
                            && let Err(e) = self.store.set_setting(
                                "auto_lock_minutes",
                                &self.auto_lock_minutes.to_string(),
                            )
                        {
                            self.fail(e);
                        }
                    }
                    ui.label(tr("Stockage local chiffré.", self.lang));
                });
            self.settings = open;
        }
        if self.about {
            egui::Window::new(tr("À propos", lang))
                .open(&mut self.about)
                .show(ctx, |ui| {
                    ui.heading(format!("Carnet de notes {}", env!("CARGO_PKG_VERSION")));
                    ui.label("© 2026 jeremdevgit · GPL-3.0-or-later");
                    ui.label(tr(
                        "Version de développement : compatibilité Joplin incomplète.",
                        lang,
                    ));
                    ui.collapsing(tr("Mes logiciels", lang), |ui| {
                        ui.hyperlink_to("jeremdevgit", "https://github.com/jeremdevgit");
                        for name in [
                            "Suite libre",
                            "Batailles de France",
                            "Pomme",
                            "Ludothèque",
                            "rJb",
                            "Boîte à outils",
                        ] {
                            ui.label(name);
                        }
                    });
                });
        }
        if self.notebook_dialog {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr(
                if self.rename.is_some() {
                    "Renommer"
                } else {
                    "Nouveau carnet"
                },
                lang,
            ))
            .open(&mut open)
            .show(ctx, |ui| {
                let name_label = ui.label(tr("Nom", lang));
                ui.text_edit_singleline(&mut self.notebook_name)
                    .labelled_by(name_label.id);
                if ui
                    .button(tr(
                        if self.rename.is_some() {
                            "Renommer"
                        } else {
                            "Créer"
                        },
                        lang,
                    ))
                    .clicked()
                {
                    let result = if let Some(id) = &self.rename {
                        self.store.rename_notebook(id, &self.notebook_name)
                    } else {
                        self.store
                            .create_notebook(&self.notebook_name, self.notebook_parent.as_deref())
                            .map(|_| ())
                    };
                    match result {
                        Ok(()) => {
                            done = true;
                            if let Err(e) = self.refresh() {
                                self.fail(e);
                            }
                        }
                        Err(e) => self.fail(e),
                    }
                }
            });
            self.notebook_dialog = open && !done;
        }
        if self.confirm_trash {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr("Corbeille", lang))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(tr("Déplacer cette note dans la corbeille ?", lang));
                    if ui.button(tr("Confirmer", lang)).clicked()
                        && self.save_draft()
                        && let Some(n) = &mut self.active
                    {
                        n.deleted = true;
                        self.changed();
                        done = self.save_draft();
                    }
                    if ui.button(tr("Annuler", lang)).clicked() {
                        done = true;
                    }
                });
            self.confirm_trash = open && !done;
        }
        if self.history {
            let mut open = true;
            let mut selected = None;
            egui::Window::new(tr("Historique", lang))
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(n) = &self.active {
                        match self.store.revision_headers(&n.id) {
                            Ok(revs) => {
                                if revs.is_empty() {
                                    ui.label(tr("Aucun historique", lang));
                                }
                                egui::ScrollArea::vertical()
                                    .max_height(350.0)
                                    .show(ui, |ui| {
                                        for r in revs {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    chrono::DateTime::from_timestamp_millis(r.at)
                                                        .map(|d| {
                                                            d.format("%Y-%m-%d %H:%M").to_string()
                                                        })
                                                        .unwrap_or_default(),
                                                );
                                                ui.label(&r.title);
                                                if ui.button(tr("Restaurer", lang)).clicked() {
                                                    selected = Some(r);
                                                }
                                            });
                                        }
                                    });
                            }
                            Err(e) => self.fail(e),
                        }
                    }
                });
            if let Some(r) = selected
                && self.save_draft()
                && let Some(n) = &mut self.active
            {
                match self.store.revision(&n.id, r.seq) {
                    Ok(full) => {
                        n.title = full.title;
                        n.body = full.body;
                        self.changed();
                        self.save_draft();
                    }
                    Err(e) => self.fail(e),
                }
            }
            self.history = open;
        }
        if self.preview
            && let Some(n) = &self.active
        {
            match self.store.attachment_headers(&n.body) {
                Ok(attachments) => {
                    if !attachments.is_empty() {
                        egui::Window::new(tr("Pièces jointes", lang)).show(ctx, |ui| {
                            ui.small(tr("Ce fichier pourra être lu sans phrase de passe. Conservez-le dans un emplacement sûr.",lang));
                            for a in attachments {
                                ui.horizontal(|ui| {
                                    ui.label(&a.title);
                                    if ui.button(tr("Exporter le fichier", lang)).clicked() {
                                        let suffix = if a.extension.is_empty() {
                                            String::new()
                                        } else {
                                            format!(".{}", a.extension)
                                        };
                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_title(tr("Export non chiffré",lang)).set_file_name(format!("{}{suffix}", a.id))
                                            .save_file()
                                            && let Err(e) = self
                                                .store
                                                .attachment_data(&a.id)
                                                .and_then(|data| exchange::write_new(&path, &data))
                                        {
                                            self.fail(e);
                                        }
                                    }
                                });
                            }
                        });
                    }
                }
                Err(e) => self.fail(e),
            }
        }
    }
}
impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
impl App {
    fn draw(&mut self, ui: &mut egui::Ui) {
        if self.sync_config.automatic
            && !self.sync_config.server.is_empty()
            && !self.sync_config.email.is_empty()
            && !self.conflicts_dialog
            && !self.password_dialog
            && !self.settings
            && !self.sync_dialog
            && self.backup_destination.is_none()
            && self.restore_source.is_none()
        {
            if Instant::now() >= self.next_sync && self.sync_task.is_none() {
                self.start_sync();
            }
            ui.ctx()
                .request_repaint_after(self.next_sync.saturating_duration_since(Instant::now()));
        }
        if self.sync_busy(ui) {
            return;
        }
        paint_glass_background(ui, self.dark);
        let ctx = ui.ctx().clone();
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_draft();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.create_note(false);
        }
        if self.dirty && self.retry_save {
            if self.changed.elapsed() >= Duration::from_millis(800) {
                self.save_draft();
            } else {
                ctx.request_repaint_after(
                    Duration::from_millis(800).saturating_sub(self.changed.elapsed()),
                );
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.save_draft() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.top(ui);
        self.sidebar(ui);
        self.editor(ui);
        self.windows(&ctx);
        self.security_windows(&ctx);
        self.management_windows(&ctx);
        self.sync_windows(&ctx);
        // Defense in depth: only our fixed About URL may leave this native app.
        ctx.output_mut(|o|o.commands.retain(|c|!matches!(c,egui::OutputCommand::OpenUrl(u) if u.url!="https://github.com/jeremdevgit")));
    }
}
impl App {
    fn prepare_lock(&mut self) -> bool {
        if self.sync_task.is_some() {
            return true;
        }
        if !self.save_draft()
            && let Some(note) = &self.active
            && let Err(e) = self.store.save_recovery_draft(note, &self.tags, &self.due)
        {
            self.fail(e);
            return false;
        }
        true
    }
    fn security_windows(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        if self.password_dialog {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr("Changer la phrase de passe", lang))
                .open(&mut open)
                .show(ctx, |ui| {
                    let device = self
                        .store
                        .profile_dir()
                        .is_some_and(carnet_rust::device_key::is_device_managed);
                    if !device {
                        let label = ui.label(tr("Phrase de passe actuelle", lang));
                        ui.add(
                            egui::TextEdit::singleline(&mut *self.current_password)
                                .password(true)
                                .char_limit(1024),
                        )
                        .labelled_by(label.id);
                        if ui
                            .button(tr(
                                "Utiliser la protection système sans mot de passe maître",
                                lang,
                            ))
                            .clicked()
                        {
                            match carnet_rust::device_key::enable(
                                &mut self.store,
                                self.current_password.as_str(),
                            ) {
                                Ok(()) => {
                                    done = true;
                                    self.auto_lock_minutes = 0;
                                    self.success = true;
                                    self.error = None;
                                }
                                Err(e) => self.fail(e),
                            }
                        }
                    }
                    let label = ui.label(tr("Nouvelle phrase de passe", lang));
                    ui.add(
                        egui::TextEdit::singleline(&mut *self.new_password)
                            .password(true)
                            .char_limit(1024),
                    )
                    .labelled_by(label.id);
                    let label = ui.label(tr("Confirmer la phrase de passe", lang));
                    ui.add(
                        egui::TextEdit::singleline(&mut *self.confirm_password)
                            .password(true)
                            .char_limit(1024),
                    )
                    .labelled_by(label.id);
                    ui.label(tr(
                        "Les anciennes sauvegardes conservent leur phrase de passe.",
                        lang,
                    ));
                    if ui.button(tr("Enregistrer", lang)).clicked() {
                        if self.new_password.as_str() != self.confirm_password.as_str() {
                            self.fail(
                                tr("Les phrases de passe ne correspondent pas", lang).to_string(),
                            );
                        } else {
                            match carnet_rust::device_key::set_master(
                                &mut self.store,
                                self.current_password.as_str(),
                                self.new_password.as_str(),
                            ) {
                                Ok(()) => {
                                    done = true;
                                    self.auto_lock_minutes = 5;
                                    self.success = true;
                                    self.error = None;
                                }
                                Err(e) => self.fail(e),
                            }
                        }
                    }
                });
            self.password_dialog = open && !done;
            if !self.password_dialog {
                self.current_password.zeroize();
                self.new_password.zeroize();
                self.confirm_password.zeroize();
            }
        }
        if let Some(path) = self.backup_destination.clone() {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr("Sauvegarde portable chiffrée",lang)).open(&mut open).show(ctx,|ui|{
                ui.label(tr("Choisissez un mot de passe pour cette sauvegarde. Il sera nécessaire sur un autre appareil.",lang));
                let label=ui.label(tr("Phrase de passe",lang));
                ui.add(egui::TextEdit::singleline(&mut *self.backup_password).password(true).char_limit(1024)).labelled_by(label.id);
                let label=ui.label(tr("Confirmer la phrase de passe",lang));
                ui.add(egui::TextEdit::singleline(&mut *self.backup_confirm).password(true).char_limit(1024)).labelled_by(label.id);
                if ui.button(tr("Enregistrer",lang)).clicked() && self.save_draft() {
                    if self.backup_password.as_str()!=self.backup_confirm.as_str() {self.fail(tr("Les phrases de passe ne correspondent pas",lang));}
                    else {match self.store.backup_with_password(&path,self.backup_password.as_str()) {
                        Ok(())=>{done=true;self.success=true;self.error=None;},Err(e)=>self.fail(e),
                    }}
                }
            });
            if !open || done {
                self.backup_destination = None;
                self.backup_password.zeroize();
                self.backup_confirm.zeroize();
            }
        }
        if let Some(path) = self.restore_source.clone() {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr("Restaurer une sauvegarde chiffrée",lang)).open(&mut open).show(ctx,|ui|{
                ui.label(tr("Le contenu actuel sera remplacé. Une copie de récupération chiffrée sera conservée.",lang));
                let label=ui.label(tr("Phrase de passe de la sauvegarde",lang));
                ui.add(egui::TextEdit::singleline(&mut *self.restore_password).password(true).char_limit(1024)).labelled_by(label.id);
                if ui.button(tr("Restaurer",lang)).clicked() && self.save_draft(){
                    match self.store.restore_backup(&path,self.restore_password.as_str()) {
                        Ok(())=>{self.active=None;self.dirty=false;self.tags.zeroize();self.query.zeroize();self.notebook=None;self.tag=None;self.offset=0;self.trash=false;self.todo=false;self.cache=CommonMarkCache::default();self.media.key.clear();self.sync_config=carnet_rust::sync::load_config(&self.store).unwrap_or_default();done=true;self.success=true;self.error=None;
                            if let Err(e)=self.refresh(){self.fail(e);}
                        },Err(e)=>self.fail(e)
                    }
                }
            });
            if !open || done {
                self.restore_source = None;
                self.restore_password.zeroize();
            }
        }
        if let Some(action) = self.export_confirm {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr("Export non chiffré",lang)).open(&mut open).show(ctx,|ui|{
                ui.label(tr("Ce fichier pourra être lu sans phrase de passe. Conservez-le dans un emplacement sûr.",lang));
                if ui.button(tr("Exporter",lang)).clicked(){done=true;self.run_file(action);}
                if ui.button(tr("Annuler",lang)).clicked(){done=true;}
            });
            if !open || done {
                self.export_confirm = None;
            }
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(note) = &mut self.active {
            note.title.zeroize();
            note.body.zeroize();
            note.extra.zeroize();
        }
        for note in &mut self.notes {
            note.title.zeroize();
        }
        for book in &mut self.notebooks {
            book.title.zeroize();
        }
        self.tags.zeroize();
        self.query.zeroize();
        self.due.zeroize();
        self.notebook_name.zeroize();
        self.tag_title.zeroize();
        self.tag.zeroize();
        self.manage_tag.zeroize();
    }
}

fn glass_reflection(ui: &egui::Ui, dark: bool) {
    let rect = ui.max_rect().expand(10.0);
    let top = rect.top() + 1.0;
    let left = rect.left() + 12.0;
    let right = rect.right() - 12.0;
    let bottom = (top + 65.0).min(rect.bottom());
    let mut mesh = egui::Mesh::default();
    let bright = egui::Color32::from_white_alpha(if dark { 12 } else { 65 });
    mesh.colored_vertex(egui::pos2(left, top), bright);
    mesh.colored_vertex(egui::pos2(right, top), bright);
    mesh.colored_vertex(egui::pos2(left, bottom), egui::Color32::TRANSPARENT);
    mesh.colored_vertex(egui::pos2(right, bottom), egui::Color32::TRANSPARENT);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().line_segment(
        [egui::pos2(left + 8.0, top), egui::pos2(right - 8.0, top)],
        egui::Stroke::new(
            0.8,
            egui::Color32::from_white_alpha(if dark { 36 } else { 180 }),
        ),
    );
}
fn glass_frame(dark: bool) -> egui::Frame {
    egui::Frame::new()
        .inner_margin(16)
        .outer_margin(7)
        .corner_radius(22)
        .fill(if dark {
            egui::Color32::from_rgba_unmultiplied(24, 39, 62, 186)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 178)
        })
        .stroke(egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_white_alpha(35)
            } else {
                egui::Color32::from_white_alpha(215)
            },
        ))
        .shadow(egui::epaint::Shadow {
            offset: [0, 6],
            blur: 22,
            spread: 0,
            color: egui::Color32::from_black_alpha(if dark { 45 } else { 18 }),
        })
}
fn set_glass_style(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = egui::Color32::TRANSPARENT;
    visuals.window_fill = if dark {
        egui::Color32::from_rgb(26, 42, 63)
    } else {
        egui::Color32::from_rgb(241, 249, 255)
    };
    visuals.window_corner_radius = egui::CornerRadius::same(22);
    visuals.menu_corner_radius = egui::CornerRadius::same(14);
    visuals.window_stroke = egui::Stroke::new(
        1.0,
        egui::Color32::from_white_alpha(if dark { 40 } else { 220 }),
    );
    visuals.override_text_color = Some(if dark {
        egui::Color32::from_rgb(229, 239, 252)
    } else {
        egui::Color32::from_rgb(29, 49, 75)
    });
    visuals.selection.bg_fill = if dark {
        egui::Color32::from_rgba_unmultiplied(83, 150, 245, 100)
    } else {
        egui::Color32::from_rgba_unmultiplied(88, 157, 243, 70)
    };
    visuals.selection.stroke = egui::Stroke::new(
        1.0,
        if dark {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(20, 65, 110)
        },
    );
    visuals.text_edit_bg_color = Some(if dark {
        egui::Color32::from_rgba_unmultiplied(10, 23, 42, 100)
    } else {
        egui::Color32::from_white_alpha(140)
    });
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(11);
        widget.bg_stroke = egui::Stroke::new(
            1.0,
            egui::Color32::from_white_alpha(if dark { 30 } else { 210 }),
        );
    }
    visuals.widgets.inactive.weak_bg_fill = if dark {
        egui::Color32::from_white_alpha(10)
    } else {
        egui::Color32::from_white_alpha(85)
    };
    visuals.widgets.hovered.weak_bg_fill = if dark {
        egui::Color32::from_rgba_unmultiplied(95, 164, 255, 65)
    } else {
        egui::Color32::from_rgba_unmultiplied(120, 180, 255, 75)
    };
    ctx.set_visuals(visuals);
    ctx.style_mut_of(
        if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        },
        |s| {
            s.spacing.item_spacing = egui::vec2(9.0, 10.0);
            s.spacing.button_padding = egui::vec2(12.0, 8.0);
            s.spacing.interact_size = egui::vec2(36.0, 32.0);
            s.text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
            s.text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.0));
        },
    );
}
fn paint_glass_background(ui: &egui::Ui, dark: bool) {
    let rect = ui.max_rect();
    let mut mesh = egui::Mesh::default();
    let colors = if dark {
        [(12, 23, 44), (24, 38, 71), (22, 61, 73), (37, 34, 76)]
    } else {
        [
            (203, 226, 250),
            (224, 222, 249),
            (190, 234, 230),
            (228, 233, 252),
        ]
    };
    for (pos, (r, g, b)) in [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .into_iter()
    .zip(colors)
    {
        mesh.colored_vertex(pos, egui::Color32::from_rgba_unmultiplied(r, g, b, 220));
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
    // Static radial highlights: no timer or continuous animation while idle.
    for (x, y, radius, color) in [
        (
            0.18,
            0.14,
            340.0,
            egui::Color32::from_rgba_unmultiplied(126, 197, 255, if dark { 34 } else { 75 }),
        ),
        (
            0.86,
            0.85,
            390.0,
            egui::Color32::from_rgba_unmultiplied(162, 139, 246, if dark { 30 } else { 58 }),
        ),
    ] {
        let center = egui::pos2(
            rect.left() + rect.width() * x,
            rect.top() + rect.height() * y,
        );
        let mut glow = egui::Mesh::default();
        glow.colored_vertex(center, color);
        for i in 0..=64 {
            let a = i as f32 * std::f32::consts::TAU / 64.0;
            glow.colored_vertex(
                center + egui::vec2(a.cos(), a.sin()) * radius,
                egui::Color32::TRANSPARENT,
            );
        }
        for i in 1..=64 {
            glow.add_triangle(0, i, i + 1);
        }
        ui.painter().add(egui::Shape::mesh(glow));
    }
}
fn configure_fonts(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto-cjk".into(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansCJKsc-Regular.otf")).into(),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("noto-cjk".into());
    }
    ctx.set_fonts(fonts);
}

fn brand_logo(ui: &mut egui::Ui, size: f32) {
    ui.add(
        egui::Image::new(egui::include_image!("../assets/carnet-de-notes-256.png"))
            .fit_to_exact_size(egui::vec2(size, size)),
    );
}

fn application_icon() -> egui::IconData {
    let rgba = image::load_from_memory(include_bytes!("../assets/carnet-de-notes-256.png"))
        .expect("bundled application icon must be a valid PNG")
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}

fn profile_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("org", "jeremdevgit", "CarnetRust")
        .map(|d| d.data_local_dir().to_path_buf())
        .ok_or(Error::Invalid("user data directory unavailable"))
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let profile = profile_path()?;
    eframe::run_native(
        "Carnet de notes",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(application_icon())
                .with_transparent(true)
                .with_inner_size([1340.0, 860.0])
                .with_min_inner_size([850.0, 540.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(desktop::Desktop::new(profile, cc)))),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[test]
    fn bundled_logo_decodes_for_the_native_window() {
        let icon = application_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }
    fn demo(lang: usize, dark: bool) -> App {
        demo_store(Store::memory().unwrap(), lang, dark)
    }
    fn demo_store(mut store: Store, lang: usize, dark: bool) -> App {
        store.set_setting("language", i18n::CODES[lang]).unwrap();
        store
            .set_setting("dark", if dark { "true" } else { "false" })
            .unwrap();
        let book = store.create_notebook("Studio", None).unwrap();
        let projects = store.create_notebook("Projects", Some(&book)).unwrap();
        for title in ["Reading list", "Ideas for the weekend", "Meeting notes"] {
            let mut n = store.create_note(&book, title, false).unwrap();
            n.body = "A demonstration note.\n\n- An idea\n- Another idea".into();
            store.save(&mut n, "ideas").unwrap();
        }
        let mut todo = store
            .create_note(&projects, "Prepare the next release", true)
            .unwrap();
        todo.body = "- [x] Design\n- [ ] Review\n- [ ] Release".into();
        store.save(&mut todo, "work").unwrap();
        let mut n = store
            .create_note(&book, "A little room for your ideas", false)
            .unwrap();
        n.body="# A little room for your ideas\n\nCapture a thought. Give it space. Come back when you are ready.\n\n## On the desk today\n\n- A notebook that stays with you, offline.\n- A calm place for notes and to-dos.\n- Your words, in your own language.\n\n> Good ideas often begin with a simple note.\n\n### A small plan\n\n| Step | Status |\n| --- | --- |\n| Write it down | Done |\n| Make it clear | In progress |\n| Share a copy | Next |\n\n**Everything starts here.**".into();
        store.save(&mut n, "ideas, personal").unwrap();
        let ctx = egui::Context::default();
        let mut app = App::from_store(store, &ctx).unwrap();
        app.select(&n.id);
        app.preview = true;
        app
    }
    fn harness(app: App) -> Harness<'static, App> {
        Harness::builder()
            .with_size(egui::vec2(1340.0, 860.0))
            .with_max_steps(20)
            .build_ui_state(
                |ui, app| {
                    // Harness owns its Context; install the same native style/fonts once.
                    let setup = ui.ctx().data_mut(|d| {
                        *d.get_temp_mut_or_default::<bool>(egui::Id::unique("carnet-test-setup"))
                    });
                    if !setup {
                        configure_fonts(ui.ctx());
                        set_glass_style(ui.ctx(), app.dark);
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::unique("carnet-test-setup"), true)
                        });
                    }
                    app.draw(ui);
                },
                app,
            )
    }
    #[test]
    fn editor_accepts_paste_copy_cut_and_undo_events() {
        let mut app = demo(0, true);
        app.preview = false;
        let mut h = harness(app);
        h.run();
        h.ctx
            .memory_mut(|m| m.request_focus(egui::Id::unique("note-body")));
        h.event(egui::Event::Paste("\nTexte collé 中文 日本語\n".into()));
        h.run();
        assert!(
            h.state()
                .active
                .as_ref()
                .unwrap()
                .body
                .contains("Texte collé 中文 日本語")
        );
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.event(egui::Event::Copy);
        h.step();
        assert!(h.output().platform_output.commands.iter().any(|c|matches!(c,egui::OutputCommand::CopyText(text) if text.contains("Texte collé 中文 日本語"))));
        h.event(egui::Event::Cut);
        h.run();
        assert!(h.state().active.as_ref().unwrap().body.is_empty());
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run();
        assert!(
            h.state()
                .active
                .as_ref()
                .unwrap()
                .body
                .contains("Texte collé 中文 日本語")
        );
    }
    #[test]
    fn new_note_button_creates_a_persisted_note() {
        let mut h = harness(demo(0, true));
        h.run();
        let before = h
            .state()
            .store
            .list(None, "", false, false, None, 0)
            .unwrap()
            .len();
        h.get_by_label("Nouvelle note").click();
        h.run();
        assert_eq!(
            h.state()
                .store
                .list(None, "", false, false, None, 0)
                .unwrap()
                .len(),
            before + 1
        );
        assert_eq!(h.state().active.as_ref().unwrap().title, "Nouvelle note");
    }
    #[test]
    fn preview_renders_and_blocks_external_url_handlers() {
        let mut app = demo(1, false);
        app.active.as_mut().unwrap().body="# Example\n\n[Untrusted](file:///C:/windows/system32/cmd.exe)\n\n![Remote](https://example.invalid/image.png)".into();
        let mut h = harness(app);
        h.run();
        h.get_by_label("Untrusted").click();
        h.run();
        assert!(
            h.output()
                .platform_output
                .commands
                .iter()
                .all(|c| !matches!(c, egui::OutputCommand::OpenUrl(_)))
        );
    }
    #[test]
    fn failed_save_retains_draft_and_prevents_switch() {
        let mut app = demo(0, true);
        let old = app.active.as_ref().unwrap().id.clone();
        let target = app.notes.iter().find(|n| n.id != old).unwrap().id.clone();
        app.active.as_mut().unwrap().title.clear();
        app.changed();
        app.select(&target);
        assert_eq!(app.active.as_ref().unwrap().id, old);
        assert!(app.dirty);
        assert!(app.error.is_some());
    }
    #[test]
    fn eight_languages_render_without_missing_glyphs() {
        for lang in 0..8 {
            let mut h = harness(demo(lang, true));
            h.run();
            assert!(!h.output().shapes.is_empty());
        }
    }
    #[test]
    #[ignore = "offscreen GPU render; set CARNET_RENDER_DIR to an output directory"]
    fn render_liquid_glass_demo() {
        let dir = PathBuf::from(std::env::var_os("CARNET_RENDER_DIR").expect("CARNET_RENDER_DIR"));
        std::fs::create_dir_all(&dir).unwrap();
        for lang in [0, 1] {
            let path = if lang == 1 {
                dir.join("en")
            } else {
                dir.clone()
            };
            std::fs::create_dir_all(&path).unwrap();
            for (dark, name) in [(true, "liquid-glass-dark"), (false, "liquid-glass-light")] {
                let profile = tempfile::tempdir().unwrap();
                let app = demo_store(
                    Store::open(profile.path(), "Demonstration vault phrase").unwrap(),
                    lang,
                    dark,
                );
                let mut h = harness(app);
                h.run();
                h.render()
                    .expect("offscreen renderer")
                    .save(path.join(format!("{name}.png")))
                    .unwrap();
            }
        }
    }
}
