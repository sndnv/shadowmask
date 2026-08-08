use crate::error::SubtitleError;
use crate::media::FetchedSubtitle;

pub trait SubtitleCombiner {
    fn combine(
        &self,
        top: &FetchedSubtitle,
        bottom: &FetchedSubtitle,
    ) -> Result<FetchedSubtitle, SubtitleError>;
}
