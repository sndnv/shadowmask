import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/common/resume_card.dart';

class ContinueFeed {
  const ContinueFeed({required this.continueWatching, required this.upNext});

  final List<CatalogCard> continueWatching;
  final List<CatalogCard> upNext;

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
    final List<CatalogCard> up = <CatalogCard>[
      for (final Episode e in episodes)
        CatalogCard.fromEpisode(e, asSeriesPoster: true),
      for (final dynamic m
          in (json['next_movies'] as List<dynamic>?) ?? <dynamic>[])
        CatalogCard.fromMovie(Movie.fromJson(m as Map<String, dynamic>)),
    ];

    return ContinueFeed(continueWatching: cont, upNext: up);
  }

  static CatalogCard? _resumeCard(dynamic entry) {
    final Map<String, dynamic> e = entry as Map<String, dynamic>;
    final Object? card = e['card'];
    if (card is Map<String, dynamic>) {
      final Object? progress = e['progress'];
      final String? versionId = progress is Map<String, dynamic>
          ? progress['version_id'] as String?
          : null;
      return CatalogCard.fromResume(
        ResumeCard.fromJson(card),
        dismissVersionId: versionId,
      );
    }
    return null;
  }
}
