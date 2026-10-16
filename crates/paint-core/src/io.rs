//! Reading and writing image files.

use std::io::{BufWriter, Write};
use std::path::Path;

use crate::raster::Raster;

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("the file is not a supported image or is damaged: {0}")]
    Decode(String),
    #[error("could not encode the image: {0}")]
    Encode(String),
    #[error("unsupported file format")]
    Unsupported,
    #[error("the image is too large ({0}×{1})")]
    TooLarge(u32, u32),
}

/// Largest side accepted when opening (keeps memory in check: 2 × 4 bytes × side²).
pub const MAX_SIDE: u32 = 30_000;

/// Raster file formats PaintRS can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Png,
    Jpeg,
    Bmp,
    Gif,
    Tiff,
    Ico,
    WebP,
}

impl Format {
    pub const ALL: [Format; 7] =
        [Format::Png, Format::Jpeg, Format::Bmp, Format::Gif, Format::Tiff, Format::Ico, Format::WebP];

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Png => &["png"],
            Format::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            Format::Bmp => &["bmp", "dib"],
            Format::Gif => &["gif"],
            Format::Tiff => &["tif", "tiff"],
            Format::Ico => &["ico"],
            Format::WebP => &["webp"],
        }
    }

    pub fn from_path(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Format::ALL.into_iter().find(|f| f.extensions().contains(&ext.as_str()))
    }

    /// Whether the format keeps an alpha channel.
    pub fn supports_alpha(self) -> bool {
        matches!(self, Format::Png | Format::Tiff | Format::Ico | Format::WebP | Format::Gif)
    }
}

/// Decodes any supported image into a premultiplied raster.
pub fn load(path: &Path) -> Result<Raster, IoError> {
    let bytes = std::fs::read(path)?;
    decode(&bytes)
}

pub fn decode(bytes: &[u8]) -> Result<Raster, IoError> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| IoError::Decode(e.to_string()))?;
    let (w, h) = reader.into_dimensions().map_err(|e| IoError::Decode(e.to_string()))?;
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(IoError::TooLarge(w, h));
    }
    let img = image::load_from_memory(bytes).map_err(|e| IoError::Decode(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Raster::from_straight_rgba8(w, h, rgba.as_raw()))
}

/// Saves `raster` (already flattened) as PNG: RGBA8, sRGB, 96 DPI — like the original.
pub fn save_png(raster: &Raster, path: &Path) -> Result<(), IoError> {
    let file = std::fs::File::create(path)?;
    let mut w = BufWriter::new(file);
    write_png(raster, &mut w)?;
    w.flush()?;
    Ok(())
}

pub fn write_png(raster: &Raster, out: impl Write) -> Result<(), IoError> {
    let mut enc = png::Encoder::new(out, raster.width(), raster.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    enc.set_pixel_dims(Some(png::PixelDimensions { xppu: 3780, yppu: 3780, unit: png::Unit::Meter }));
    let mut writer = enc.write_header().map_err(|e| IoError::Encode(e.to_string()))?;
    writer.write_image_data(&raster.to_straight_rgba8()).map_err(|e| IoError::Encode(e.to_string()))?;
    writer.finish().map_err(|e| IoError::Encode(e.to_string()))?;
    Ok(())
}

/// Writes `raster` in the format implied by the path's extension (PNG for unknown ones).
pub fn save(raster: &Raster, path: &Path) -> Result<(), IoError> {
    match Format::from_path(path).unwrap_or(Format::Png) {
        Format::Png => save_png(raster, path),
        _ => Err(IoError::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn png_roundtrip_keeps_pixels_and_alpha() {
        let mut r = Raster::new(3, 2);
        r.put(0, 0, Rgba::rgb(10, 20, 30).premultiplied());
        r.put(2, 1, Rgba::new(255, 0, 0, 128).premultiplied());
        let mut buf = Vec::new();
        write_png(&r, &mut buf).unwrap();
        let back = decode(&buf).unwrap();
        assert_eq!((back.width(), back.height()), (3, 2));
        assert_eq!(back.get(0, 0).unwrap().unpremultiplied(), Rgba::rgb(10, 20, 30));
        assert_eq!(back.get(1, 0).unwrap().alpha(), 0);
        let c = back.get(2, 1).unwrap().unpremultiplied();
        assert_eq!((c.a, c.g, c.b), (128, 0, 0));
        assert!(c.r >= 253);
    }

    #[test]
    fn png_has_96_dpi() {
        let mut buf = Vec::new();
        write_png(&Raster::new(1, 1), &mut buf).unwrap();
        let dec = png::Decoder::new(std::io::Cursor::new(buf));
        let reader = dec.read_info().unwrap();
        let dims = reader.info().pixel_dims.unwrap();
        assert_eq!((dims.xppu, dims.unit), (3780, png::Unit::Meter));
    }

    #[test]
    fn format_from_extension() {
        assert_eq!(Format::from_path(Path::new("a.JPG")), Some(Format::Jpeg));
        assert_eq!(Format::from_path(Path::new("a.tiff")), Some(Format::Tiff));
        assert_eq!(Format::from_path(Path::new("a")), None);
    }

    #[test]
    fn garbage_is_a_decode_error() {
        assert!(matches!(decode(b"not an image"), Err(IoError::Decode(_))));
    }
}
