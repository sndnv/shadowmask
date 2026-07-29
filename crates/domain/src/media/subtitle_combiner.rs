use crate::error::SubtitleError;
use crate::media::FetchedSubtitle;

pub trait SubtitleCombiner {
    fn combine(
        &self,
        primary: &FetchedSubtitle,
        secondary: &FetchedSubtitle,
    ) -> Result<FetchedSubtitle, SubtitleError>;
}
