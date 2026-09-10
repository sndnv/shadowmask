import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/edit_metadata_dialog.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/breadcrumbs.dart';

import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/facts_row.dart';
import 'package:shadowmask/components/library_toggles.dart';
import 'package:shadowmask/components/muted_note.dart';
import 'package:shadowmask/components/overview_text.dart';
import 'package:shadowmask/components/play_button.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/artwork_fallback.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/version_menu.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/version_order.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/view/title_actions.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';

class EpisodePage extends StatelessWidget {
  const EpisodePage({
    super.key,
    required this.api,
    required this.id,
    this.series,
    this.season,
  });

  final ApiClient api;
  final String id;
  final String? series;
  final String? season;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.series,
      errorText: Strings.couldNotLoadEpisode,
      loading: const SkeletonPage(
        child: SkeletonDetail(aspect: CardAspect.landscape),
      ),
      bodyBuilder: (BuildContext context, SelfUser user) => _EpisodeBody(
        api: api,
        user: user,
        id: id,
        series: series,
        season: season,
      ),
    );
  }
}

typedef _EpisodeData = ({
  Episode episode,
  List<Version> versions,
  ItemState state,
  Map<String, int> resumable,
  EpisodeNeighbours neighbours,
});

class _EpisodeBody extends StatefulWidget {
  const _EpisodeBody({
    required this.api,
    required this.user,
    required this.id,
    this.series,
    this.season,
  });

  final ApiClient api;
  final SelfUser user;
  final String id;
  final String? series;
  final String? season;

  @override
  State<_EpisodeBody> createState() => _EpisodeBodyState();
}

class _EpisodeBodyState extends State<_EpisodeBody>
    with Mutations<_EpisodeBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final TitleRef _ref = TitleRef(type: TitleKind.episode, id: widget.id);
  final MenuController _posterMenu = MenuController();
  late Future<_EpisodeData> _future = _load();

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _delete(String title, String seasonId) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteEpisode,
      message: Strings.confirmDeleteTitle(title),
      confirmLabel: Strings.delete,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      () => AdminApi(widget.api).deleteEpisode(widget.id),
      successText: Strings.toastDeleted,
      errorText: Strings.errorDelete,
      then: () => Navigator.of(
        context,
      ).pushReplacementNamed(seasonRoute(seasonId, series: widget.series)),
    );
  }

  Future<void> _edit(Episode episode, String seriesId) async {
    final bool saved = await showEditEpisodeDialog(
      context,
      admin: AdminApi(widget.api),
      episode: episode,
      seriesId: seriesId,
    );
    if (saved && mounted) {
      _reload();
    }
  }

  Future<Map<String, int>> _resumable() async {
    try {
      final ContinueFeed feed = await _catalog.continueFeed(widget.user.id);
      return feed.resumeProgress;
    } catch (_) {
      return const <String, int>{};
    }
  }

  Future<_EpisodeData> _load() async {
    final Future<Map<String, int>> pendingResumable = _resumable();
    final Episode episode = await _catalog.episode(
      widget.id,
      series: widget.series,
      season: widget.season,
    );
    List<Version> versions = const <Version>[];
    try {
      versions = (await _catalog.episodeVersions(
        widget.id,
        series: widget.series,
        season: widget.season,
      )).items;
    } catch (_) {}
    ItemState state = ItemState(title: _ref);
    try {
      state = await _catalog.stateOne(widget.user.id, _ref);
    } catch (_) {}
    return (
      episode: episode,
      versions: versions,
      state: state,
      resumable: await pendingResumable,
      neighbours: await loadNeighbours(_catalog, episode),
    );
  }

  VoidCallback? _goTo(EpisodeLink? link, String? seriesId) => link == null
      ? null
      : () => Navigator.of(context).pushReplacementNamed(
          episodeRoute(link.id, series: seriesId, season: link.seasonId),
        );

  @override
  Widget build(BuildContext context) {
    return buildBlock<_EpisodeData>(
      future: _future,
      errorText: Strings.couldNotLoadEpisode,
      builder: (BuildContext context, _EpisodeData data) {
        final Episode e = data.episode;
        final List<Version> versions = data.versions;
        final ItemState state = data.state;
        final String? seriesId = e.seriesId;
        final int? seasonNumber = e.seasonNumber;
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
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.navigationSeries, route: seriesListRoute()),
              if (seriesId != null)
                Crumb(e.seriesTitle ?? seriesId, route: seriesRoute(seriesId)),
              Crumb(
                e.seasonTitle ??
                    (seasonNumber != null
                        ? Strings.seasonLabel(seasonNumber)
                        : Strings.seasonsHeading),
                route: seasonRoute(e.seasonId, series: seriesId),
              ),
              Crumb(Strings.episodeTitle(e.number, e.title)),
            ]),
            PageBackdrop(
              artwork: backdropOrParent(e.artwork, e.seriesArtwork),
              imageBase: _catalog.imageBase,
            ),
            DetailSplit(
              posterWidth: 320,
              compactPosterWidth: double.infinity,
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
                    artwork: backdropOrParent(e.artwork, e.seriesArtwork),
                    aspect: CardAspect.landscape,
                    imageBase: _catalog.imageBase,
                  ),
                ),
              ),
              headline: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  TitleHeading(
                    title: Strings.episodeTitle(e.number, e.title),
                    pager: <TitleAction>[
                      TitleAction(
                        icon: Icons.chevron_left,
                        tooltip: data.neighbours.previous == null
                            ? Strings.noEarlierEpisode
                            : Strings.previousNamed(
                                data.neighbours.previous!.label,
                              ),
                        onPressed: _goTo(data.neighbours.previous, seriesId),
                      ),
                      TitleAction(
                        icon: Icons.chevron_right,
                        tooltip: data.neighbours.next == null
                            ? Strings.noLaterEpisode
                            : Strings.nextNamed(data.neighbours.next!.label),
                        onPressed: _goTo(data.neighbours.next, seriesId),
                      ),
                    ],
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
                      if (widget.user.isAdmin && seriesId != null)
                        TitleAction(
                          icon: Icons.edit,
                          tooltip: Strings.editDetails,
                          onPressed: () => _edit(e, seriesId),
                        ),
                      if (widget.user.isAdmin)
                        TitleAction(
                          icon: Icons.delete_outline,
                          tooltip:
                              deleteBlockForVersions(versions.length) ??
                              Strings.deleteEpisode,
                          danger: true,
                          onPressed: versions.isEmpty
                              ? () => _delete(e.title, e.seasonId)
                              : null,
                        ),
                    ],
                  ),
                ],
              ),
              actions: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  LibraryToggles(
                    catalog: _catalog,
                    userId: widget.user.id,
                    ref: _ref,
                    title: e.title,
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
              info: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  FactsRow(
                    FactsRow.of(<(String, String?)>[
                      (Strings.factAirDate, dateText(e.airDate)),
                      (
                        Strings.factRuntime,
                        e.runtimeMinutes != null
                            ? runtime(e.runtimeMinutes!)
                            : null,
                      ),
                      (Strings.factQuality, best?.quality.label),
                    ]),
                    labels: false,
                  ),
                  if (e.overview != null) ...<Widget>[
                    const SizedBox(height: Space.s3),
                    OverviewText(e.overview!),
                  ],
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}
