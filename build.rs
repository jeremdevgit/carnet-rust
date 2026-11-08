// SPDX-License-Identifier: GPL-3.0-or-later — Copyright (C) 2026 jeremdevgit
fn main() {
    println!("cargo:rerun-if-changed=assets/carnet-de-notes.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/carnet-de-notes.ico")
            .set("ProductName", "Carnet de notes")
            .set("FileDescription", "Carnet de notes — encrypted notebooks")
            .set("CompanyName", "jeremdevgit")
            .set("LegalCopyright", "Copyright © 2026 jeremdevgit")
            .set("OriginalFilename", "CarnetDeNotes.exe")
            .set("InternalName", "CarnetDeNotes")
            .compile()
            .expect("Windows icon and publisher metadata must be embedded");
    }
}
