use super::{App, egui, tr};
impl App {
    pub(super) fn reload_after_management(&mut self) {
        let id = self.active.as_ref().map(|n| n.id.clone());
        if let Err(e) = self.refresh() {
            self.fail(e);
            return;
        }
        if let Some(id) = id {
            if self.store.note(&id).is_ok() {
                self.select(&id);
            } else {
                self.active = None;
                self.dirty = false;
            }
        }
        self.error = None;
        self.success = true;
    }
    pub(super) fn management_windows(&mut self, ctx: &egui::Context) {
        let lang = self.lang;
        if let Some(id) = self.manage_book.clone() {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr(if self.book_delete{"Supprimer le carnet"}else{"Déplacer le carnet"},lang)).open(&mut open).show(ctx,|ui|{
                if self.book_delete {ui.label(tr("Les notes seront déplacées vers le carnet choisi. Les sous-carnets seront conservés.",lang));}
                let label=ui.label(tr("Carnets",lang));
                let title=self.book_target.as_ref().and_then(|id|self.notebooks.iter().find(|b|&b.id==id)).map_or(tr("Racine",lang),|b|b.title.as_str());
                egui::ComboBox::from_id_salt("manage-book-target").selected_text(title).show_ui(ui,|ui|{
                    if !self.book_delete{ui.selectable_value(&mut self.book_target,None,tr("Racine",lang));}
                    for b in &self.notebooks {if b.id!=id{ui.selectable_value(&mut self.book_target,Some(b.id.clone()),&b.title);}}
                }).response.labelled_by(label.id);
                if ui.button(tr("Confirmer",lang)).clicked()&&self.save_draft(){
                    let result=if self.book_delete{
                        self.book_target.clone().ok_or(carnet_rust::Error::Invalid("destination notebook")).and_then(|target|self.store.remove_notebook(&id,&target))
                    }else{self.store.move_notebook(&id,self.book_target.as_deref())};
                    match result{Ok(())=>{done=true;if self.notebook.as_deref()==Some(&id){self.notebook=self.book_target.clone();}self.reload_after_management();},Err(e)=>self.fail(e)}
                }
            });
            if !open || done {
                self.manage_book = None;
            }
        }
        if let Some(tag) = self.manage_tag.clone() {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr(
                if self.tag_delete {
                    "Supprimer l’étiquette"
                } else {
                    "Renommer"
                },
                lang,
            ))
            .id(egui::Id::unique("manage-tag"))
            .open(&mut open)
            .show(ctx, |ui| {
                if !self.tag_delete {
                    let label = ui.label(tr("Nom", lang));
                    ui.text_edit_singleline(&mut self.tag_title)
                        .labelled_by(label.id);
                } else {
                    ui.label(&tag);
                }
                if ui.button(tr("Confirmer", lang)).clicked() && self.save_draft() {
                    let result = if self.tag_delete {
                        self.store.remove_tag(&tag)
                    } else {
                        self.store.rename_tag(&tag, &self.tag_title)
                    };
                    match result {
                        Ok(()) => {
                            done = true;
                            self.tag = None;
                            self.reload_after_management();
                        }
                        Err(e) => self.fail(e),
                    }
                }
            });
            if !open || done {
                self.manage_tag = None;
            }
        }
        if self.confirm_purge || self.confirm_gc {
            let mut open = true;
            let mut done = false;
            egui::Window::new(tr(if self.confirm_purge{"Vider la corbeille"}else{"Nettoyer les pièces jointes inutilisées"},lang)).open(&mut open).show(ctx,|ui|{
                ui.label(tr("Cette suppression est définitive. Les sauvegardes existantes ne seront pas modifiées.",lang));
                if ui.button(tr("Confirmer",lang)).clicked()&&self.save_draft(){
                    let result=if self.confirm_purge{self.store.purge_trash()}else{self.store.clean_unreferenced_attachments()};
                    match result{Ok(_)=>{done=true;self.reload_after_management();},Err(e)=>self.fail(e)}
                }
            });
            if !open || done {
                self.confirm_purge = false;
                self.confirm_gc = false;
            }
        }
    }
}
