use domain::job::JobKind;

pub const ALL_KINDS: [JobKind; 19] = [
    JobKind::LibraryScan,
    JobKind::Metadata,
    JobKind::Artwork,
    JobKind::Subtitles,
    JobKind::Trickplay,
    JobKind::Fingerprint,
    JobKind::Dedup,
    JobKind::CacheEviction,
    JobKind::SearchReindex,
    JobKind::Ingest,
    JobKind::Relink,
    JobKind::Transcription,
    JobKind::Translation,
    JobKind::Upscale,
    JobKind::Combine,
    JobKind::Fetch,
    JobKind::ScheduledScan,
    JobKind::Retention,
    JobKind::OrphanSweep,
];

fn is_enrichment_job(kind: JobKind) -> bool {
    match kind {
        JobKind::Transcription | JobKind::Translation | JobKind::Upscale => true,
        JobKind::LibraryScan
        | JobKind::Metadata
        | JobKind::Artwork
        | JobKind::Subtitles
        | JobKind::Trickplay
        | JobKind::Fingerprint
        | JobKind::Dedup
        | JobKind::CacheEviction
        | JobKind::SearchReindex
        | JobKind::Ingest
        | JobKind::Relink
        | JobKind::Combine
        | JobKind::Fetch
        | JobKind::ScheduledScan
        | JobKind::Retention
        | JobKind::OrphanSweep => false,
    }
}

fn is_fetch_job(kind: JobKind) -> bool {
    matches!(kind, JobKind::Fetch)
}

pub fn enrichment_kinds() -> Vec<JobKind> {
    ALL_KINDS.into_iter().filter(|kind| is_enrichment_job(*kind)).collect()
}

pub fn fetch_kinds() -> Vec<JobKind> {
    ALL_KINDS.into_iter().filter(|kind| is_fetch_job(*kind)).collect()
}

pub fn normal_kinds() -> Vec<JobKind> {
    ALL_KINDS.into_iter().filter(|kind| !is_enrichment_job(*kind) && !is_fetch_job(*kind)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_partition_into_enrichment_fetch_and_normal() {
        assert_eq!(ALL_KINDS.len(), 19);
        let enrichment = enrichment_kinds();
        let fetch = fetch_kinds();
        let normal = normal_kinds();
        assert_eq!(enrichment.len() + fetch.len() + normal.len(), ALL_KINDS.len());
        assert!(enrichment.iter().all(|kind| !normal.contains(kind)));
        assert!(enrichment.iter().all(|kind| !fetch.contains(kind)));
        assert!(fetch.iter().all(|kind| !normal.contains(kind)));
        assert_eq!(
            enrichment,
            vec![JobKind::Transcription, JobKind::Translation, JobKind::Upscale]
        );
        assert_eq!(fetch, vec![JobKind::Fetch]);
    }

    #[test]
    fn only_transcription_translation_and_upscale_are_enrichment() {
        assert!(is_enrichment_job(JobKind::Transcription));
        assert!(is_enrichment_job(JobKind::Translation));
        assert!(is_enrichment_job(JobKind::Upscale));
        assert!(!is_enrichment_job(JobKind::LibraryScan));
        assert!(!is_enrichment_job(JobKind::Subtitles));
        assert!(!is_enrichment_job(JobKind::Combine));
        assert!(!is_enrichment_job(JobKind::Fetch));
    }

    #[test]
    fn only_fetch_is_a_fetch_job() {
        assert!(is_fetch_job(JobKind::Fetch));
        assert!(!is_fetch_job(JobKind::LibraryScan));
        assert!(!is_fetch_job(JobKind::Transcription));
    }
}
