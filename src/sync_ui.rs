use super::{App, egui, glass_frame, paint_glass_background, tr};
use carnet_rust::{
    Error, Result,
    store::Store,
    sync::{self, Outcome, Resolution},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;
pub(super) struct Task {
    receiver: mpsc::Receiver<(Store, Result<Outcome>)>,
    cancel: Arc<AtomicBool>,
}
impl Drop for Task {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
impl App {
    pub(super) fn sync_busy(&mut self, ui: &mut egui::Ui) -> bool {
        let result = self
            .sync_task
            .as_ref()
            .and_then(|task| match task.receiver.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(Error::Invalid("sync worker stopped")))
                }
            });
        if let Some(result) = result {
            self.sync_task = None;
            match result {
                Ok((store, result)) => {
                    self.store = store;
                    match result {
                        Ok(Outcome::Complete {
                            uploaded,
                            downloaded,
                            deleted,
                        }) => {
                            self.sync_status = Some(format!(
                                "{}: {uploaded} / {downloaded} / {deleted}",
                                tr("Envoi / réception / suppression", self.lang)
                            ));
                            self.reload_after_management();
                        }
                        Ok(Outcome::Conflicts(_)) => {
                            self.conflicts_dialog = true;
                            self.sync_status = Some(tr("Conflits à résoudre", self.lang).into());
                            if let Err(e) = self.refresh() {
                                self.fail(e);
                            }
                        }
                        Err(e) => self.fail(e),
                    }
                }
                Err(e) => {
                    self.fail(e);
                    self.lock_requested = true;
                }
            }
        }
        if self.sync_task.is_none() {
            return false;
        }
        let ctx = ui.ctx().clone();
        ctx.request_repaint_after(Duration::from_millis(100));
        paint_glass_background(ui, self.dark);
        egui::CentralPanel::default()
            .frame(glass_frame(self.dark))
            .show(ui, |ui| {
                ui.heading(tr("Synchronisation en cours…", self.lang));
                ui.spinner();
                if ui.button(tr("Annuler", self.lang)).clicked()
                    && let Some(task) = &self.sync_task
                {
                    task.cancel.store(true, Ordering::Relaxed);
                }
                if ui.button(tr("Verrouiller", self.lang)).clicked() {
                    self.lock_requested = true;
                }
                if ctx.input(|i| i.viewport().close_requested())
                    && let Some(task) = &self.sync_task
                {
                    task.cancel.store(true, Ordering::Relaxed);
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
            });
        true
    }
    pub(super) fn start_sync(&mut self) {
        self.next_sync = std::time::Instant::now() + Duration::from_secs(300);
        if self.sync_task.is_some() || !self.save_draft() {
            return;
        }
        if let Err(e) = sync::save_config(&self.store, &self.sync_config) {
            self.fail(e);
            return;
        }
        let replacement = match Store::memory() {
            Ok(s) => s,
            Err(e) => {
                self.fail(e);
                return;
            }
        };
        let mut store = std::mem::replace(&mut self.store, replacement);
        let config = self.sync_config.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (send, receiver) = mpsc::channel();
        self.sync_task = Some(Task { receiver, cancel });
        self.sync_dialog = false;
        std::thread::spawn(move || {
            let result = sync::run(&mut store, &config, worker_cancel);
            let _ = send.send((store, result));
        });
    }
    pub(super) fn sync_windows(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        if self.sync_dialog {
            let mut open = true;
            egui::Window::new(tr("Synchronisation",lang)).open(&mut open).default_width(480.0).show(ctx,|ui|{
                account_form(ui, &mut self.sync_edit, lang);
                ui.label(tr("Les identifiants sont conservés dans le coffre chiffré.",lang));
                ui.label(tr("Compatibilité expérimentale. Les cibles Joplin E2EE et les carnets partagés sont refusés.",lang));
                ui.checkbox(&mut self.sync_edit.automatic,tr("Synchroniser automatiquement",lang));
                ui.small(tr("Nouvel essai toutes les 5 minutes tant que le coffre est ouvert.",lang));
                if ui.button(tr("Se connecter et synchroniser",lang)).clicked(){
                    if let Err(e)=sync::save_config(&self.store,&self.sync_edit){self.fail(e);}
                    else {self.sync_config=self.sync_edit.clone();self.start_sync();}
                }
                if ui.button(tr("Enregistrer",lang)).clicked(){
                    match sync::save_config(&self.store,&self.sync_edit) {
                        Ok(())=>{self.sync_config=self.sync_edit.clone();self.next_sync=std::time::Instant::now()+Duration::from_secs(300);self.success=true;self.error=None;},
                        Err(e)=>self.fail(e),
                    }
                }
                if ui.button(tr("Conflits à résoudre",lang)).clicked(){self.conflicts_dialog=true;}
                if let Some(status)=&self.sync_status{ui.label(status);}
            });
            self.sync_dialog = open && self.sync_task.is_none();
            if !self.sync_dialog {
                self.sync_edit = sync::Config::default();
            }
        }
        if self.conflicts_dialog {
            let mut open = true;
            let mut choice = None;
            egui::Window::new(tr("Conflits à résoudre", lang))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(tr(
                        "Choisissez la version à conserver, puis relancez la synchronisation.",
                        lang,
                    ));
                    match sync::conflicts(&self.store) {
                        Ok(conflicts) => {
                            egui::ScrollArea::vertical()
                                .max_height(450.0)
                                .show(ui, |ui| {
                                    for c in conflicts {
                                        ui.group(|ui| {
                                            ui.label(&c.path);
                                            ui.label(format!(
                                                "{} / {}",
                                                c.local_title, c.remote_title
                                            ));
                                            if ui
                                                .button(tr("Conserver la version locale", lang))
                                                .clicked()
                                            {
                                                choice = Some((c.path.clone(), Resolution::Local));
                                            }
                                            if ui
                                                .button(tr("Conserver la version du serveur", lang))
                                                .clicked()
                                            {
                                                choice = Some((c.path, Resolution::Remote));
                                            }
                                        });
                                    }
                                });
                        }
                        Err(e) => self.fail(e),
                    }
                });
            self.conflicts_dialog = open;
            if let Some((path, choice)) = choice
                && let Err(e) = sync::resolve(&mut self.store, &path, choice)
            {
                self.fail(e);
            }
        }
    }
}

pub(super) fn account_form(ui: &mut egui::Ui, config: &mut sync::Config, lang: usize) {
    let previous = config.target;
    let target_label = ui.label(tr("Synchronisation", lang));
    egui::ComboBox::from_id_salt("sync-target")
        .selected_text(match config.target {
            sync::Target::Nextcloud => "Nextcloud",
            sync::Target::WebDav => "WebDAV",
            sync::Target::JoplinServer => "Joplin Cloud / Joplin Server",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut config.target, sync::Target::Nextcloud, "Nextcloud");
            ui.selectable_value(&mut config.target, sync::Target::WebDav, "WebDAV");
            ui.selectable_value(
                &mut config.target,
                sync::Target::JoplinServer,
                "Joplin Cloud / Joplin Server",
            );
        })
        .response
        .labelled_by(target_label.id);
    if config.target != previous {
        config.server.clear();
        config.password.zeroize();
        config.e2ee_password.zeroize();
    }
    if config.target == sync::Target::JoplinServer && ui.button("Joplin Cloud").clicked() {
        config.server = "https://api.joplincloud.com".into();
    }
    let label = ui.label(tr(
        if config.target == sync::Target::WebDav {
            "URL du dossier WebDAV"
        } else {
            "Adresse du serveur"
        },
        lang,
    ));
    ui.add(egui::TextEdit::singleline(&mut config.server).desired_width(430.0))
        .labelled_by(label.id);
    let label = ui.label(tr(
        if config.target == sync::Target::JoplinServer {
            "Adresse e-mail"
        } else {
            "Nom d’utilisateur"
        },
        lang,
    ));
    ui.text_edit_singleline(&mut config.email)
        .labelled_by(label.id);
    let label = ui.label(tr(
        if config.target == sync::Target::JoplinServer {
            "Mot de passe du compte"
        } else {
            "Mot de passe d’application"
        },
        lang,
    ));
    ui.add(
        egui::TextEdit::singleline(&mut config.password)
            .password(true)
            .char_limit(4096),
    )
    .labelled_by(label.id);
    let label = ui.label(tr(
        "Mot de passe de chiffrement de bout en bout (facultatif)",
        lang,
    ));
    ui.add(
        egui::TextEdit::singleline(&mut config.e2ee_password)
            .password(true)
            .char_limit(4096),
    )
    .labelled_by(label.id);
    ui.small(tr("Indiquez le mot de passe maître si la cible Joplin est chiffrée. Sur une cible vide, il active le chiffrement compatible Joplin (12 caractères minimum).", lang));
    if config.target == sync::Target::Nextcloud {
        ui.small(tr("Saisissez l’adresse de votre Nextcloud. Le dossier Joplin de votre compte sera utilisé ; créez-le dans Nextcloud avant la première connexion.",lang));
    }
    ui.small(tr("Le mot de passe du compte sert à la connexion ; le chiffrement local utilise une clé distincte.",lang));
}
use zeroize::Zeroize;
