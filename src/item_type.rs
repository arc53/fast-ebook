/// Item type constants for EPUB content classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ItemType {
    Unknown = 0,
    Image = 1,
    Style = 2,
    Script = 3,
    Navigation = 4,
    Vector = 5,
    Font = 6,
    Video = 7,
    Audio = 8,
    Document = 9,
    Cover = 10,
    Smil = 11,
}

impl ItemType {
    /// Determine item type from media-type and optional OPF properties.
    pub fn from_media_type(media_type: &str, properties: Option<&str>) -> Self {
        // Check properties first — they override media-type classification
        if let Some(props) = properties {
            if props.contains("cover-image") {
                return ItemType::Cover;
            }
            if props.contains("nav") {
                return ItemType::Navigation;
            }
        }

        // NCX is always navigation
        if media_type == "application/x-dtbncx+xml" {
            return ItemType::Navigation;
        }

        match media_type {
            // Documents
            "application/xhtml+xml" | "text/html" => ItemType::Document,

            // Stylesheets
            "text/css" => ItemType::Style,

            // Scripts
            "application/javascript" | "text/javascript" | "application/ecmascript" => {
                ItemType::Script
            }

            // Vector images
            "image/svg+xml" => ItemType::Vector,

            // Raster images
            m if m.starts_with("image/") => ItemType::Image,

            // Fonts
            m if m.starts_with("font/")
                || m.starts_with("application/font-")
                || m == "application/x-font-ttf"
                || m == "application/x-font-opentype"
                || m == "application/vnd.ms-opentype" =>
            {
                ItemType::Font
            }

            // Video
            m if m.starts_with("video/") => ItemType::Video,

            // Audio
            m if m.starts_with("audio/") => ItemType::Audio,

            // SMIL
            "application/smil+xml" => ItemType::Smil,

            _ => ItemType::Unknown,
        }
    }

    /// Convert a u8 constant back to an ItemType.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => ItemType::Unknown,
            1 => ItemType::Image,
            2 => ItemType::Style,
            3 => ItemType::Script,
            4 => ItemType::Navigation,
            5 => ItemType::Vector,
            6 => ItemType::Font,
            7 => ItemType::Video,
            8 => ItemType::Audio,
            9 => ItemType::Document,
            10 => ItemType::Cover,
            11 => ItemType::Smil,
            _ => ItemType::Unknown,
        }
    }
}

/// Guess MIME type from a file extension.
pub fn guess_media_type(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "xhtml" | "html" | "htm" => "application/xhtml+xml",
        "css" => "text/css",
        "js" => "application/javascript",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "ncx" => "application/x-dtbncx+xml",
        "smil" => "application/smil+xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xhtml_is_document() {
        assert_eq!(
            ItemType::from_media_type("application/xhtml+xml", None),
            ItemType::Document
        );
    }

    #[test]
    fn test_css_is_style() {
        assert_eq!(ItemType::from_media_type("text/css", None), ItemType::Style);
    }

    #[test]
    fn test_jpeg_is_image() {
        assert_eq!(
            ItemType::from_media_type("image/jpeg", None),
            ItemType::Image
        );
    }

    #[test]
    fn test_svg_is_vector() {
        assert_eq!(
            ItemType::from_media_type("image/svg+xml", None),
            ItemType::Vector
        );
    }

    #[test]
    fn test_ncx_is_navigation() {
        assert_eq!(
            ItemType::from_media_type("application/x-dtbncx+xml", None),
            ItemType::Navigation
        );
    }

    #[test]
    fn test_cover_image_property_overrides() {
        assert_eq!(
            ItemType::from_media_type("image/jpeg", Some("cover-image")),
            ItemType::Cover
        );
    }

    #[test]
    fn test_nav_property_overrides() {
        assert_eq!(
            ItemType::from_media_type("application/xhtml+xml", Some("nav")),
            ItemType::Navigation
        );
    }

    #[test]
    fn test_font_types() {
        assert_eq!(ItemType::from_media_type("font/otf", None), ItemType::Font);
        assert_eq!(
            ItemType::from_media_type("application/font-woff", None),
            ItemType::Font
        );
        assert_eq!(
            ItemType::from_media_type("application/vnd.ms-opentype", None),
            ItemType::Font
        );
    }

    #[test]
    fn test_audio_video() {
        assert_eq!(
            ItemType::from_media_type("audio/mpeg", None),
            ItemType::Audio
        );
        assert_eq!(
            ItemType::from_media_type("video/mp4", None),
            ItemType::Video
        );
    }

    #[test]
    fn test_unknown() {
        assert_eq!(
            ItemType::from_media_type("application/octet-stream", None),
            ItemType::Unknown
        );
    }
}
