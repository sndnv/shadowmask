use std::collections::HashMap;

use crate::catalog::{Series, SeriesId};

#[derive(Debug, Clone)]
pub struct SeriesCard {
    pub series: Series,
    pub season_count: u16,
}

impl SeriesCard {
    pub fn counted(series: Series, season_counts: &HashMap<SeriesId, u16>) -> Self {
        let season_count = season_counts.get(&series.id).copied().unwrap_or(0);
        Self { series, season_count }
    }
}
