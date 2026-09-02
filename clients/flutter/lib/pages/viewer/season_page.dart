import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/section_heading.dart';
import 'package:shadowmask/components/facts_row.dart';
import 'package:shadowmask/components/library_toggles.dart';
import 'package:shadowmask/components/overview_text.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/artwork_fallback.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/next_episode.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/model/user_library/watched_rollup.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/view/title_actions.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';

class SeasonPage extends StatelessWidget {
  const SeasonPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    final Map<String, String> q = Uri.base.queryParameters;
    return SectionPage(
      api: api,
      section: NavSection.series,
      errorText: Strings.couldNotLoadSeason,
      loading: const SkeletonPage(child: SkeletonDetail()),
      bodyBuilder: (BuildContext context, SelfUser user) => _SeasonBody(
        api: api,
        user: user,
        id: q['id'] ?? '',
        series: q['series'],
      ),
    );
  }
}

typedef _SeasonData = ({
  Season season,
  List<CatalogCard> cards,
  WatchedRollup rollup,
  Episode? next,
  SeasonNeighbours neighbours,
});

class _SeasonBody extends StatefulWidget {
  const _SeasonBody({
    required this.api,
    required this.user,
    required this.id,
    this.series,
  });

  final ApiClient api;
  final SelfUser user;
  final String id;
  final String? series;

  @override
  State<_SeasonBody> createState() => _SeasonBodyState();
}

class _SeasonBodyState extends State<_SeasonBody> with Mutations<_SeasonBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final TitleRef _ref = TitleRef(type: TitleKind.season, id: widget.id);
  late Future<_SeasonData> _future = _load();

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _delete(String title, String seriesId) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteSeason,
      message: Strings.confirmDeleteTitle(title),
      confirmLabel: Strings.delete,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      () => AdminApi(widget.api).deleteSeason(widget.id),
      successText: Strings.toastDeleted,
      errorText: Strings.errorDelete,
      then: () =>
          Navigator.of(context).pushReplacementNamed(seriesRoute(seriesId)),
    );
  }

  Future<_SeasonData> _load() async {
    final Season season = await _catalog.season(
      widget.id,
      series: widget.series,
    );
    List<Episode> episodes = const <Episode>[];
    try {
      episodes = await _catalog.episodes(widget.id, series: season.seriesId);
    } catch (_) {}
    final List<CatalogCard> cards = episodes
        .map(CatalogCard.fromEpisode)
        .toList();
    await tagWatched(_catalog, widget.user.id, cards, withProgress: true);
    WatchedRollup rollup = WatchedRollup(target: _ref);
    try {
      rollup = await _catalog.rollupOne(widget.user.id, _ref);
    } catch (_) {}
    return (
      season: season,
      cards: cards,
      rollup: rollup,
      next: firstUnwatched(episodes, <String>{
        for (final CatalogCard c in cards)
          if (c.watched) c.ref.id,
      }),
      neighbours: await _neighbours(season),
    );
  }

  Future<SeasonNeighbours> _neighbours(Season season) async {
    try {
      return seasonNeighboursOf(
        season.id,
        await _catalog.seasons(season.seriesId),
      );
    } catch (_) {
      return const SeasonNeighbours();
    }
  }

  VoidCallback? _goTo(SeasonLink? link, String seriesId) => link == null
      ? null
      : () => Navigator.of(
          context,
        ).pushReplacementNamed(seasonRoute(link.id, series: seriesId));

  Artwork? _poster(_SeasonData data) =>
      posterOrParent(data.season.artwork, data.season.seriesArtwork);

  Artwork? _backdrop(_SeasonData data) =>
      backdropOrParent(data.season.artwork, data.season.seriesArtwork);

  @override
  Widget build(BuildContext context) {
    return buildBlock<_SeasonData>(
      future: _future,
      errorText: Strings.couldNotLoadSeason,
      builder: (BuildContext context, _SeasonData data) {
        final Season s = data.season;
        final WatchedRollup rollup = data.rollup;
        final String title = s.title ?? Strings.seasonLabel(s.number);
        final Episode? next = data.next;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.navigationSeries, route: seriesListRoute()),
              Crumb(
                s.seriesTitle ?? Strings.navigationSeries,
                route: seriesRoute(s.seriesId),
              ),
              Crumb(title),
            ]),
            PageBackdrop(
              artwork: _backdrop(data),
              imageBase: _catalog.imageBase,
            ),
            DetailSplit(
              poster: PosterPlay(
                tooltip: Strings.nextEpisode,
                onTap: next == null
                    ? null
                    : () => Navigator.of(context).pushNamed(
                        episodeRoute(next.id, series: s.seriesId, season: s.id),
                      ),
                child: CardArt(
                  artwork: _poster(data),
                  aspect: CardAspect.poster,
                  imageBase: _catalog.imageBase,
                ),
              ),
              info: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  TitleHeading(
                    title: title,
                    pager: <TitleAction>[
                      TitleAction(
                        icon: Icons.chevron_left,
                        tooltip: data.neighbours.previous == null
                            ? Strings.noEarlierSeason
                            : Strings.previousNamed(
                                data.neighbours.previous!.label,
                              ),
                        onPressed: _goTo(data.neighbours.previous, s.seriesId),
                      ),
                      TitleAction(
                        icon: Icons.chevron_right,
                        tooltip: data.neighbours.next == null
                            ? Strings.noLaterSeason
                            : Strings.nextNamed(data.neighbours.next!.label),
                        onPressed: _goTo(data.neighbours.next, s.seriesId),
                      ),
                    ],
                    actions: <TitleAction>[
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.delete_outline,
                          tooltip:
                              deleteBlockForEpisodes(data.cards.length) ??
                              Strings.deleteSeason,
                          danger: true,
                          onPressed: data.cards.isEmpty
                              ? () => _delete(title, s.seriesId)
                              : null,
                        ),
                    ],
                  ),
                  const SizedBox(height: Space.s3),
                  FactsRow(
                    FactsRow.of(<(String, String?)>[
                      (
                        Strings.watchedLabel,
                        rollup.totalEpisodes == 0
                            ? null
                            : Strings.watchedCount(
                                rollup.watchedEpisodes,
                                rollup.totalEpisodes,
                              ),
                      ),
                    ]),
                    labels: false,
                  ),
                  if (s.overview != null) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    OverviewText(s.overview!),
                  ],
                  const SizedBox(height: Space.s4),
                  LibraryToggles(
                    catalog: _catalog,
                    userId: widget.user.id,
                    ref: _ref,
                    title: title,
                    initialWatched: rollup.watched,
                    showWatchlistFavorite: false,
                    episodeCount: rollup.totalEpisodes,
                    onWatchedChanged: (bool _) => _reload(),
                  ),
                ],
              ),
            ),
            const SizedBox(height: Space.s5),
            SectionHeading(
              title: Strings.countLabel(
                Strings.episodesHeading,
                data.cards.length,
              ),
              trailing: data.cards.isEmpty
                  ? null
                  : RandomButton(
                      tooltip: Strings.randomInSeason,
                      pick: () =>
                          _catalog.randomInSeason(s.id, series: s.seriesId),
                    ),
            ),
            const SizedBox(height: Space.s3),
            if (data.cards.isEmpty)
              Text(
                Strings.noEpisodesFound,
                style: TextStyle(color: context.tokens.muted),
              )
            else
              CardGrid(cards: data.cards, imageBase: _catalog.imageBase),
          ],
        );
      },
    );
  }
}
