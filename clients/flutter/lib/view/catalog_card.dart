import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/util/credit_labels.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/catalog/collection.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/catalog/person_profile.dart';
import 'package:shadowmask/model/common/resume_card.dart';
import 'package:shadowmask/model/catalog/season.dart';
import 'package:shadowmask/model/catalog/series.dart';
import 'package:shadowmask/model/common/title_ref.dart';

class CatalogCard {
  CatalogCard({
    required this.ref,
    required this.route,
    required this.title,
    this.subtitle,
    this.caption,
    this.artwork,
    this.aspect = CardAspect.poster,
    this.mosaic = false,
    this.watched = false,
    this.watchlisted = false,
    this.favorite = false,
    this.stateKnown = false,
    this.progressPercent,
    this.dismissVersionId,
  });

  final TitleRef ref;
  final String route;
  final String title;
  final String? subtitle;
  final String? caption;
  final Artwork? artwork;
  final CardAspect aspect;
  final bool mosaic;
  bool watched;
  bool watchlisted;
  bool favorite;
  bool stateKnown;
  int? progressPercent;
  final String? dismissVersionId;

  static CatalogCard fromMovie(Movie m) => CatalogCard(
    ref: TitleRef(type: TitleKind.movie, id: m.id),
    route: movieRoute(m.id),
    title: m.title,
    subtitle: m.year?.toString(),
    artwork: m.artwork,
  );

  static CatalogCard fromSeries(Series s) => CatalogCard(
    ref: TitleRef(type: TitleKind.series, id: s.id),
    route: seriesRoute(s.id),
    title: s.title,
    subtitle: s.year?.toString(),
    artwork: s.artwork,
  );

  static CatalogCard fromHubSeries(Series s, int? episodes) => CatalogCard(
    ref: TitleRef(type: TitleKind.series, id: s.id),
    route: seriesRoute(s.id),
    title: s.title,
    subtitle: episodes == null
        ? s.year?.toString()
        : Strings.episodeCountLabel(episodes),
    artwork: s.artwork,
  );

  static CatalogCard fromEpisode(Episode e, {bool asSeriesPoster = false}) {
    final String route = episodeRoute(
      e.id,
      series: e.seriesId,
      season: e.seasonId,
    );
    final TitleRef ref = TitleRef(type: TitleKind.episode, id: e.id);
    if (!asSeriesPoster) {
      return CatalogCard(
        ref: ref,
        route: route,
        title: e.title,
        subtitle: _episodeSubtitle(e),
        artwork: e.artwork,
        aspect: CardAspect.landscape,
      );
    }
    return CatalogCard(
      ref: ref,
      route: route,
      title: e.seriesTitle ?? e.title,
      subtitle: episodeCode(e.seasonNumber, e.number),
      caption: e.seriesTitle == null ? null : e.title,
      artwork: e.seriesArtwork ?? e.artwork,
    );
  }

  static CatalogCard fromSeason(Season s) => CatalogCard(
    ref: TitleRef(type: TitleKind.season, id: s.id),
    route: seasonRoute(s.id, series: s.seriesId),
    title: s.title ?? Strings.seasonLabel(s.number),
    artwork: s.artwork,
  );

  static CatalogCard fromCollection(Collection c) => CatalogCard(
    ref: TitleRef(type: TitleKind.collection, id: c.id),
    route: collectionRoute(c.id),
    title: c.name,
    artwork: c.artwork,
    mosaic: c.artwork?.hasMultiplePosters ?? false,
  );

  static CatalogCard fromResume(ResumeCard r, {String? dismissVersionId}) {
    final bool isEpisode = r.title.type == TitleKind.episode;
    return CatalogCard(
      ref: r.title,
      route: titleRoute(r.title.type, r.title.id),
      title: isEpisode ? (r.seriesTitle ?? r.displayTitle) : r.displayTitle,
      subtitle: isEpisode
          ? (r.episodeNumber == null
                ? null
                : episodeCode(r.seasonNumber, r.episodeNumber!))
          : r.year?.toString(),
      caption: isEpisode && r.seriesTitle != null ? r.displayTitle : null,
      artwork: isEpisode ? (r.seriesArtwork ?? r.artwork) : r.artwork,
      progressPercent: r.progressPercent,
      dismissVersionId: dismissVersionId,
    );
  }

  static CatalogCard fromFilmography(
    FilmographyEntry e, {
    bool withCredit = false,
  }) => CatalogCard(
    ref: TitleRef(type: e.kind, id: e.titleId),
    route: titleRoute(e.kind, e.titleId),
    title: e.displayTitle,
    subtitle: e.year?.toString(),
    caption: withCredit ? creditCaption(e.role, e.character) : null,
    artwork: e.artwork,
  );

  static CatalogCard fromJson(
    Map<String, dynamic> json, {
    bool asSeriesPoster = false,
  }) {
    switch (json['type'] as String?) {
      case 'series':
        return fromSeries(Series.fromJson(json));
      case 'episode':
        return fromEpisode(
          Episode.fromJson(json),
          asSeriesPoster: asSeriesPoster,
        );
      case 'person':
        return _fromPersonJson(json);
      case 'movie':
      default:
        return fromMovie(Movie.fromJson(json));
    }
  }

  static CatalogCard _fromPersonJson(Map<String, dynamic> json) {
    final String id = json['id'] as String;
    final Object? art = json['artwork'];
    return CatalogCard(
      ref: TitleRef(type: TitleKind.person, id: id),
      route: personRoute(id),
      title: json['name'] as String? ?? '',
      artwork: art is Map<String, dynamic> ? Artwork.fromJson(art) : null,
      aspect: CardAspect.person,
    );
  }

  static String _episodeSubtitle(Episode e) {
    final String code = episodeCode(e.seasonNumber, e.number);
    return e.seriesTitle != null ? '${e.seriesTitle} · $code' : code;
  }
}
