//! Bounded raster preview. Original attachments remain unchanged in the encrypted DB.
use crate::{Error, Result, store::MAX_ATTACHMENT};
use std::io::Cursor;
pub fn thumbnail_png(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() as u64 > MAX_ATTACHMENT {
        return Err(Error::Invalid("image input limit"));
    }
    let mut reader = image::ImageReader::new(Cursor::new(data)).with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(Error::Invalid("PNG/JPEG previews only"));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| Error::Invalid("image decode or dimension limit"))?;
    let thumb = decoded.thumbnail(1024, 1024);
    drop(decoded);
    let mut out = Cursor::new(Vec::new());
    thumb
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|_| Error::Invalid("thumbnail encode"))?;
    Ok(out.into_inner())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_png_preview_is_bounded_and_invalid_images_are_rejected() {
        let mut input = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1800, 200)
            .write_to(&mut input, image::ImageFormat::Png)
            .unwrap();
        let bytes = thumbnail_png(input.get_ref()).unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert!(image.width() <= 1024 && image.height() <= 1024);
        assert!(thumbnail_png(b"<svg onload='malicious()'/>").is_err());
        assert!(thumbnail_png(b"not an image").is_err());
        let mut large = Cursor::new(Vec::new());
        image::DynamicImage::new_luma8(4097, 1)
            .write_to(&mut large, image::ImageFormat::Png)
            .unwrap();
        assert!(thumbnail_png(large.get_ref()).is_err());
    }
}
