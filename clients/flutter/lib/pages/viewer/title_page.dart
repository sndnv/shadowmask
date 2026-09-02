import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/facts_row.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/cast_rail.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/genre_chips.dart';
import 'package:shadowmask/components/overview_text.dart';
import 'package:shadowmask/components/library_toggles.dart';
import 'package:shadowmask/components/muted_note.dart';
import 'package:shadowmask/components/load_more.dart';
import 'package:shadowmask/components/play_button.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/rating_chips.dart';
import 'package:shadowmask/components/section_heading.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/edit_metadata_dialog.dart';
import 'package:shadowmask/components/admin/refresh_metadata_dialog.dart';
import 'package:shadowmask/components/admin/relink_dialog.dart';
import 'package:shadowmask/components/admin/series_relink_dialog.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/model/catalog/collection.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/model/user_library/watched_rollup.dart';
import 'package:shadowmask/model/catalog/series_detail.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/next_episode.dart';
import 'package:shadowmask/view/cast_order.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/title_actions.dart';
import 'package:shadowmask/view/version_order.dart';
import 'package:shadowmask/pages/default/lazy_block.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/pages/default/section_page.dart';

class TitlePage extends StatelessWidget {
  const TitlePage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    final Map<String, String> q = Uri.base.queryParameters;
    final String type = q['type'] ?? 'movie';
    final String id = q['id'] ?? '';
    final bool isSeries = type == 'series';
    return SectionPage(
      api: api,
      section: isSeries ? NavSection.series : NavSection.movies,
      errorText: Strings.couldNotLoadTitle,
      loading: const SkeletonPage(child: SkeletonDetail()),
      bodyBuilder: (BuildContext context, SelfUser user) => isSeries
          ? _SeriesDetailBody(api: api, id: id, user: user)
          : _MovieDetailBody(api: api, id: id, user: user),
    );
  }
}

class _MovieDetailBody extends StatefulWidget {
  const _MovieDetailBody({
    required this.api,
    required this.id,
    required this.user,
  });

  final ApiClient api;
  final String id;
  final SelfUser user;

  @override
  State<_MovieDetailBody> createState() => _MovieDetailBodyState();
}

typedef _MovieData = ({
  MovieDetail detail,
  List<Version> versions,
  ItemState state,
  Map<String, int> resumable,
});

class _MovieDetailBodyState extends State<_MovieDetailBody>
    with Mutations<_MovieDetailBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final TitleRef _ref = TitleRef(type: TitleKind.movie, id: widget.id);
  final MenuController _posterMenu = MenuController();
  late Future<_MovieData> _future = _load();

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _relink(List<Version> versions) async {
    final bool queued = await showRelinkDialog(
      context,
      admin: AdminApi(widget.api),
      catalog: _catalog,
      versionIds: versions.map((Version v) => v.id).toList(),
    );
    if (queued && mounted) {
      setState(() {
        _future = _load();
      });
    }
  }

  Future<void> _refresh(bool manuallyEdited) async {
    final bool queued = await showRefreshMetadataDialog(
      context,
      admin: AdminApi(widget.api),
      kind: TitleKind.movie,
      id: widget.id,
      manuallyEdited: manuallyEdited,
    );
    if (queued && manuallyEdited && mounted) {
      _reload();
    }
  }

  Future<void> _delete(String title) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteMovie,
      message: Strings.confirmDeleteTitle(title),
      confirmLabel: Strings.delete,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      () => AdminApi(widget.api).deleteMovie(widget.id),
      successText: Strings.toastDeleted,
      errorText: Strings.errorDelete,
      then: () => Navigator.of(context).pushReplacementNamed(moviesRoute()),
    );
  }

  Future<void> _edit(MovieDetail detail) async {
    final List<RatingSystem> systems = await PlaybackApi(widget.api)
        .serverInfo()
        .then((ServerInfo info) => info.ratingSystems)
        .catchError((Object _) => const <RatingSystem>[]);
    if (!mounted) {
      return;
    }
    final bool saved = await showEditMovieDialog(
      context,
      admin: AdminApi(widget.api),
      movie: detail,
      ratingSystems: systems,
    );
    if (saved && mounted) {
      _reload();
    }
  }

  Future<_MovieData> _load() async {
    final Future<Paged<Version>> pendingVersions = _catalog.movieVersions(
      widget.id,
    );
    final Future<ItemState> pendingState = _catalog.stateOne(
      widget.user.id,
      _ref,
    );
    final Future<ContinueFeed> pendingFeed = _catalog.continueFeed(
      widget.user.id,
    );
    final MovieDetail detail = await _catalog.movie(widget.id);
    List<Version> versions = const <Version>[];
    try {
      versions = (await pendingVersions).items;
    } catch (_) {}
    ItemState state = ItemState(title: _ref);
    try {
      state = await pendingState;
    } catch (_) {}
    Map<String, int> resumable = const <String, int>{};
    try {
      resumable = (await pendingFeed).resumeProgress;
    } catch (_) {}
    return (
      detail: detail,
      versions: versions,
      state: state,
      resumable: resumable,
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<_MovieData>(
      future: _future,
      errorText: Strings.couldNotLoadTitle,
      builder: (BuildContext context, _MovieData data) {
        final MovieDetail d = data.detail;
        final List<Version> versions = data.versions;
        final ItemState state = data.state;
        final List<Version> ordered = orderedVersions(versions);
        final List<Version> available = ordered
            .where((Version v) => v.available)
            .toList();
        final Version? best = available.isNotEmpty
            ? available.first
            : (versions.isNotEmpty ? versions.first : null);
        final Version? shortcut = resolvePlayTarget(
          available,
          data.resumable.keys.toSet(),
        );
        final int? resumeAt = shortcut == null
            ? null
            : data.resumable[shortcut.id];
        final List<PersonRef> actors = billedActors(d.credits);
        final String? relinkBlock = relinkBlockForVersions(versions);
        final String? deleteBlock = deleteBlockForVersions(versions.length);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.navigationMovies, route: moviesRoute()),
              Crumb(d.title),
            ]),
            PageBackdrop(artwork: d.artwork, imageBase: _catalog.imageBase),
            DetailSplit(
              poster: VersionMenu(
                controller: _posterMenu,
                ordered: ordered,
                onPlay: (Version v) =>
                    Navigator.of(context).pushNamed(watchRoute(v.id)),
                child: PosterPlay(
                  progressPercent: resumeAt,
                  tooltip: shortcut == null
                      ? Strings.chooseVersion
                      : (resumeAt != null
                            ? Strings.resume(resumeAt)
                            : Strings.play),
                  icon: shortcut == null
                      ? Icons.playlist_play
                      : Icons.play_arrow,
                  onTap: available.isEmpty
                      ? null
                      : (shortcut == null
                            ? () => VersionMenu.toggle(_posterMenu)
                            : () => Navigator.of(
                                context,
                              ).pushNamed(watchRoute(shortcut.id))),
                  child: CardArt(
                    artwork: d.artwork,
                    aspect: CardAspect.poster,
                    imageBase: _catalog.imageBase,
                  ),
                ),
              ),
              info: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  TitleHeading(
                    title: d.title,
                    actions: <TitleAction>[
                      if (versions.isNotEmpty)
                        TitleAction(
                          icon: Icons.layers,
                          tooltip: Strings.versionsHeading,
                          onPressed: () => AllVersionsDialog.show(
                            context,
                            catalog: _catalog,
                            playback: PlaybackApi(widget.api),
                            userId: widget.user.id,
                            versions: ordered,
                            onProgressCleared: _reload,
                          ),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.refresh,
                          tooltip: Strings.refreshMetadata,
                          onPressed: () => _refresh(d.manuallyEdited),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.edit,
                          tooltip: Strings.editDetails,
                          onPressed: () => _edit(d),
                        ),
                      if (widget.user.isAdmin && versions.isNotEmpty)
                        TitleAction(
                          icon: Icons.link,
                          tooltip: relinkBlock ?? Strings.relink,
                          onPressed: relinkBlock != null
                              ? null
                              : () => _relink(versions),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.delete_outline,
                          tooltip: deleteBlock ?? Strings.deleteMovie,
                          danger: true,
                          onPressed: deleteBlock != null
                              ? null
                              : () => _delete(d.title),
                        ),
                    ],
                  ),
                  const SizedBox(height: Space.s3),
                  FactsRow(
                    FactsRow.of(<(String, String?)>[
                      (Strings.factYear, d.year?.toString()),
                      (
                        Strings.factRuntime,
                        d.runtimeMinutes != null
                            ? runtime(d.runtimeMinutes!)
                            : null,
                      ),
                      (Strings.factRating, d.contentRating?.label),
                      (Strings.factQuality, best?.quality.label),
                    ]),
                    labels: false,
                  ),
                  if (d.genres.isNotEmpty) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    GenreChips(d.genres, basePath: moviesRoute()),
                  ],
                  if (d.ratings.isNotEmpty) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    RatingChips(d.ratings),
                  ],
                  if (d.overview != null) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    OverviewText(d.overview!),
                  ],
                  const SizedBox(height: Space.s4),
                  LibraryToggles(
                    catalog: _catalog,
                    userId: widget.user.id,
                    ref: _ref,
                    title: d.title,
                    initialWatched: state.watched,
                    initialWatchlisted: state.watchlisted,
                    initialFavorite: state.favorite,
                    onWatchedChanged: (bool _) => _reload(),
                    leading: PlayButton(
                      catalog: _catalog,
                      userId: widget.user.id,
                      versions: versions,
                      resumable: data.resumable,
                      playback: PlaybackApi(widget.api),
                      onProgressCleared: _reload,
                    ),
                  ),
                  if (available.isEmpty) ...<Widget>[
                    const SizedBox(height: Space.s2),
                    const MutedNote(Strings.noVersionsAvailable),
                  ],
                ],
              ),
            ),
            if (d.credits.isNotEmpty) ...<Widget>[
              const SizedBox(height: Space.s5),
              CastRail(catalog: _catalog, credits: d.credits),
            ],
            _RelatedRails(
              catalog: _catalog,
              userId: widget.user.id,
              actors: actors,
              exclude: TitleRef(type: TitleKind.movie, id: d.id),
              movieId: d.id,
            ),
          ],
        );
      },
    );
  }
}

typedef _CollectionTitles = ({String id, String name, List<CatalogCard> cards});

List<_CollectionTitles> _otherMovies(
  List<Collection> collections,
  String movieId,
) {
  final List<_CollectionTitles> rails = <_CollectionTitles>[];
  for (final Collection c in collections) {
    final List<CatalogCard> cards = c.items
        .where((Movie m) => m.id != movieId)
        .map(CatalogCard.fromMovie)
        .toList();
    if (cards.isNotEmpty) {
      rails.add((id: c.id, name: c.name, cards: cards));
    }
  }
  return rails;
}

const int kMinActorTitles = 3;
const int kMaxActorProbes = 5;

typedef _RelatedRail = ({
  String prefix,
  String link,
  String route,
  List<CatalogCard> cards,
});

class _RelatedRails extends StatefulWidget {
  const _RelatedRails({
    required this.catalog,
    required this.userId,
    required this.actors,
    required this.exclude,
    this.movieId,
  });

  final CatalogApi catalog;
  final String userId;
  final List<PersonRef> actors;
  final TitleRef exclude;
  final String? movieId;

  @override
  State<_RelatedRails> createState() => _RelatedRailsState();
}

class _RelatedRailsState extends State<_RelatedRails> {
  final List<_RelatedRail> _rails = <_RelatedRail>[];
  final Set<TitleRef> _shown = <TitleRef>{};
  int _probe = 0;
  bool _busy = false;
  bool _answered = false;
  Object? _lastError;

  bool get _exhausted => _probe >= widget.actors.length;

  Future<List<_RelatedRail>> _first() async {
    await _loadCollections();
    await _loadActor();
    final Object? error = _lastError;
    if (_rails.isEmpty && !_answered && error != null) {
      throw error;
    }
    return _rails;
  }

  Future<void> _loadCollections() async {
    final String? movieId = widget.movieId;
    if (movieId == null) {
      return;
    }
    final List<_CollectionTitles> found = _otherMovies(
      await widget.catalog.movieCollections(movieId),
      movieId,
    );
    for (final _CollectionTitles rail in found) {
      await tagWatched(widget.catalog, widget.userId, rail.cards);
      _shown.addAll(rail.cards.map((CatalogCard card) => card.ref));
      _rails.add((
        prefix: Strings.moreInPrefix,
        link: rail.name,
        route: collectionRoute(rail.id),
        cards: rail.cards,
      ));
    }
  }

  Future<void> _loadActor() async {
    int probes = 0;
    while (!_exhausted && probes < kMaxActorProbes) {
      final PersonRef actor = widget.actors[_probe++];
      probes++;
      final List<CatalogCard> cards;
      try {
        cards = await _titlesOf(actor);
        _answered = true;
      } catch (error) {
        _lastError = error;
        continue;
      }
      final Set<TitleRef> taken = <TitleRef>{};
      final List<CatalogCard> fresh = <CatalogCard>[];
      for (final CatalogCard card in cards) {
        if (!_shown.contains(card.ref) && taken.add(card.ref)) {
          fresh.add(card);
        }
      }
      if (fresh.length < kMinActorTitles) {
        continue;
      }
      await tagWatched(widget.catalog, widget.userId, fresh);
      _shown.addAll(taken);
      _rails.add((
        prefix: Strings.moreWithPrefix,
        link: actor.name,
        route: personRoute(actor.id),
        cards: fresh,
      ));
      return;
    }
  }

  Future<List<CatalogCard>> _titlesOf(PersonRef actor) async {
    final PersonProfile person = await widget.catalog.person(actor.id);
    return person.filmography
        .where(
          (FilmographyEntry e) =>
              e.titleId != widget.exclude.id || e.kind != widget.exclude.type,
        )
        .map(CatalogCard.fromFilmography)
        .toList();
  }

  void _more() {
    if (_busy || _exhausted) {
      return;
    }
    _busy = true;
    _loadActor().whenComplete(() {
      _busy = false;
      if (mounted) {
        setState(() {});
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return LazyBlock<List<_RelatedRail>>(
      load: _first,
      loading: const Padding(
        padding: EdgeInsets.only(top: Space.s5),
        child: SkeletonRail(),
      ),
      errorText: Strings.couldNotLoadRelated,
      builder: (BuildContext context, List<_RelatedRail> _) => LoadMore(
        enabled: !_exhausted,
        onLoad: _more,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            for (final _RelatedRail rail in _rails)
              Padding(
                padding: const EdgeInsets.only(top: Space.s5),
                child: CardRail(
                  title: rail.prefix,
                  titleLink: rail.link,
                  titleRoute: rail.route,
                  cards: rail.cards,
                  imageBase: widget.catalog.imageBase,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _SeriesDetailBody extends StatefulWidget {
  const _SeriesDetailBody({
    required this.api,
    required this.id,
    required this.user,
  });

  final ApiClient api;
  final String id;
  final SelfUser user;

  @override
  State<_SeriesDetailBody> createState() => _SeriesDetailBodyState();
}

typedef _SeriesData = ({
  SeriesDetail detail,
  List<Season> seasons,
  List<CatalogCard> cards,
  WatchedRollup rollup,
});

class _SeriesDetailBodyState extends State<_SeriesDetailBody>
    with Mutations<_SeriesDetailBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final TitleRef _ref = TitleRef(type: TitleKind.series, id: widget.id);
  late Future<_SeriesData> _future = _load();

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _refresh(bool manuallyEdited) async {
    final bool queued = await showRefreshMetadataDialog(
      context,
      admin: AdminApi(widget.api),
      kind: TitleKind.series,
      id: widget.id,
      manuallyEdited: manuallyEdited,
    );
    if (queued && manuallyEdited && mounted) {
      _reload();
    }
  }

  Future<void> _edit(SeriesDetail detail) async {
    final List<RatingSystem> systems = await PlaybackApi(widget.api)
        .serverInfo()
        .then((ServerInfo info) => info.ratingSystems)
        .catchError((Object _) => const <RatingSystem>[]);
    if (!mounted) {
      return;
    }
    final bool saved = await showEditSeriesDialog(
      context,
      admin: AdminApi(widget.api),
      series: detail,
      ratingSystems: systems,
    );
    if (saved && mounted) {
      _reload();
    }
  }

  Future<void> _relink(String title) async {
    final bool queued = await showSeriesRelinkDialog(
      context,
      admin: AdminApi(widget.api),
      seriesId: widget.id,
      title: title,
    );
    if (queued && mounted) {
      _reload();
    }
  }

  Future<void> _delete(String title) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteSeries,
      message: Strings.confirmDeleteTitle(title),
      confirmLabel: Strings.delete,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      () => AdminApi(widget.api).deleteSeries(widget.id),
      successText: Strings.toastDeleted,
      errorText: Strings.errorDelete,
      then: () => Navigator.of(context).pushReplacementNamed(seriesListRoute()),
    );
  }

  String? _relinkBlock(SeriesDetail d) =>
      relinkBlockForSeries(d.episodesTotal, d.episodesWithAvailableVersion);

  Future<_SeriesData> _load() async {
    final Future<List<Season>> pendingSeasons = _catalog
        .seasons(widget.id)
        .catchError((Object _) => <Season>[]);
    final Future<WatchedRollup> pendingRollup = _catalog.rollupOne(
      widget.user.id,
      _ref,
    );
    final SeriesDetail detail = await _catalog.seriesDetail(widget.id);
    final List<Season> seasons = await pendingSeasons;
    final List<CatalogCard> seasonCards = seasons
        .map(CatalogCard.fromSeason)
        .toList();
    WatchedRollup rollup = WatchedRollup(target: _ref);
    try {
      rollup = await pendingRollup;
    } catch (_) {}
    await tagWatched(_catalog, widget.user.id, seasonCards);
    return (
      detail: detail,
      seasons: seasons,
      cards: seasonCards,
      rollup: rollup,
    );
  }

  Future<void> _openNext(_SeriesData data) async {
    final Episode? next = await resolveNextEpisode(
      _catalog,
      widget.user.id,
      widget.id,
      data.seasons,
      <String>{
        for (final CatalogCard c in data.cards)
          if (c.watched) c.ref.id,
      },
    );
    if (!mounted || next == null) {
      return;
    }
    Navigator.of(context).pushNamed(
      episodeRoute(next.id, series: widget.id, season: next.seasonId),
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<_SeriesData>(
      future: _future,
      errorText: Strings.couldNotLoadTitle,
      builder: (BuildContext context, _SeriesData data) {
        final SeriesDetail d = data.detail;
        final WatchedRollup rollup = data.rollup;
        final List<CatalogCard> seasonCards = data.cards;
        final List<PersonRef> actors = billedActors(d.credits);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.navigationSeries, route: seriesListRoute()),
              Crumb(d.title),
            ]),
            PageBackdrop(artwork: d.artwork, imageBase: _catalog.imageBase),
            DetailSplit(
              poster: PosterPlay(
                tooltip: Strings.nextEpisode,
                onTap: data.seasons.isEmpty ? null : () => _openNext(data),
                child: CardArt(
                  artwork: d.artwork,
                  aspect: CardAspect.poster,
                  imageBase: _catalog.imageBase,
                ),
              ),
              info: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  TitleHeading(
                    title: d.title,
                    actions: <TitleAction>[
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.refresh,
                          tooltip: Strings.refreshMetadata,
                          onPressed: () => _refresh(d.manuallyEdited),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.edit,
                          tooltip: Strings.editDetails,
                          onPressed: () => _edit(d),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.link,
                          tooltip: _relinkBlock(d) ?? Strings.relinkSeries,
                          onPressed: _relinkBlock(d) != null
                              ? null
                              : () => _relink(d.title),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.delete_outline,
                          tooltip:
                              deleteBlockForSeasons(data.seasons.length) ??
                              Strings.deleteSeries,
                          danger: true,
                          onPressed: data.seasons.isEmpty
                              ? () => _delete(d.title)
                              : null,
                        ),
                    ],
                  ),
                  const SizedBox(height: Space.s3),
                  FactsRow(
                    FactsRow.of(<(String, String?)>[
                      (Strings.factYear, d.year?.toString()),
                      (Strings.factRating, d.contentRating?.label),
                    ]),
                    labels: false,
                  ),
                  if (d.genres.isNotEmpty) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    GenreChips(d.genres, basePath: seriesListRoute()),
                  ],
                  if (d.ratings.isNotEmpty) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    RatingChips(d.ratings),
                  ],
                  if (d.overview != null) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    OverviewText(d.overview!),
                  ],
                  const SizedBox(height: Space.s4),
                  LibraryToggles(
                    catalog: _catalog,
                    userId: widget.user.id,
                    ref: _ref,
                    title: d.title,
                    initialWatched: rollup.watched,
                    showWatchlistFavorite: false,
                    episodeCount: rollup.totalEpisodes,
                    onWatchedChanged: (bool _) => _reload(),
                  ),
                ],
              ),
            ),
            if (d.credits.isNotEmpty) ...<Widget>[
              const SizedBox(height: Space.s5),
              CastRail(catalog: _catalog, credits: d.credits),
            ],
            const SizedBox(height: Space.s5),
            SectionHeading(
              title: Strings.countLabel(
                Strings.seasonsHeading,
                seasonCards.length,
              ),
              trailing: data.seasons.isEmpty
                  ? null
                  : RandomButton(
                      tooltip: Strings.randomInSeries,
                      pick: () => _catalog.randomInSeries(d.id),
                    ),
            ),
            const SizedBox(height: Space.s3),
            if (seasonCards.isEmpty)
              Text(
                Strings.noSeasonsFound,
                style: TextStyle(color: context.tokens.muted),
              )
            else
              CardGrid(cards: seasonCards, imageBase: _catalog.imageBase),
            if (actors.isNotEmpty)
              _RelatedRails(
                catalog: _catalog,
                userId: widget.user.id,
                actors: actors,
                exclude: TitleRef(type: TitleKind.series, id: d.id),
              ),
          ],
        );
      },
    );
  }
}
