import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/item_state.dart';

typedef NextEpisode = ({Episode episode, int season, int progressPercent});

String nextEpisodeAction(NextEpisode? next) =>
    next != null && next.progressPercent > 0
    ? Strings.resumeAction
    : Strings.play;

String nextEpisodeName(NextEpisode next, {required bool withSeason}) =>
    Strings.episodeLine(
      withSeason ? next.season : null,
      next.episode.number,
      next.episode.title,
    );

String nextEpisodeCaption(NextEpisode next, {required bool withSeason}) =>
    Strings.upNextNote(nextEpisodeName(next, withSeason: withSeason));

String nextEpisodeSpoken(NextEpisode next, {required bool withSeason}) {
  final String? title = Strings.ownEpisodeTitle(
    next.episode.number,
    next.episode.title,
  );
  return Strings.upNextNote(
    <String>[
      if (withSeason) Strings.seasonLabel(next.season),
      '${Strings.episodeLabel} ${next.episode.number}',
      ?title,
    ].join(', '),
  );
}

Season? nextSeason(List<Season> seasons, Set<String> watchedSeasonIds) {
  final List<Season> regular = seasons
      .where((Season s) => s.number > 0)
      .toList();
  final List<Season> ordered = List<Season>.of(
    regular.isEmpty ? seasons : regular,
  )..sort((Season a, Season b) => a.number.compareTo(b.number));
  for (final Season s in ordered) {
    if (!watchedSeasonIds.contains(s.id)) {
      return s;
    }
  }
  return ordered.isEmpty ? null : ordered.first;
}

Episode? firstUnwatched(List<Episode> episodes, Set<String> watchedIds) {
  final List<Episode> ordered = List<Episode>.of(episodes)
    ..sort((Episode a, Episode b) => a.number.compareTo(b.number));
  for (final Episode e in ordered) {
    if (!watchedIds.contains(e.id)) {
      return e;
    }
  }
  return ordered.isEmpty ? null : ordered.first;
}

NextEpisode? nextInSeason(
  int season,
  List<Episode> episodes,
  Set<String> watchedIds,
  Map<String, int> progress,
) {
  final Episode? episode = firstUnwatched(episodes, watchedIds);
  if (episode == null) {
    return null;
  }
  return (
    episode: episode,
    season: season,
    progressPercent: progress[episode.id] ?? 0,
  );
}

Future<NextEpisode?> resolveNextEpisode(
  CatalogApi catalog,
  String userId,
  String seriesId,
  List<Season> seasons,
  Set<String> watchedSeasonIds,
) async {
  final Season? season = nextSeason(seasons, watchedSeasonIds);
  if (season == null) {
    return null;
  }
  List<Episode> episodes = const <Episode>[];
  try {
    episodes = await catalog.episodes(season.id, series: seriesId);
  } catch (_) {
    return null;
  }
  if (episodes.isEmpty) {
    return null;
  }
  List<ItemState> states = const <ItemState>[];
  try {
    states = await catalog.stateBatch(userId, <TitleRef>[
      for (final Episode e in episodes)
        TitleRef(type: TitleKind.episode, id: e.id),
    ]);
  } catch (_) {}
  return nextInSeason(
    season.number,
    episodes,
    <String>{
      for (final ItemState s in states)
        if (s.watched) s.title.id,
    },
    <String, int>{
      for (final ItemState s in states) s.title.id: s.progressPercent,
    },
  );
}
