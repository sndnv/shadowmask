import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/item_state.dart';

Season? nextSeason(List<Season> seasons, Set<String> watchedSeasonIds) {
  final List<Season> ordered = List<Season>.of(seasons)
    ..sort((Season a, Season b) => a.number.compareTo(b.number));
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

Future<Episode?> resolveNextEpisode(
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
  Set<String> watched = const <String>{};
  try {
    final List<ItemState> states = await catalog.stateBatch(userId, <TitleRef>[
      for (final Episode e in episodes)
        TitleRef(type: TitleKind.episode, id: e.id),
    ]);
    watched = <String>{
      for (final ItemState s in states)
        if (s.watched) s.title.id,
    };
  } catch (_) {}
  return firstUnwatched(episodes, watched);
}
