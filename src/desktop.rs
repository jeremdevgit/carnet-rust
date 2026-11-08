use super::{
    App, brand_logo, configure_fonts, glass_frame, paint_glass_background, set_glass_style,
};
use carnet_rust::{
    Result,
    i18n::{self, tr},
    store::Store,
    vault::{self, ProfileState},
};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant, SystemTime},
};
use zeroize::{Zeroize, Zeroizing};

pub struct Desktop {
    path: PathBuf,
    app: Option<Box<App>>,
    state: ProfileState,
    password: Zeroizing<String>,
    confirm: Zeroizing<String>,
    worker: Option<mpsc::Receiver<Result<Store>>>,
    last_activity: Instant,
    last_activity_wall: SystemTime,
    lang: usize,
    error: Option<String>,
    focus: bool,
    use_master: bool,
    connect_account: bool,
    account: carnet_rust::sync::Config,
}
impl Desktop {
    pub fn new(path: PathBuf, cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        set_glass_style(&cc.egui_ctx, true);
        #[cfg(target_os = "windows")]
        if let Some(window) = cc.winit_window() {
            let _ = window_vibrancy::apply_mica(window, Some(true));
        }
        let mut desktop = Self::locked(path);
        if desktop.state == ProfileState::Encrypted
            && carnet_rust::device_key::is_device_managed(&desktop.path)
        {
            desktop.unlock();
        }
        desktop
    }
    fn locked(path: PathBuf) -> Self {
        let state = vault::profile_state(&path);
        let error = state.as_ref().err().map(ToString::to_string);
        Self {
            path,
            app: None,
            state: state.unwrap_or(ProfileState::Encrypted),
            password: Zeroizing::new(String::new()),
            confirm: Zeroizing::new(String::new()),
            worker: None,
            last_activity: Instant::now(),
            last_activity_wall: SystemTime::now(),
            lang: i18n::language(&sys_locale::get_locale().unwrap_or_else(|| "en".into())),
            error,
            focus: true,
            use_master: false,
            connect_account: true,
            account: carnet_rust::sync::Config::default(),
        }
    }
    fn unlock(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let device = !self.use_master
            && (self.state != ProfileState::Encrypted
                || carnet_rust::device_key::is_device_managed(&self.path));
        if self.state != ProfileState::Encrypted && self.connect_account {
            if let Err(e) = carnet_rust::sync::endpoint(&self.account) {
                self.error = Some(e.to_string());
                return;
            }
            if self.account.email.is_empty() || self.account.password.is_empty() {
                self.error = Some(tr("Renseignez les identifiants du compte", self.lang).into());
                return;
            }
        }
        if self.state != ProfileState::Encrypted && !device {
            if self.password.as_str() != self.confirm.as_str() {
                self.error =
                    Some(tr("Les phrases de passe ne correspondent pas", self.lang).into());
                return;
            }
            if vault::validate_new_password(self.password.as_str()).is_err() {
                self.error = Some(tr("Utilisez au moins 12 caractères", self.lang).into());
                return;
            }
        }
        let account = if self.state != ProfileState::Encrypted && self.connect_account {
            Some(self.account.clone())
        } else {
            None
        };
        let password = std::mem::take(&mut self.password);
        self.confirm.zeroize();
        self.error = None;
        let path = self.path.clone();
        let (send, receive) = mpsc::channel();
        self.worker = Some(receive);
        std::thread::spawn(move || {
            let result = (|| {
                let store = if device {
                    carnet_rust::device_key::open(&path)?
                } else {
                    Store::open(&path, password.as_str())?
                };
                if let Some(account) = account {
                    carnet_rust::sync::save_config(&store, &account)?;
                }
                Ok(store)
            })();
            let _ = send.send(result);
        });
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let result = self
            .worker
            .as_ref()
            .and_then(|worker| match worker.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(carnet_rust::Error::Invalid("unlock task stopped")))
                }
            });
        if let Some(result) = result {
            self.worker = None;
            match result.and_then(|store| App::from_store(store, ctx)) {
                Ok(app) => {
                    self.account = carnet_rust::sync::Config::default();
                    self.lang = app.lang;
                    self.app = Some(Box::new(app));
                    self.last_activity = Instant::now();
                    self.last_activity_wall = SystemTime::now();
                    self.state = ProfileState::Encrypted;
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    self.focus = true;
                }
            }
        }
        if self.worker.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn lock(&mut self, ctx: &egui::Context) -> bool {
        if let Some(app) = self.app.as_mut() {
            if !app.prepare_lock() {
                app.lock_requested = false;
                return false;
            }
            self.lang = app.lang;
        }
        self.app = None;
        self.password.zeroize();
        self.confirm.zeroize();
        self.error = None;
        self.focus = true;
        // Drop widget edit states and galleys from the unlocked workspace.
        ctx.memory_mut(|m| *m = egui::Memory::default());
        ctx.forget_all_images();
        configure_fonts(ctx);
        set_glass_style(ctx, true);
        true
    }
    fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        if let Some(app) = self.app.as_ref().filter(|a| a.auto_lock_minutes > 0) {
            let timeout = Duration::from_secs(u64::from(app.auto_lock_minutes) * 60);
            // Check expiry before processing input: the first event after sleep must
            // not accidentally postpone a lock that was already due.
            if self.last_activity.elapsed() >= timeout
                || self
                    .last_activity_wall
                    .elapsed()
                    .is_ok_and(|d| d >= timeout)
            {
                self.lock(&ctx);
            } else {
                ctx.request_repaint_after(timeout.saturating_sub(self.last_activity.elapsed()));
            }
        }
        if self.app.is_some() {
            if ctx.input(|i| !i.events.is_empty()) {
                self.last_activity = Instant::now();
                self.last_activity_wall = SystemTime::now();
            }
            let app = self.app.as_mut().expect("checked app");
            app.draw(ui);
            if app.lock_requested {
                self.lock(&ctx);
            }
            return;
        }
        paint_glass_background(ui, true);
        let lang = self.lang;
        let setup = self.state != ProfileState::Encrypted;
        let busy = self.worker.is_some();
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui,|ui|{
            egui::ScrollArea::vertical().show(ui,|ui|{ui.vertical_centered(|ui|{
                ui.add_space((ui.available_height()*0.03).max(10.0));
                ui.set_max_width(600.0);
                glass_frame(true).show(ui,|ui|{
                    brand_logo(ui, 88.0);
                    ui.heading("Carnet de notes");
                    ui.label(tr(if setup{"Créer un coffre chiffré"}else{"Déverrouiller le coffre"},lang));
                    if self.state==ProfileState::Plaintext {ui.label(tr("Votre ancien profil sera chiffré sans modifier vos notes.",lang));}
                    let device=carnet_rust::device_key::is_device_managed(&self.path);
                    if setup {
                        ui.checkbox(&mut self.connect_account,tr("Se connecter à un compte",lang));
                        if self.connect_account {super::sync_ui::account_form(ui,&mut self.account,lang);}
                        ui.separator();
                        ui.checkbox(&mut self.use_master,tr("Utiliser un mot de passe maître (optionnel)",lang));
                        if !self.use_master {ui.label(tr("Les données locales seront chiffrées avec une clé protégée par votre session système.",lang));}
                    } else if device {
                        ui.label(tr("Coffre chiffré protégé par votre session système.",lang));
                        ui.checkbox(&mut self.use_master,tr("Utiliser mon ancien mot de passe maître",lang));
                    }
                    let mut enter=false;
                    if (setup && self.use_master) || (!setup && (!device || self.use_master)) {
                        ui.label(tr("Sans cette phrase de passe, aucune récupération n’est possible.",lang));
                        let label=ui.label(tr("Phrase de passe",lang));
                        let response=ui.add_enabled(!busy,egui::TextEdit::singleline(&mut *self.password).password(true).char_limit(1024).desired_width(430.0)).labelled_by(label.id);
                        if self.focus&&!busy{response.request_focus();self.focus=false;}
                        enter=response.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter));
                        if setup {
                            let label=ui.label(tr("Confirmer la phrase de passe",lang));
                            ui.add_enabled(!busy,egui::TextEdit::singleline(&mut *self.confirm).password(true).char_limit(1024).desired_width(430.0)).labelled_by(label.id);
                            ui.small(tr("Utilisez au moins 12 caractères",lang));
                        }
                    }
                    if (ui.add_enabled(!busy,egui::Button::new(tr(if setup&&self.connect_account{"Se connecter et synchroniser"}else if setup{"Chiffrer et ouvrir"}else{"Déverrouiller"},lang))).clicked()||enter)&&!busy {self.unlock();}
                    if busy{ui.spinner();ui.label(tr("Ouverture du coffre…",lang));}
                    if self.error.is_some(){
                        ui.colored_label(egui::Color32::from_rgb(245,140,130),tr("Ouverture impossible : vérifiez la phrase de passe et la sauvegarde.",lang));
                        ui.collapsing(tr("Détails techniques",lang),|ui|{ui.label(self.error.as_deref().unwrap_or(""));});
                    }
                    ui.separator();
                    let label=ui.label(tr("Langue",lang));
                    egui::ComboBox::from_id_salt("lock-language").selected_text(i18n::NAMES[self.lang]).show_ui(ui,|ui|{
                        for (i,name) in i18n::NAMES.iter().enumerate(){ui.selectable_value(&mut self.lang,i,*name);}
                    }).response.labelled_by(label.id);
                });
            });});
        });
    }
}
impl eframe::App for Desktop {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lock_drops_profile_key_and_preserves_invalid_draft() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path(), "Demonstration password").unwrap();
        let book = store.create_notebook("Example", None).unwrap();
        let note = store.create_note(&book, "Example", false).unwrap();
        let ctx = egui::Context::default();
        let mut app = App::from_store(store, &ctx).unwrap();
        app.select(&note.id);
        app.active.as_mut().unwrap().title.clear();
        app.active.as_mut().unwrap().body = "An unfinished encrypted draft".into();
        app.changed();
        let mut desktop = Desktop::locked(dir.path().into());
        desktop.app = Some(Box::new(app));
        assert!(desktop.lock(&ctx));
        assert!(desktop.app.is_none());
        let store = Store::open(dir.path(), "Demonstration password").unwrap();
        let draft = store.recovery_draft(&note.id).unwrap().unwrap();
        assert_eq!(draft.0.body, "An unfinished encrypted draft");
        assert!(draft.0.title.is_empty());
    }
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    use egui_kittest::Harness;
    #[test]
    fn locked_screen_is_accessible_in_all_eight_languages() {
        let dir = tempfile::tempdir().unwrap();
        for lang in 0..8 {
            let mut desktop = Desktop::locked(dir.path().into());
            desktop.lang = lang;
            let mut ready = false;
            let mut h = Harness::builder()
                .with_size(egui::vec2(1340.0, 860.0))
                .with_max_steps(20)
                .build_ui_state(
                    move |ui, d: &mut Desktop| {
                        if !ready {
                            configure_fonts(ui.ctx());
                            set_glass_style(ui.ctx(), true);
                            ready = true;
                        }
                        d.draw(ui);
                    },
                    desktop,
                );
            h.run();
            assert!(h.state().app.is_none());
        }
    }
    #[test]
    fn first_input_after_idle_does_not_bypass_lock() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), "Demonstration vault phrase").unwrap();
        let ctx = egui::Context::default();
        let app = App::from_store(store, &ctx).unwrap();
        let mut desktop = Desktop::locked(dir.path().into());
        desktop.app = Some(Box::new(app));
        desktop.last_activity = Instant::now() - Duration::from_secs(301);
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(egui::pos2(20.0, 20.0))],
                ..Default::default()
            },
            |ui| desktop.draw(ui),
        );
        output.textures_delta.clear();
        assert!(desktop.app.is_none());
    }
    #[test]
    #[ignore = "offscreen GPU render; set CARNET_RENDER_DIR"]
    fn render_locked_vault() {
        let dir = tempfile::tempdir().unwrap();
        let mut desktop = Desktop::locked(dir.path().into());
        desktop.lang = 0;
        let mut ready = false;
        let mut h = Harness::builder()
            .with_size(egui::vec2(1340.0, 860.0))
            .with_max_steps(20)
            .build_ui_state(
                move |ui, d: &mut Desktop| {
                    if !ready {
                        configure_fonts(ui.ctx());
                        set_glass_style(ui.ctx(), true);
                        ready = true;
                    }
                    d.draw(ui);
                },
                desktop,
            );
        h.run();
        let out = PathBuf::from(std::env::var_os("CARNET_RENDER_DIR").expect("CARNET_RENDER_DIR"));
        std::fs::create_dir_all(&out).unwrap();
        h.render()
            .unwrap()
            .save(out.join("encrypted-vault.png"))
            .unwrap();
    }
}
