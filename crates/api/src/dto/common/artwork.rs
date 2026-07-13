use serde::Serialize;

use domain::catalog::ArtworkRef;
use domain::metadata::ArtworkKind;

#[derive(Debug, Serialize)]
pub struct ImageSetDto {
    pub base: String,
    pub widths: Vec<u32>,
}

#[derive(Debug, Default, Serialize)]
pub struct ArtworkDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<ImageSetDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<ImageSetDto>,
}

impl ArtworkDto {
    pub fn from_refs(refs: Vec<ArtworkRef>) -> Self {
        let mut artwork = ArtworkDto::default();
        for art in refs {
            let slot = match art.kind {
                ArtworkKind::Poster => &mut artwork.poster,
                ArtworkKind::Backdrop => &mut artwork.backdrop,
                _ => continue,
            };
            if slot.is_none() {
                *slot = Some(ImageSetDto {
                    base: format!("/images/{}", art.id.0),
                    widths: art.widths,
                });
            }
        }
        artwork
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::ArtworkId;
    use serde_json::json;

    fn art(id: &str, kind: ArtworkKind, widths: Vec<u32>) -> ArtworkRef {
        ArtworkRef {
            id: ArtworkId(id.into()),
            kind,
            widths,
        }
    }

    #[test]
    fn empty_refs_serialize_to_empty_object() {
        let dto = ArtworkDto::from_refs(Vec::new());
        assert!(dto.poster.is_none());
        assert!(dto.backdrop.is_none());
        assert_eq!(serde_json::to_value(&dto).unwrap(), json!({}));
    }

    #[test]
    fn poster_and_backdrop_are_mapped_to_image_sets() {
        let dto = ArtworkDto::from_refs(vec![
            art("p1", ArtworkKind::Poster, vec![180, 480, 960]),
            art("b1", ArtworkKind::Backdrop, vec![480, 960]),
        ]);
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({
                "poster": {"base": "/images/p1", "widths": [180, 480, 960]},
                "backdrop": {"base": "/images/b1", "widths": [480, 960]},
            })
        );
    }

    #[test]
    fn other_kinds_are_ignored() {
        let dto = ArtworkDto::from_refs(vec![
            art("l1", ArtworkKind::Logo, vec![480]),
            art("bn1", ArtworkKind::Banner, vec![480]),
            art("ca1", ArtworkKind::ClearArt, vec![480]),
        ]);
        assert!(dto.poster.is_none());
        assert!(dto.backdrop.is_none());
    }

    #[test]
    fn first_ref_of_each_kind_wins() {
        let dto = ArtworkDto::from_refs(vec![
            art("p1", ArtworkKind::Poster, vec![180]),
            art("p2", ArtworkKind::Poster, vec![480]),
        ]);
        let poster = dto.poster.unwrap();
        assert_eq!(poster.base, "/images/p1");
        assert_eq!(poster.widths, vec![180]);
    }

    #[test]
    fn only_present_kinds_are_serialized() {
        let dto = ArtworkDto::from_refs(vec![art("p1", ArtworkKind::Poster, vec![180])]);
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({"poster": {"base": "/images/p1", "widths": [180]}})
        );
    }
}
