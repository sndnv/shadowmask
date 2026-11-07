use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::catalog::{TitleId, VersionId};
use domain::error::RepositoryError;
use domain::playback::{
    Favorite, Playlist, PlaylistId, PlaylistItem, QueueEntry, QueueEntryId, SubtitleTrackRef,
    UserSubtitleOffset, WatchlistItem,
};
use domain::repository::PreferencesRepository;
use domain::user::UserId;
use jiff::Timestamp;

#[derive(Default)]
struct State {
    watchlist: Vec<WatchlistItem>,
    favorites: Vec<Favorite>,
    offsets: Vec<UserSubtitleOffset>,
    playlists: Vec<(UserId, Playlist)>,
    playlist_items: Vec<(UserId, PlaylistId, PlaylistItem)>,
    queue: Vec<(UserId, QueueEntry)>,
    queued: u64,
}

impl State {
    fn items(&self, user: &UserId, id: &PlaylistId) -> Vec<PlaylistItem> {
        let mut items: Vec<PlaylistItem> = self
            .playlist_items
            .iter()
            .filter(|(owner, playlist, _)| owner == user && playlist == id)
            .map(|(_, _, item)| item.clone())
            .collect();
        items.sort_by_key(|item| item.position);
        items
    }

    fn enqueue(&mut self, user: &UserId, title: TitleId) {
        self.queued += 1;
        let entry = QueueEntry { id: QueueEntryId(self.queued.to_string()), title };
        self.queue.push((user.clone(), entry));
    }

    fn touch(&mut self, user: &UserId, id: &PlaylistId, at: Timestamp) {
        for (owner, playlist) in &mut self.playlists {
            if owner == user && &playlist.id == id {
                playlist.updated_at = at;
            }
        }
    }

    fn reposition(&mut self, user: &UserId, id: &PlaylistId, order: &[PlaylistItem]) {
        self.playlist_items.retain(|(owner, playlist, _)| !(owner == user && playlist == id));
        for (position, item) in (0..).zip(order) {
            self.playlist_items.push((
                user.clone(),
                id.clone(),
                PlaylistItem { position, ..item.clone() },
            ));
        }
    }
}

#[derive(Clone, Default)]
pub struct MockPreferencesRepo {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockPreferencesRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_watchlist(&self, item: WatchlistItem) {
        self.state.lock().unwrap().watchlist.push(item);
    }

    pub fn seed_playlist(&self, user: &UserId, playlist: Playlist, titles: &[TitleId]) {
        let mut state = self.state.lock().unwrap();
        for (position, title) in (0..).zip(titles) {
            state.playlist_items.push((
                user.clone(),
                playlist.id.clone(),
                PlaylistItem { position, title: title.clone(), added_at: playlist.created_at },
            ));
        }
        state.playlists.push((user.clone(), playlist));
    }

    pub fn seed_queue(&self, user: &UserId, titles: &[TitleId]) {
        let mut state = self.state.lock().unwrap();
        for title in titles {
            state.enqueue(user, title.clone());
        }
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            Err(RepositoryError::Backend("mock preferences failure".to_owned()))
        } else {
            Ok(())
        }
    }
}

impl PreferencesRepository for MockPreferencesRepo {
    async fn list_watchlist(&self, user: &UserId) -> Result<Vec<WatchlistItem>, RepositoryError> {
        self.guard()?;
        let mut items: Vec<WatchlistItem> = self
            .state
            .lock()
            .unwrap()
            .watchlist
            .iter()
            .filter(|i| &i.user == user)
            .cloned()
            .collect();
        items.sort_by(|a, b| {
            a.added_at
                .cmp(&b.added_at)
                .then_with(|| saved_order(&a.title).cmp(&saved_order(&b.title)))
        });
        Ok(items)
    }

    async fn add_watchlist(&self, item: WatchlistItem) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        match state.watchlist.iter_mut().find(|i| i.user == item.user && i.title == item.title) {
            Some(existing) => *existing = item,
            None => state.watchlist.push(item),
        }
        Ok(())
    }

    async fn remove_watchlist(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .watchlist
            .retain(|i| !(&i.user == user && i.title.id() == title_id));
        Ok(())
    }

    async fn list_favorites(&self, user: &UserId) -> Result<Vec<Favorite>, RepositoryError> {
        self.guard()?;
        let mut items: Vec<Favorite> = self
            .state
            .lock()
            .unwrap()
            .favorites
            .iter()
            .filter(|i| &i.user == user)
            .cloned()
            .collect();
        items.sort_by(|a, b| {
            a.added_at
                .cmp(&b.added_at)
                .then_with(|| saved_order(&a.title).cmp(&saved_order(&b.title)))
        });
        Ok(items)
    }

    async fn add_favorite(&self, item: Favorite) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        match state.favorites.iter_mut().find(|i| i.user == item.user && i.title == item.title) {
            Some(existing) => *existing = item,
            None => state.favorites.push(item),
        }
        Ok(())
    }

    async fn remove_favorite(&self, user: &UserId, title_id: &str) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .favorites
            .retain(|i| !(&i.user == user && i.title.id() == title_id));
        Ok(())
    }

    async fn get_subtitle_offset(
        &self,
        user: &UserId,
        version: &VersionId,
        subtitle: &SubtitleTrackRef,
    ) -> Result<Option<UserSubtitleOffset>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .offsets
            .iter()
            .find(|o| &o.user == user && &o.version == version && &o.subtitle == subtitle)
            .cloned())
    }

    async fn set_subtitle_offset(&self, offset: UserSubtitleOffset) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.offsets.iter_mut().find(|o| {
            o.user == offset.user && o.version == offset.version && o.subtitle == offset.subtitle
        }) {
            *existing = offset;
        } else {
            state.offsets.push(offset);
        }
        Ok(())
    }

    async fn list_playlists(&self, user: &UserId) -> Result<Vec<Playlist>, RepositoryError> {
        self.guard()?;
        let mut playlists: Vec<Playlist> = self
            .state
            .lock()
            .unwrap()
            .playlists
            .iter()
            .filter(|(owner, _)| owner == user)
            .map(|(_, playlist)| playlist.clone())
            .collect();
        playlists.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        Ok(playlists)
    }

    async fn get_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<Option<Playlist>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .playlists
            .iter()
            .find(|(owner, playlist)| owner == user && &playlist.id == id)
            .map(|(_, playlist)| playlist.clone()))
    }

    async fn save_playlist(
        &self,
        user: &UserId,
        playlist: Playlist,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        match state.playlists.iter_mut().find(|(owner, p)| owner == user && p.id == playlist.id) {
            Some((_, existing)) => {
                existing.name = playlist.name;
                existing.updated_at = playlist.updated_at;
            }
            None => state.playlists.push((user.clone(), playlist)),
        }
        Ok(())
    }

    async fn delete_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        state.playlist_items.retain(|(owner, playlist, _)| !(owner == user && playlist == id));
        let before = state.playlists.len();
        state.playlists.retain(|(owner, playlist)| !(owner == user && &playlist.id == id));
        Ok(state.playlists.len() < before)
    }

    async fn list_playlist_items(
        &self,
        user: &UserId,
        id: &PlaylistId,
    ) -> Result<Vec<PlaylistItem>, RepositoryError> {
        self.guard()?;
        Ok(self.state.lock().unwrap().items(user, id))
    }

    async fn add_playlist_items(
        &self,
        user: &UserId,
        id: &PlaylistId,
        titles: &[TitleId],
        at: Timestamp,
    ) -> Result<Option<usize>, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if !state.playlists.iter().any(|(owner, playlist)| owner == user && &playlist.id == id) {
            return Ok(None);
        }
        let mut items = state.items(user, id);
        let mut position = items.last().map_or(0, |item| item.position + 1);
        let mut added = 0;
        for title in titles {
            if items.iter().any(|item| &item.title == title) {
                continue;
            }
            let item = PlaylistItem { position, title: title.clone(), added_at: at };
            state.playlist_items.push((user.clone(), id.clone(), item.clone()));
            items.push(item);
            position += 1;
            added += 1;
        }
        if added > 0 {
            state.touch(user, id, at);
        }
        Ok(Some(added))
    }

    async fn remove_playlist_item(
        &self,
        user: &UserId,
        id: &PlaylistId,
        title_id: &str,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let mut items = state.items(user, id);
        let before = items.len();
        items.retain(|item| item.title.id() != title_id);
        if items.len() < before {
            state.reposition(user, id, &items);
            state.touch(user, id, at);
        }
        Ok(())
    }

    async fn move_playlist_item(
        &self,
        user: &UserId,
        id: &PlaylistId,
        title_id: &str,
        position: u32,
        at: Timestamp,
    ) -> Result<bool, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let mut items = state.items(user, id);
        let Some(from) = items.iter().position(|item| item.title.id() == title_id) else {
            return Ok(false);
        };
        let to = items.iter().position(|item| item.position >= position).unwrap_or(items.len() - 1);
        let item = items.remove(from);
        items.insert(to, item);
        state.reposition(user, id, &items);
        state.touch(user, id, at);
        Ok(true)
    }

    async fn list_queue(&self, user: &UserId) -> Result<Vec<QueueEntry>, RepositoryError> {
        self.guard()?;
        Ok(self
            .state
            .lock()
            .unwrap()
            .queue
            .iter()
            .filter(|(owner, _)| owner == user)
            .map(|(_, entry)| entry.clone())
            .collect())
    }

    async fn queue_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
        shuffle: bool,
    ) -> Result<usize, RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        let mut titles: Vec<TitleId> =
            state.items(user, id).into_iter().map(|item| item.title).collect();
        if shuffle {
            titles.reverse();
        }
        let added = titles.len();
        for title in titles {
            state.enqueue(user, title);
        }
        Ok(added)
    }

    async fn clear_queue(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state.lock().unwrap().queue.retain(|(owner, _)| owner != user);
        Ok(())
    }

    async fn remove_queue_entry(
        &self,
        user: &UserId,
        id: &QueueEntryId,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        self.state
            .lock()
            .unwrap()
            .queue
            .retain(|(owner, entry)| !(owner == user && &entry.id == id));
        Ok(())
    }

    async fn remove_first_queued(
        &self,
        user: &UserId,
        title: &TitleId,
    ) -> Result<(), RepositoryError> {
        self.guard()?;
        let mut state = self.state.lock().unwrap();
        if let Some(index) =
            state.queue.iter().position(|(owner, entry)| owner == user && &entry.title == title)
        {
            state.queue.remove(index);
        }
        Ok(())
    }
}

/// The stored rows carry the kind as a string, and the listings order by it, so an episode
/// sorts before a movie added at the same moment.
fn saved_order(title: &domain::catalog::TitleId) -> (&'static str, &str) {
    match title {
        domain::catalog::TitleId::Episode(id) => ("episode", id.0.as_str()),
        domain::catalog::TitleId::Movie(id) => ("movie", id.0.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};
    use jiff::Timestamp;

    fn user() -> UserId {
        UserId("u1".to_owned())
    }

    fn title() -> TitleId {
        TitleId::Movie(MovieId("m1".to_owned()))
    }

    fn watchlist_item() -> WatchlistItem {
        WatchlistItem { user: user(), title: title(), added_at: Timestamp::UNIX_EPOCH }
    }

    fn favorite() -> Favorite {
        Favorite { user: user(), title: title(), added_at: Timestamp::UNIX_EPOCH }
    }

    fn offset() -> UserSubtitleOffset {
        UserSubtitleOffset {
            user: user(),
            version: VersionId("v1".to_owned()),
            subtitle: SubtitleTrackRef::Embedded(1),
            offset_ms: 500,
        }
    }

    #[tokio::test]
    async fn watchlist_add_dedupe_remove() {
        let repo = MockPreferencesRepo::new();
        assert!(repo.list_watchlist(&user()).await.unwrap().is_empty());
        repo.add_watchlist(watchlist_item()).await.unwrap();
        repo.add_watchlist(watchlist_item()).await.unwrap();
        assert_eq!(repo.list_watchlist(&user()).await.unwrap().len(), 1);
        repo.remove_watchlist(&user(), "m1").await.unwrap();
        assert!(repo.list_watchlist(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn favorites_add_dedupe_remove() {
        let repo = MockPreferencesRepo::new();
        repo.add_favorite(favorite()).await.unwrap();
        repo.add_favorite(favorite()).await.unwrap();
        assert_eq!(repo.list_favorites(&user()).await.unwrap().len(), 1);
        repo.remove_favorite(&user(), "m1").await.unwrap();
        assert!(repo.list_favorites(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn subtitle_offset_set_and_get() {
        let repo = MockPreferencesRepo::new();
        let version = VersionId("v1".to_owned());
        let track = SubtitleTrackRef::Embedded(1);
        assert!(repo.get_subtitle_offset(&user(), &version, &track).await.unwrap().is_none());
        repo.set_subtitle_offset(offset()).await.unwrap();
        let mut updated = offset();
        updated.offset_ms = 750;
        repo.set_subtitle_offset(updated).await.unwrap();
        assert_eq!(
            repo.get_subtitle_offset(&user(), &version, &track).await.unwrap().unwrap().offset_ms,
            750
        );
    }

    #[tokio::test]
    async fn surfaces_backend_failure() {
        let repo = MockPreferencesRepo::new();
        repo.set_fail();
        assert!(repo.list_watchlist(&user()).await.is_err());
        assert!(repo.add_watchlist(watchlist_item()).await.is_err());
        assert!(repo.remove_watchlist(&user(), "m1").await.is_err());
        assert!(repo.list_favorites(&user()).await.is_err());
        assert!(repo.add_favorite(favorite()).await.is_err());
        assert!(repo.remove_favorite(&user(), "m1").await.is_err());
        assert!(
            repo.get_subtitle_offset(
                &user(),
                &VersionId("v1".into()),
                &SubtitleTrackRef::Embedded(1)
            )
            .await
            .is_err()
        );
        assert!(repo.set_subtitle_offset(offset()).await.is_err());

        let playlist = PlaylistId("p1".to_owned());
        let at = Timestamp::UNIX_EPOCH;
        let saved =
            Playlist { id: playlist.clone(), name: "p".into(), created_at: at, updated_at: at };
        assert!(repo.list_playlists(&user()).await.is_err());
        assert!(repo.get_playlist(&user(), &playlist).await.is_err());
        assert!(repo.save_playlist(&user(), saved).await.is_err());
        assert!(repo.delete_playlist(&user(), &playlist).await.is_err());
        assert!(repo.list_playlist_items(&user(), &playlist).await.is_err());
        assert!(repo.add_playlist_items(&user(), &playlist, &[title()], at).await.is_err());
        assert!(repo.remove_playlist_item(&user(), &playlist, "m1", at).await.is_err());
        assert!(repo.move_playlist_item(&user(), &playlist, "m1", 0, at).await.is_err());
        assert!(repo.list_queue(&user()).await.is_err());
        assert!(repo.queue_playlist(&user(), &playlist, false).await.is_err());
        assert!(repo.clear_queue(&user()).await.is_err());
        assert!(repo.remove_queue_entry(&user(), &QueueEntryId("1".into())).await.is_err());
        assert!(repo.remove_first_queued(&user(), &title()).await.is_err());
    }

    #[tokio::test]
    async fn a_seeded_playlist_and_queue_read_back_in_order() {
        let repo = MockPreferencesRepo::new();
        let playlist = PlaylistId("p1".to_owned());
        let at = Timestamp::UNIX_EPOCH;
        let episode = TitleId::Episode(domain::catalog::EpisodeId("e1".to_owned()));
        let saved =
            Playlist { id: playlist.clone(), name: "p".into(), created_at: at, updated_at: at };
        repo.seed_playlist(&user(), saved, &[title(), episode.clone()]);
        repo.seed_queue(&user(), std::slice::from_ref(&episode));

        let items = repo.list_playlist_items(&user(), &playlist).await.unwrap();
        assert_eq!(items.iter().map(|i| i.position).collect::<Vec<_>>(), [0, 1]);
        assert_eq!(items[1].title, episode);
        let queue = repo.list_queue(&user()).await.unwrap();
        assert_eq!(queue, [QueueEntry { id: QueueEntryId("1".into()), title: episode }]);
    }
}
