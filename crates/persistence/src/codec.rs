use domain::catalog::{EpisodeId, MovieId, TitleId};
use domain::error::RepositoryError;
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

pub(crate) fn subtitle_ref_parts(subtitle: &SubtitleTrackRef) -> (&'static str, String) {
    match subtitle {
        SubtitleTrackRef::Embedded(index) => ("embedded", index.to_string()),
        SubtitleTrackRef::File(id) => ("file", id.0.clone()),
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
}
