use std::collections::HashSet;

use crate::catalog::VersionId;
use crate::common::LanguageCode;
use crate::media::SubtitleFormat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtitleFileId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtitleSource {
    OpenSubtitles,
    External,
    Generated,
    MachineTranslated,
    Combined,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleFile {
    pub id: SubtitleFileId,
    pub version: VersionId,
    pub language: Option<LanguageCode>,
    pub format: SubtitleFormat,
    pub source: SubtitleSource,
    pub path: String,
    pub translated_from: Option<SubtitleFileId>,
    pub label: Option<String>,
    pub pinned: bool,
}

pub fn prune_orphaned_translations(files: Vec<SubtitleFile>) -> Vec<SubtitleFile> {
    let ids: HashSet<&str> = files.iter().map(|file| file.id.0.as_str()).collect();
    files
        .iter()
        .filter(|file| match (&file.source, &file.translated_from) {
            (SubtitleSource::MachineTranslated, Some(source)) => ids.contains(source.0.as_str()),
            _ => true,
        })
        .cloned()
        .collect()
}

pub fn sidecars_replaced(
    held: Vec<SubtitleFile>,
    discovered: Vec<SubtitleFile>,
) -> Vec<SubtitleFile> {
    let mut fresh = discovered;
    let mut kept: Vec<SubtitleFile> = held
        .into_iter()
        .filter_map(|file| {
            if file.source != SubtitleSource::External {
                return Some(file);
            }
            let at = fresh.iter().position(|found| found.id == file.id)?;
            Some(fresh.remove(at))
        })
        .collect();
    kept.extend(fresh);
    prune_orphaned_translations(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(id: &str, source: SubtitleSource, translated_from: Option<&str>) -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId(id.into()),
            version: VersionId("v1".into()),
            language: None,
            format: SubtitleFormat::Vtt,
            source,
            path: format!("/subs/{id}.vtt"),
            translated_from: translated_from.map(|code| SubtitleFileId(code.into())),
            label: None,
            pinned: false,
        }
    }

    #[test]
    fn keeps_translations_whose_source_is_present() {
        let files = vec![
            file("os-en", SubtitleSource::OpenSubtitles, None),
            file("mt-es", SubtitleSource::MachineTranslated, Some("os-en")),
        ];
        let kept = prune_orphaned_translations(files);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn drops_translations_whose_source_is_gone() {
        let files = vec![
            file("os-en-new", SubtitleSource::OpenSubtitles, None),
            file("mt-es", SubtitleSource::MachineTranslated, Some("os-en-old")),
        ];
        let kept = prune_orphaned_translations(files);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id.0, "os-en-new");
    }

    #[test]
    fn keeps_translations_without_a_recorded_source() {
        let files = vec![file("mt-es", SubtitleSource::MachineTranslated, None)];
        let kept = prune_orphaned_translations(files);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn leaves_non_translation_rows_untouched() {
        let files = vec![
            file("ext", SubtitleSource::External, None),
            file("gen", SubtitleSource::Generated, None),
        ];
        let kept = prune_orphaned_translations(files);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn a_rescan_replaces_the_sidecars_and_keeps_every_other_source() {
        let pinned =
            SubtitleFile { pinned: true, ..file("os-en", SubtitleSource::OpenSubtitles, None) };
        let held = vec![
            file("ext-gone", SubtitleSource::External, None),
            pinned,
            file("gen", SubtitleSource::Generated, None),
            file("mt-es", SubtitleSource::MachineTranslated, Some("os-en")),
            file("cmb", SubtitleSource::Combined, None),
        ];
        let discovered = vec![file("ext-new", SubtitleSource::External, None)];

        let merged = sidecars_replaced(held, discovered);

        let ids: Vec<&str> = merged.iter().map(|file| file.id.0.as_str()).collect();
        assert_eq!(ids, ["os-en", "gen", "mt-es", "cmb", "ext-new"]);
        assert!(merged.iter().find(|file| file.id.0 == "os-en").unwrap().pinned);
    }

    #[test]
    fn a_rescan_of_unchanged_sidecars_keeps_the_order_the_list_already_has() {
        let held = vec![
            file("ext-en", SubtitleSource::External, None),
            file("os-fr", SubtitleSource::OpenSubtitles, None),
            file("ext-de", SubtitleSource::External, None),
            file("mt-es", SubtitleSource::MachineTranslated, Some("ext-en")),
        ];
        let discovered = vec![
            file("ext-de", SubtitleSource::External, None),
            file("ext-en", SubtitleSource::External, None),
        ];

        assert_eq!(sidecars_replaced(held.clone(), discovered), held);
    }

    #[test]
    fn a_sidecar_found_again_takes_its_new_details_in_its_old_place() {
        let held = vec![
            file("ext-en", SubtitleSource::External, None),
            file("os-fr", SubtitleSource::OpenSubtitles, None),
        ];
        let renamed = SubtitleFile {
            path: "/subs/ext-en.srt".into(),
            ..file("ext-en", SubtitleSource::External, None)
        };
        let discovered = vec![file("ext-pt", SubtitleSource::External, None), renamed.clone()];

        let merged = sidecars_replaced(held, discovered);

        let ids: Vec<&str> = merged.iter().map(|file| file.id.0.as_str()).collect();
        assert_eq!(ids, ["ext-en", "os-fr", "ext-pt"]);
        assert_eq!(merged[0], renamed);
    }

    #[test]
    fn a_sidecar_that_left_disk_takes_its_translation_with_it() {
        let held = vec![
            file("ext", SubtitleSource::External, None),
            file("mt-es", SubtitleSource::MachineTranslated, Some("ext")),
        ];

        let merged = sidecars_replaced(held, Vec::new());

        assert!(merged.is_empty());
    }

    #[test]
    fn a_rescan_that_finds_nothing_still_keeps_what_it_does_not_own() {
        let held = vec![file("gen", SubtitleSource::Generated, None)];

        let merged = sidecars_replaced(held, Vec::new());

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id.0, "gen");
    }
}
