use serde::Serialize;

use domain::catalog::TitleKind;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogTitleKindDto {
    Movie,
    Series,
}

impl From<TitleKind> for CatalogTitleKindDto {
    fn from(k: TitleKind) -> Self {
        match k {
            TitleKind::Movie => CatalogTitleKindDto::Movie,
            TitleKind::Series => CatalogTitleKindDto::Series,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_both_variants() {
        assert_eq!(
            serde_json::to_string(&CatalogTitleKindDto::from(TitleKind::Movie)).unwrap(),
            "\"movie\""
        );
        assert_eq!(
            serde_json::to_string(&CatalogTitleKindDto::from(TitleKind::Series)).unwrap(),
            "\"series\""
        );
    }
}
