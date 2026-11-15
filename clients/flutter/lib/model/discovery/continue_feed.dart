import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/common/resume_card.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';

List<CatalogCard> oncePerTitle(Iterable<CatalogCard> cards) {
  final Set<String> seen = <String>{};
  return <CatalogCard>[
    for (final CatalogCard c in cards)
      if (seen.add(c.ref.key)) c,
  ];
}

class ContinueFeed {
  const ContinueFeed({
    required this.continueWatching,
    required this.upNext,
    this.queueNext,
  });

  final List<CatalogCard> continueWatching;
  final List<CatalogCard> upNext;
  final QueueEntry? queueNext;

  bool get isEmpty => continueWatching.isEmpty && upNext.isEmpty;

  Map<String, int> get resumeProgress => <String, int>{
    for (final CatalogCard c in continueWatching)
      if (c.dismissVersionId != null)
        c.dismissVersionId!: c.progressPercent ?? 0,
  };

  factory ContinueFeed.fromJson(Map<String, dynamic> json) {
    final List<CatalogCard> cont = <CatalogCard>[];
    for (final CatalogCard? c in <Iterable<dynamic>>[
      (json['now_playing'] as List<dynamic>?) ?? <dynamic>[],
      (json['in_progress'] as List<dynamic>?) ?? <dynamic>[],
    ].expand((Iterable<dynamic> list) => list).map(_resumeCard)) {
      if (c != null) {
        cont.add(c);
      }
    }

    final List<Episode> episodes = <Episode>[
      for (final dynamic e
          in (json['next_episodes'] as List<dynamic>?) ?? <dynamic>[])
        Episode.fromJson(e as Map<String, dynamic>),
    ];
    final Object? queued = json['queue_next'];
    final QueueEntry? queueNext = queued is Map<String, dynamic>
        ? QueueEntry.fromJson(queued)
        : null;
    final String? queuedSeries = switch (queued) {
      {'card': {'series_id': final String id}} => id,
      _ => null,
    };
    final List<(CatalogCard, String?)> candidates = <(CatalogCard, String?)>[
      if (queueNext != null) (queueNext.card, queuedSeries),
      for (final Episode e in episodes)
        (CatalogCard.fromEpisode(e, asSeriesPoster: true), e.seriesId),
      for (final dynamic m
          in (json['next_movies'] as List<dynamic>?) ?? <dynamic>[])
        (
          CatalogCard.fromMovie(Movie.fromJson(m as Map<String, dynamic>)),
          null,
        ),
    ];
    final Set<String> seen = <String>{};
    final List<CatalogCard> up = <CatalogCard>[
      for (final (CatalogCard card, String? series) in candidates)
        if (_firstInRail(seen, card, series)) card,
    ];

    return ContinueFeed(
      continueWatching: cont,
      upNext: up,
      queueNext: queueNext,
    );
  }

  static bool _firstInRail(Set<String> seen, CatalogCard card, String? series) {
    final String? seriesKey = series == null
        ? null
        : TitleRef(type: TitleKind.series, id: series).key;
    if (seen.contains(card.ref.key) || seen.contains(seriesKey)) {
      return false;
    }
    seen.add(card.ref.key);
    if (seriesKey != null) {
      seen.add(seriesKey);
    }
    return true;
  }

  static CatalogCard? _resumeCard(dynamic entry) {
    final Map<String, dynamic> e = entry as Map<String, dynamic>;
    final Object? card = e['card'];
    if (card is Map<String, dynamic>) {
      final Object? progress = e['progress'];
      final String? versionId = progress is Map<String, dynamic>
          ? progress['version_id'] as String?
          : e['version_id'] as String?;
      return CatalogCard.fromResume(
        ResumeCard.fromJson(card),
        dismissVersionId: versionId,
      );
    }
    return null;
  }
}
