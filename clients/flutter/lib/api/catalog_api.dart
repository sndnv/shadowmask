import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/model/catalog/collection.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/discovery/hub.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/model/user_library/watched_rollup.dart';
import 'package:shadowmask/model/user_library/watchlist_item.dart';
import 'package:shadowmask/model/user_library/favorite.dart';
import 'package:shadowmask/model/user_library/watch_history.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/model/catalog/random_pick.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/catalog/series.dart';
import 'package:shadowmask/model/catalog/series_detail.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/api/api_client.dart';

class CatalogApi {
  CatalogApi(this._api);

  final ApiClient _api;

  String get imageBase => _api.baseUrl;

  Future<Paged<Movie>> movies({
    int offset = 0,
    int? limit,
    String? sort,
    String? order,
    List<String> genres = const <String>[],
    String? library,
  }) => _api.getPage(
    withQuery(
      '/api/v1/movies',
      _listParams(offset, limit, sort, order, genres, library),
    ),
    Movie.fromJson,
  );

  Future<Paged<Series>> series({
    int offset = 0,
    int? limit,
    String? sort,
    String? order,
    List<String> genres = const <String>[],
    String? library,
  }) => _api.getPage(
    withQuery(
      '/api/v1/series',
      _listParams(offset, limit, sort, order, genres, library),
    ),
    Series.fromJson,
  );

  Future<List<Genre>> genres({String? kind}) => _api.getJsonArray(
    withQuery('/api/v1/genres', <String, String?>{'kind': kind}),
    Genre.fromJson,
  );

  Future<RandomPick> randomMovie({
    List<String> genres = const <String>[],
    String? library,
  }) => _api.getJson(
    withQuery('/api/v1/movies/random', _filterParams(genres, library)),
    RandomPick.fromJson,
  );

  Future<RandomPick> randomEpisode({
    List<String> genres = const <String>[],
    String? library,
  }) => _api.getJson(
    withQuery('/api/v1/series/random', _filterParams(genres, library)),
    RandomPick.fromJson,
  );

  Future<RandomPick> randomInCollection(String id) => _api.getJson(
    '/api/v1/movies/collections/${_enc(id)}/random',
    RandomPick.fromJson,
  );

  Future<RandomPick> randomInSeries(String id) =>
      _api.getJson('/api/v1/series/${_enc(id)}/random', RandomPick.fromJson);

  Future<RandomPick> randomInSeason(
    String seasonId, {
    String? series,
  }) => _api.getJson(
    '/api/v1/series/${_enc(series ?? '-')}/seasons/${_enc(seasonId)}/random',
    RandomPick.fromJson,
  );

  Future<Paged<Collection>> collections({int offset = 0, int? limit}) =>
      _api.getPage(
        withQuery('/api/v1/movies/collections', _pageParams(offset, limit)),
        Collection.fromJson,
      );

  Future<Collection> collection(String id) => _api.getJson(
    '/api/v1/movies/collections/${_enc(id)}',
    Collection.fromJson,
  );

  Future<List<Collection>> movieCollections(String id) => _api.getJsonArray(
    '/api/v1/movies/${_enc(id)}/collections',
    Collection.fromJson,
  );

  Future<void> createCollection(Map<String, dynamic> body) =>
      _api.sendVoid('POST', '/api/v1/movies/collections', body: body);

  Future<void> updateCollection(String id, Map<String, dynamic> body) => _api
      .sendVoid('PUT', '/api/v1/movies/collections/${_enc(id)}', body: body);

  Future<void> deleteCollection(String id) =>
      _api.sendVoid('DELETE', '/api/v1/movies/collections/${_enc(id)}');

  Future<MovieDetail> movie(String id) =>
      _api.getJson('/api/v1/movies/${_enc(id)}', MovieDetail.fromJson);

  Future<SeriesDetail> seriesDetail(String id) =>
      _api.getJson('/api/v1/series/${_enc(id)}', SeriesDetail.fromJson);

  Future<List<Season>> seasons(String seriesId) => _api.getJsonArray(
    '/api/v1/series/${_enc(seriesId)}/seasons',
    Season.fromJson,
  );

  Future<Season> season(String id, {String? series}) => _api.getJson(
    '/api/v1/series/${_enc(series ?? '-')}/seasons/${_enc(id)}',
    Season.fromJson,
  );

  Future<List<Episode>> episodes(
    String seasonId, {
    String? series,
  }) => _api.getJsonArray(
    '/api/v1/series/${_enc(series ?? '-')}/seasons/${_enc(seasonId)}/episodes',
    Episode.fromJson,
  );

  Future<Episode> episode(String id, {String? series, String? season}) =>
      _api.getJson(_episodePath(id, series, season), Episode.fromJson);

  Future<Paged<Version>> movieVersions(String id) =>
      _api.getPage('/api/v1/movies/${_enc(id)}/versions', Version.fromJson);

  Future<Paged<Version>> episodeVersions(
    String id, {
    String? series,
    String? season,
  }) => _api.getPage(
    '${_episodePath(id, series, season)}/versions',
    Version.fromJson,
  );

  Future<PersonProfile> person(String id) =>
      _api.getJson('/api/v1/people/${_enc(id)}', PersonProfile.fromJson);

  Future<VersionDetail> version(String id) =>
      _api.getJson('/api/v1/versions/${_enc(id)}', VersionDetail.fromJson);

  Future<Paged<CatalogCard>> search({
    required String q,
    String? type,
    int offset = 0,
    int? limit,
  }) => _api.getPage(
    withQuery('/api/v1/search', <String, String?>{
      'q': q,
      'type': type,
      ..._pageParams(offset, limit),
    }),
    CatalogCard.fromJson,
  );

  Future<List<ItemState>> stateBatch(String userId, List<TitleRef> refs) =>
      _api.postJsonArray(
        '/api/v1/users/${_enc(userId)}/state/batch',
        <String, dynamic>{
          'titles': refs.map((TitleRef r) => r.toJson()).toList(),
        },
        ItemState.fromJson,
      );

  Future<List<WatchedRollup>> stateRollup(
    String userId,
    List<TitleRef> targets,
  ) => _api.postJsonArray(
    '/api/v1/users/${_enc(userId)}/state/rollup',
    <String, dynamic>{
      'targets': targets.map((TitleRef r) => r.toJson()).toList(),
    },
    WatchedRollup.fromJson,
  );

  Future<ItemState> stateOne(String userId, TitleRef ref) async {
    final List<ItemState> states = await stateBatch(userId, <TitleRef>[ref]);
    return states.isNotEmpty ? states.first : ItemState(title: ref);
  }

  Future<WatchedRollup> rollupOne(String userId, TitleRef target) async {
    final List<WatchedRollup> rollups = await stateRollup(userId, <TitleRef>[
      target,
    ]);
    return rollups.isNotEmpty ? rollups.first : WatchedRollup(target: target);
  }

  Future<void> setWatched(String userId, TitleRef ref, bool watched) =>
      _api.sendVoid(
        'PUT',
        '/api/v1/users/${_enc(userId)}/watched/${_enc(ref.id)}',
        body: <String, dynamic>{'type': ref.type.wire, 'watched': watched},
      );

  Future<void> addToWatchlist(String userId, TitleRef ref) => _api.sendVoid(
    'PUT',
    '/api/v1/users/${_enc(userId)}/watchlist/${_enc(ref.id)}',
    body: <String, dynamic>{'type': ref.type.wire},
  );

  Future<void> removeFromWatchlist(String userId, TitleRef ref) =>
      _api.sendVoid(
        'DELETE',
        '/api/v1/users/${_enc(userId)}/watchlist/${_enc(ref.id)}',
      );

  Future<void> addFavorite(String userId, TitleRef ref) => _api.sendVoid(
    'PUT',
    '/api/v1/users/${_enc(userId)}/favorites/${_enc(ref.id)}',
    body: <String, dynamic>{'type': ref.type.wire},
  );

  Future<void> removeFavorite(String userId, TitleRef ref) => _api.sendVoid(
    'DELETE',
    '/api/v1/users/${_enc(userId)}/favorites/${_enc(ref.id)}',
  );

  Future<List<CatalogCard>> titleCards(
    List<TitleRef> refs, {
    bool asSeriesPoster = false,
  }) => _api.postJsonArray(
    '/api/v1/titles/batch',
    <String, dynamic>{'titles': refs.map((TitleRef r) => r.toJson()).toList()},
    (Map<String, dynamic> json) =>
        CatalogCard.fromJson(json, asSeriesPoster: asSeriesPoster),
  );

  Future<List<CatalogCard>> peopleCards(List<String> ids) => _api.postJsonArray(
    '/api/v1/people/batch',
    <String, dynamic>{'people': ids},
    (Map<String, dynamic> json) =>
        CatalogCard.fromJson(<String, dynamic>{...json, 'type': 'person'}),
  );

  Future<List<WatchlistItem>> watchlist(String userId) => _api.getJsonArray(
    '/api/v1/users/${_enc(userId)}/watchlist',
    WatchlistItem.fromJson,
  );

  Future<List<Favorite>> favorites(String userId) => _api.getJsonArray(
    '/api/v1/users/${_enc(userId)}/favorites',
    Favorite.fromJson,
  );

  Future<Paged<WatchHistory>> history(
    String userId, {
    int offset = 0,
    int? limit,
  }) => _api.getPage(
    withQuery(
      '/api/v1/users/${_enc(userId)}/history',
      _pageParams(offset, limit),
    ),
    WatchHistory.fromJson,
  );

  Future<void> removeFromHistory(String userId, String titleId) =>
      _api.sendVoid(
        'DELETE',
        '/api/v1/users/${_enc(userId)}/history/${_enc(titleId)}',
      );

  Future<void> clearHistory(String userId) =>
      _api.sendVoid('DELETE', '/api/v1/users/${_enc(userId)}/history');

  Future<AccountProfile> user(String userId) =>
      _api.getJson('/api/v1/users/${_enc(userId)}', AccountProfile.fromJson);

  Future<void> updateProfile(String userId, Map<String, dynamic> body) =>
      _api.sendVoid('PUT', '/api/v1/users/${_enc(userId)}', body: body);

  Future<List<Hub>> hub(String userId) =>
      _api.getJsonArray('/api/v1/users/${_enc(userId)}/hub', Hub.fromJson);

  Future<ContinueFeed> continueFeed(String userId) => _api.getJson(
    '/api/v1/users/${_enc(userId)}/continue',
    ContinueFeed.fromJson,
  );

  String _episodePath(String id, String? series, String? season) =>
      '/api/v1/series/${_enc(series ?? '-')}/seasons/${_enc(season ?? '-')}'
      '/episodes/${_enc(id)}';

  Map<String, String?> _pageParams(int offset, int? limit) => <String, String?>{
    if (offset > 0) 'offset': offset.toString(),
    if (limit != null) 'limit': limit.toString(),
  };

  Map<String, String?> _filterParams(List<String> genres, String? library) =>
      <String, String?>{
        'genres': genres.isEmpty ? null : genres.join(','),
        'library': library,
      };

  Map<String, String?> _listParams(
    int offset,
    int? limit,
    String? sort,
    String? order,
    List<String> genres,
    String? library,
  ) => <String, String?>{
    ..._pageParams(offset, limit),
    'sort': sort,
    'order': order,
    'genres': genres.isEmpty ? null : genres.join(','),
    'library': library,
  };

  String _enc(String s) => Uri.encodeComponent(s);
}
