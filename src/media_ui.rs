use carnet_rust::{
    media,
    store::{Store, valid_id},
};
use eframe::egui;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;
#[derive(Default)]
pub(super) struct PreviewCache {
    pub key: String,
    rendered: String,
    uris: Vec<String>,
}
impl PreviewCache {
    pub(super) fn prepare<'a>(
        &'a mut self,
        store: &Store,
        ctx: &egui::Context,
        body: &str,
    ) -> &'a str {
        let key = format!("{:x}", Sha256::digest(body.as_bytes()));
        if key == self.key {
            return &self.rendered;
        }
        for uri in self.uris.drain(..) {
            ctx.forget_image(&uri);
        }
        self.rendered.zeroize();
        self.rendered = body.to_string();
        self.key = key;
        let mut replacements = Vec::new();
        let mut loaded = std::collections::BTreeMap::<String, String>::new();
        for (event, range) in
            pulldown_cmark::Parser::new_ext(body, pulldown_cmark::Options::all()).into_offset_iter()
        {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) = event
            {
                if replacements
                    .last()
                    .is_some_and(|(r, _): &(std::ops::Range<usize>, String)| r.end > range.start)
                {
                    continue;
                }
                let uri = dest_url
                    .strip_prefix(":/")
                    .filter(|id| valid_id(id))
                    .and_then(|id| {
                        if let Some(uri) = loaded.get(id) {
                            return Some(uri.clone());
                        }
                        if loaded.len() >= 4 {
                            return None;
                        }
                        let png = store
                            .attachment_data(id)
                            .and_then(|data| media::thumbnail_png(&data))
                            .ok()?;
                        let uri = format!("bytes://carnet/{id}-{}.png", &self.key[..12]);
                        ctx.include_bytes(uri.clone(), png);
                        self.uris.push(uri.clone());
                        loaded.insert(id.into(), uri.clone());
                        Some(uri)
                    });
                let alt = || {
                    let mut text = String::new();
                    for e in pulldown_cmark::Parser::new(&body[range.clone()]) {
                        if let pulldown_cmark::Event::Text(t) | pulldown_cmark::Event::Code(t) = e {
                            text.push_str(&t);
                        }
                    }
                    text.chars()
                        .take(512)
                        .collect::<String>()
                        .replace('\\', "\\\\")
                        .replace('[', "\\[")
                        .replace(']', "\\]")
                        .replace(['\n', '\r'], " ")
                };
                let replacement =
                    uri.map_or_else(|| "[—]".to_string(), |uri| format!("![{}]({uri})", alt()));
                replacements.push((range, replacement));
            }
        }
        for (range, text) in replacements.into_iter().rev() {
            self.rendered.replace_range(range, &text);
        }
        &self.rendered
    }
}
impl Drop for PreviewCache {
    fn drop(&mut self) {
        self.rendered.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_validated_local_raster_bytes_reach_the_preview_loader() {
        let store = Store::memory().unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let id = store.add_attachment("example.png", png.get_ref()).unwrap();
        let ctx = egui::Context::default();
        let mut cache = PreviewCache::default();
        let body = format!(
            "![local](:/{id})\n\n![remote](https://example.invalid/tracker.png)\n\n![file](file:///private/example)"
        );
        let rendered = cache.prepare(&store, &ctx, &body);
        assert!(rendered.contains("bytes://carnet/"));
        assert!(!rendered.contains("https://"));
        assert!(!rendered.contains("file:///"));
        for e in pulldown_cmark::Parser::new_ext(rendered, pulldown_cmark::Options::all()) {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { dest_url, .. }) = e {
                assert!(dest_url.starts_with("bytes://carnet/"));
            }
        }
    }
}
