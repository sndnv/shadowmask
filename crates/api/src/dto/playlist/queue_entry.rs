use serde::Serialize;

use domain::playback::QueueCard;

use crate::dto::catalog::TitleCardResponse;

#[derive(Debug, Serialize)]
pub struct QueueEntryResponse {
    pub id: String,
    pub card: TitleCardResponse,
}

impl From<QueueCard> for QueueEntryResponse {
    fn from(q: QueueCard) -> Self {
        QueueEntryResponse { id: q.entry.id.0, card: q.card.into() }
    }
}
