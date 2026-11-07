use std::collections::{HashMap, HashSet};

use domain::catalog::{
    EpisodeCard, EpisodeContext, EpisodeId, MovieId, Series, SeriesId, TitleCard, TitleId,
    TitleListFilter,
};
use domain::error::RepositoryError;
use domain::repository::CatalogRepository;

fn split(titles: &[TitleId]) -> (Vec<MovieId>, Vec<EpisodeId>) {
    let mut movies = Vec::new();
    let mut episodes = Vec::new();
    for title in titles {
        match title {
            TitleId::Movie(id) => movies.push(id.clone()),
            TitleId::Episode(id) => episodes.push(id.clone()),
        }
    }
    (movies, episodes)
}

pub async fn visible_titles(
    catalog: &(impl CatalogRepository + Sync),
    titles: &[TitleId],
    filter: &TitleListFilter,
) -> Result<HashSet<TitleId>, RepositoryError> {
    let (movies, episodes) = split(titles);
    let shown = catalog.visible_movies(&movies, filter).await?;
    let mut visible: HashSet<TitleId> =
        shown.into_iter().map(|movie| TitleId::Movie(movie.id)).collect();
    let shown = catalog.visible_episodes(&episodes, filter).await?;
    visible.extend(shown.into_iter().map(|context| TitleId::Episode(context.episode.id)));
    Ok(visible)
}

pub async fn episode_cards(
    catalog: &(impl CatalogRepository + Sync),
    contexts: Vec<EpisodeContext>,
) -> Result<Vec<EpisodeCard>, RepositoryError> {
    let mut parents: HashMap<SeriesId, Option<Series>> = HashMap::new();
    let mut cards = Vec::with_capacity(contexts.len());
    for context in contexts {
        let parent = match parents.get(&context.series) {
            Some(found) => found.clone(),
            None => {
                let found = catalog.get_series(&context.series).await?;
                parents.insert(context.series.clone(), found.clone());
                found
            }
        };
        cards.push(EpisodeCard {
            episode: context.episode,
            series: Some(context.series),
            series_title: parent.as_ref().map(|s| s.title.clone()),
            series_artwork: parent.map(|s| s.artwork).unwrap_or_default(),
            season_number: Some(context.season_number),
            season_title: context.season_title,
        });
    }
    Ok(cards)
}

pub async fn visible_cards(
    catalog: &(impl CatalogRepository + Sync),
    titles: &[TitleId],
    filter: &TitleListFilter,
) -> Result<HashMap<TitleId, TitleCard>, RepositoryError> {
    let (movies, episodes) = split(titles);
    let mut cards: HashMap<TitleId, TitleCard> = catalog
        .visible_movies(&movies, filter)
        .await?
        .into_iter()
        .map(|movie| (TitleId::Movie(movie.id.clone()), TitleCard::Movie(movie)))
        .collect();
    let contexts = catalog.visible_episodes(&episodes, filter).await?;
    for card in episode_cards(catalog, contexts).await? {
        cards.insert(TitleId::Episode(card.episode.id.clone()), TitleCard::Episode(card));
    }
    Ok(cards)
}
