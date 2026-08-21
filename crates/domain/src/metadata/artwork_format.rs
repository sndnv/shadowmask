#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkFormat {
    Jpeg,
    Png,
}

impl ArtworkFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_format_names_its_extension_and_content_type() {
        assert_eq!(ArtworkFormat::Jpeg.extension(), "jpg");
        assert_eq!(ArtworkFormat::Jpeg.content_type(), "image/jpeg");
        assert_eq!(ArtworkFormat::Png.extension(), "png");
        assert_eq!(ArtworkFormat::Png.content_type(), "image/png");
    }
}
