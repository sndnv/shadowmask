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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub posters: Vec<ImageSetDto>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub backdrops: Vec<ImageSetDto>,
}

impl ArtworkDto {
    pub fn is_empty(&self) -> bool {
        self.posters.is_empty() && self.backdrops.is_empty()
    }

    pub fn from_refs(refs: Vec<ArtworkRef>) -> Self {
        let mut artwork = ArtworkDto::default();
        for art in refs {
            let set = ImageSetDto {
                base: format!("/images/{}", art.id.0),
                widths: art.sizes(),
            };
            match art.kind {
                ArtworkKind::Poster => artwork.posters.push(set),
                ArtworkKind::Backdrop => artwork.backdrops.push(set),
                _ => {}
            }
        }
        artwork
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{ArtworkId, ArtworkWidth};
    use serde_json::json;

    fn art(id: &str, kind: ArtworkKind, widths: Vec<u32>) -> ArtworkRef {
        ArtworkRef {
            id: ArtworkId(id.into()),
            kind,
            widths: widths
                .into_iter()
                .map(|width| ArtworkWidth::new(width, format!("/art/{id}/{width}.jpg")))
                .collect(),
        }
    }

    #[test]
    fn empty_refs_serialize_to_empty_object() {
        let dto = ArtworkDto::from_refs(Vec::new());
        assert!(dto.posters.is_empty());
        assert!(dto.backdrops.is_empty());
        assert_eq!(serde_json::to_value(&dto).unwrap(), json!({}));
    }

    #[test]
    fn posters_and_backdrops_are_mapped_to_image_sets() {
        let dto = ArtworkDto::from_refs(vec![
            art("p1", ArtworkKind::Poster, vec![180, 480, 960]),
            art("b1", ArtworkKind::Backdrop, vec![480, 960]),
        ]);
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({
                "posters": [{"base": "/images/p1", "widths": [180, 480, 960]}],
                "backdrops": [{"base": "/images/b1", "widths": [480, 960]}],
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
        assert!(dto.posters.is_empty());
        assert!(dto.backdrops.is_empty());
    }

    #[test]
    fn all_refs_of_each_kind_are_collected_in_order() {
        let dto = ArtworkDto::from_refs(vec![
            art("p1", ArtworkKind::Poster, vec![180]),
            art("p2", ArtworkKind::Poster, vec![480]),
        ]);
        assert_eq!(dto.posters.len(), 2);
        assert_eq!(dto.posters[0].base, "/images/p1");
        assert_eq!(dto.posters[1].base, "/images/p2");
    }

    #[test]
    fn only_present_kinds_are_serialized() {
        let dto = ArtworkDto::from_refs(vec![art("p1", ArtworkKind::Poster, vec![180])]);
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({"posters": [{"base": "/images/p1", "widths": [180]}]})
        );
    }
}
