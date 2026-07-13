use domain::catalog::{ArtworkOwner, EpisodeId, MovieId, SeriesId, TitleId, TitleKind, TitleRef};
use domain::error::RepositoryError;
use domain::metadata::{ArtworkKind, CreditRole, ExtraKind};
use domain::playback::SubtitleTrackRef;
use domain::user::Role;

use crate::pool::backend;

pub(crate) fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Admin => "admin",
        Role::User => "user",
        Role::Player => "player",
    }
}

pub(crate) fn role_from_str(value: &str) -> Result<Role, RepositoryError> {
    match value {
        "admin" => Ok(Role::Admin),
        "user" => Ok(Role::User),
        "player" => Ok(Role::Player),
        other => Err(backend(format!("unknown role: {other}"))),
    }
}

pub(crate) fn title_kind(title: &TitleId) -> &'static str {
    match title {
        TitleId::Movie(_) => "movie",
        TitleId::Episode(_) => "episode",
    }
}

pub(crate) fn title_from_parts(kind: &str, id: String) -> Result<TitleId, RepositoryError> {
    match kind {
        "movie" => Ok(TitleId::Movie(MovieId(id))),
        "episode" => Ok(TitleId::Episode(EpisodeId(id))),
        other => Err(backend(format!("unknown title kind: {other}"))),
    }
}

pub(crate) fn title_kind_to_str(kind: TitleKind) -> &'static str {
    match kind {
        TitleKind::Movie => "movie",
        TitleKind::Series => "series",
    }
}

pub(crate) fn title_kind_from_str(value: &str) -> Result<TitleKind, RepositoryError> {
    match value {
        "movie" => Ok(TitleKind::Movie),
        "series" => Ok(TitleKind::Series),
        other => Err(backend(format!("unknown title kind: {other}"))),
    }
}

pub(crate) fn title_ref_from_parts(kind: &str, id: String) -> Result<TitleRef, RepositoryError> {
    match title_kind_from_str(kind)? {
        TitleKind::Movie => Ok(TitleRef::Movie(MovieId(id))),
        TitleKind::Series => Ok(TitleRef::Series(SeriesId(id))),
    }
}

pub(crate) fn credit_role_to_str(role: CreditRole) -> &'static str {
    match role {
        CreditRole::Actor => "actor",
        CreditRole::Director => "director",
        CreditRole::Writer => "writer",
    }
}

pub(crate) fn credit_role_from_str(value: &str) -> Result<CreditRole, RepositoryError> {
    match value {
        "actor" => Ok(CreditRole::Actor),
        "director" => Ok(CreditRole::Director),
        "writer" => Ok(CreditRole::Writer),
        other => Err(backend(format!("unknown credit role: {other}"))),
    }
}

pub(crate) fn extra_kind_to_str(kind: ExtraKind) -> &'static str {
    match kind {
        ExtraKind::Trailer => "trailer",
        ExtraKind::Featurette => "featurette",
        ExtraKind::BehindTheScenes => "behind_the_scenes",
    }
}

pub(crate) fn extra_kind_from_str(value: &str) -> Result<ExtraKind, RepositoryError> {
    match value {
        "trailer" => Ok(ExtraKind::Trailer),
        "featurette" => Ok(ExtraKind::Featurette),
        "behind_the_scenes" => Ok(ExtraKind::BehindTheScenes),
        other => Err(backend(format!("unknown extra kind: {other}"))),
    }
}

pub(crate) fn subtitle_ref_parts(subtitle: &SubtitleTrackRef) -> (&'static str, String) {
    match subtitle {
        SubtitleTrackRef::Embedded(index) => ("embedded", index.to_string()),
        SubtitleTrackRef::File(id) => ("file", id.0.clone()),
    }
}

pub(crate) fn artwork_owner_parts(owner: &ArtworkOwner) -> (&'static str, &str) {
    match owner {
        ArtworkOwner::Movie(id) => ("movie", id.0.as_str()),
        ArtworkOwner::Series(id) => ("series", id.0.as_str()),
        ArtworkOwner::Season(id) => ("season", id.0.as_str()),
        ArtworkOwner::Episode(id) => ("episode", id.0.as_str()),
        ArtworkOwner::Collection(id) => ("collection", id.0.as_str()),
    }
}

pub(crate) fn artwork_kind_to_str(kind: ArtworkKind) -> &'static str {
    match kind {
        ArtworkKind::Poster => "poster",
        ArtworkKind::Backdrop => "backdrop",
        ArtworkKind::Banner => "banner",
        ArtworkKind::Logo => "logo",
        ArtworkKind::ClearArt => "clearart",
    }
}

pub(crate) fn artwork_kind_from_str(value: &str) -> Result<ArtworkKind, RepositoryError> {
    match value {
        "poster" => Ok(ArtworkKind::Poster),
        "backdrop" => Ok(ArtworkKind::Backdrop),
        "banner" => Ok(ArtworkKind::Banner),
        "logo" => Ok(ArtworkKind::Logo),
        "clearart" => Ok(ArtworkKind::ClearArt),
        other => Err(backend(format!("unknown artwork kind: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use domain::media::SubtitleFileId;

    use super::*;

    #[test]
    fn role_round_trips_and_rejects_unknown() {
        for role in [Role::Admin, Role::User, Role::Player] {
            assert_eq!(role_from_str(role_to_str(role)).unwrap(), role);
        }
        assert!(role_from_str("nope").is_err());
    }

    #[test]
    fn subtitle_ref_encodes_both_variants() {
        assert_eq!(
            subtitle_ref_parts(&SubtitleTrackRef::Embedded(3)),
            ("embedded", "3".to_owned())
        );
        assert_eq!(
            subtitle_ref_parts(&SubtitleTrackRef::File(SubtitleFileId("sub-1".into()))),
            ("file", "sub-1".to_owned())
        );
    }

    #[test]
    fn artwork_kind_round_trips_and_rejects_unknown() {
        for kind in [
            ArtworkKind::Poster,
            ArtworkKind::Backdrop,
            ArtworkKind::Banner,
            ArtworkKind::Logo,
            ArtworkKind::ClearArt,
        ] {
            assert_eq!(
                artwork_kind_from_str(artwork_kind_to_str(kind)).unwrap(),
                kind
            );
        }
        assert!(artwork_kind_from_str("nope").is_err());
    }

    #[test]
    fn title_kind_round_trips_and_rejects_unknown() {
        for kind in [TitleKind::Movie, TitleKind::Series] {
            assert_eq!(title_kind_from_str(title_kind_to_str(kind)).unwrap(), kind);
        }
        assert!(title_kind_from_str("nope").is_err());
    }

    #[test]
    fn title_ref_from_parts_builds_both_variants_and_rejects_unknown() {
        assert_eq!(
            title_ref_from_parts("movie", "m1".to_owned()).unwrap(),
            TitleRef::Movie(MovieId("m1".into()))
        );
        assert_eq!(
            title_ref_from_parts("series", "s1".to_owned()).unwrap(),
            TitleRef::Series(SeriesId("s1".into()))
        );
        assert!(title_ref_from_parts("nope", "x".to_owned()).is_err());
    }

    #[test]
    fn credit_role_round_trips_and_rejects_unknown() {
        for role in [CreditRole::Actor, CreditRole::Director, CreditRole::Writer] {
            assert_eq!(
                credit_role_from_str(credit_role_to_str(role)).unwrap(),
                role
            );
        }
        assert!(credit_role_from_str("nope").is_err());
    }

    #[test]
    fn extra_kind_round_trips_and_rejects_unknown() {
        for kind in [
            ExtraKind::Trailer,
            ExtraKind::Featurette,
            ExtraKind::BehindTheScenes,
        ] {
            assert_eq!(extra_kind_from_str(extra_kind_to_str(kind)).unwrap(), kind);
        }
        assert!(extra_kind_from_str("nope").is_err());
    }

    #[test]
    fn artwork_owner_encodes_every_kind() {
        use domain::catalog::{CollectionId, SeasonId, SeriesId};

        assert_eq!(
            artwork_owner_parts(&ArtworkOwner::Movie(MovieId("m1".into()))),
            ("movie", "m1")
        );
        assert_eq!(
            artwork_owner_parts(&ArtworkOwner::Series(SeriesId("s1".into()))),
            ("series", "s1")
        );
        assert_eq!(
            artwork_owner_parts(&ArtworkOwner::Season(SeasonId("se1".into()))),
            ("season", "se1")
        );
        assert_eq!(
            artwork_owner_parts(&ArtworkOwner::Episode(EpisodeId("e1".into()))),
            ("episode", "e1")
        );
        assert_eq!(
            artwork_owner_parts(&ArtworkOwner::Collection(CollectionId("c1".into()))),
            ("collection", "c1")
        );
    }
}
