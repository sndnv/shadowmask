use domain::catalog::{SeriesId, TitleId};
use domain::common::PageRequest;
use domain::library::LibraryId;
use domain::repository::{CatalogRepository, LibraryRepository};

pub async fn version_library<C>(catalog: &C, title: &TitleId) -> Option<LibraryId>
where
    C: CatalogRepository + Sync,
{
    let first = PageRequest { offset: 0, limit: 1 };
    let versions = catalog.list_versions(title, first).await.ok()?;
    versions.items.first().map(|version| version.library.clone())
}

pub async fn series_library<C>(catalog: &C, series: &SeriesId) -> Option<LibraryId>
where
    C: CatalogRepository + Sync,
{
    for season in catalog.list_seasons(series).await.unwrap_or_default() {
        for episode in catalog.list_episodes(&season.id).await.unwrap_or_default() {
            if let Some(library) = version_library(catalog, &TitleId::Episode(episode.id)).await {
                return Some(library);
            }
        }
    }
    None
}

pub async fn library_articles<L>(libraries: &L, library: Option<LibraryId>) -> Vec<String>
where
    L: LibraryRepository + Sync,
{
    let Some(id) = library else {
        return Vec::new();
    };
    match libraries.get(&id).await {
        Ok(Some(library)) => library.sort_articles,
        _ => Vec::new(),
    }
}
