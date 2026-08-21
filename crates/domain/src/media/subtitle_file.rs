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

#[derive(Debug, Clone)]
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
            file(
                "mt-es",
                SubtitleSource::MachineTranslated,
                Some("os-en-old"),
            ),
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
}
