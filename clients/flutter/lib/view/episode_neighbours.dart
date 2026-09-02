import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/util/format.dart';

class EpisodeLink {
  const EpisodeLink({
    required this.id,
    required this.seasonId,
    required this.label,
  });

  final String id;
  final String seasonId;
  final String label;
}

class EpisodeNeighbours {
  const EpisodeNeighbours({this.previous, this.next});

  final EpisodeLink? previous;
  final EpisodeLink? next;
}

class SeasonLink {
  const SeasonLink({required this.id, required this.label});

  final String id;
  final String label;
}

class SeasonNeighbours {
  const SeasonNeighbours({this.previous, this.next});

  final SeasonLink? previous;
  final SeasonLink? next;
}

SeasonNeighbours seasonNeighboursOf(String seasonId, List<Season> seasons) {
  final List<Season> ordered = _ordered(seasons);
  final int at = _seasonIndex(ordered, seasonId);
  if (at < 0) {
    return const SeasonNeighbours();
  }
  SeasonLink? step(int by) {
    final int index = at + by;
    if (index < 0 || index >= ordered.length) {
      return null;
    }
    final Season season = ordered[index];
    return SeasonLink(
      id: season.id,
      label: season.title ?? Strings.seasonLabel(season.number),
    );
  }

  return SeasonNeighbours(previous: step(-1), next: step(1));
}

List<Season> _ordered(List<Season> seasons) =>
    List<Season>.of(seasons)..sort((Season a, Season b) => a.number - b.number);

List<Episode> _sorted(List<Episode> episodes) =>
    List<Episode>.of(episodes)
      ..sort((Episode a, Episode b) => a.number - b.number);

int _seasonIndex(List<Season> seasons, String seasonId) =>
    seasons.indexWhere((Season s) => s.id == seasonId);

List<String> seasonsToLoad(
  Episode current,
  List<Season> seasons,
  List<Episode> currentSeason,
) {
  final List<Season> ordered = _ordered(seasons);
  final int at = _seasonIndex(ordered, current.seasonId);
  if (at < 0) {
    return const <String>[];
  }
  final List<Episode> episodes = _sorted(currentSeason);
  final int index = episodes.indexWhere((Episode e) => e.id == current.id);
  if (index < 0) {
    return const <String>[];
  }
  return <String>[
    if (index == 0 && at > 0) ordered[at - 1].id,
    if (index == episodes.length - 1 && at < ordered.length - 1)
      ordered[at + 1].id,
  ];
}

EpisodeLink? _link(Episode episode, Season season) => EpisodeLink(
  id: episode.id,
  seasonId: episode.seasonId,
  label: '${episodeCode(season.number, episode.number)} · ${episode.title}',
);

EpisodeNeighbours neighboursOf(
  Episode current,
  List<Season> seasons,
  Map<String, List<Episode>> episodesBySeason,
) {
  final List<Season> ordered = _ordered(seasons);
  final int at = _seasonIndex(ordered, current.seasonId);
  if (at < 0) {
    return const EpisodeNeighbours();
  }
  final List<Episode> episodes = _sorted(
    episodesBySeason[current.seasonId] ?? const <Episode>[],
  );
  final int index = episodes.indexWhere((Episode e) => e.id == current.id);
  if (index < 0) {
    return const EpisodeNeighbours();
  }

  EpisodeLink? edge(int seasonStep, bool takeLast) {
    final int neighbour = at + seasonStep;
    if (neighbour < 0 || neighbour >= ordered.length) {
      return null;
    }
    final Season season = ordered[neighbour];
    final List<Episode> theirs = _sorted(
      episodesBySeason[season.id] ?? const <Episode>[],
    );
    if (theirs.isEmpty) {
      return null;
    }
    return _link(takeLast ? theirs.last : theirs.first, season);
  }

  return EpisodeNeighbours(
    previous: index > 0
        ? _link(episodes[index - 1], ordered[at])
        : edge(-1, true),
    next: index < episodes.length - 1
        ? _link(episodes[index + 1], ordered[at])
        : edge(1, false),
  );
}

Future<EpisodeNeighbours> loadNeighbours(
  CatalogApi catalog,
  Episode episode,
) async {
  final String? seriesId = episode.seriesId;
  if (seriesId == null) {
    return const EpisodeNeighbours();
  }
  try {
    final List<Season> seasons = await catalog.seasons(seriesId);
    final Map<String, List<Episode>> bySeason = <String, List<Episode>>{
      episode.seasonId: await catalog.episodes(
        episode.seasonId,
        series: seriesId,
      ),
    };
    for (final String id in seasonsToLoad(
      episode,
      seasons,
      bySeason[episode.seasonId]!,
    )) {
      bySeason[id] = await catalog.episodes(id, series: seriesId);
    }
    return neighboursOf(episode, seasons, bySeason);
  } catch (_) {
    return const EpisodeNeighbours();
  }
}
