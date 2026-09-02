import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';

part 'episode.freezed.dart';
part 'episode.g.dart';

@freezed
abstract class Episode with _$Episode {
  const factory Episode({
    required String id,
    required String seasonId,
    required int number,
    required String title,
    String? overview,
    int? runtimeMinutes,
    String? airDate,
    String? seriesId,
    String? seriesTitle,
    int? seasonNumber,
    String? seasonTitle,
    @Default(false) bool manuallyEdited,
    String? addedAt,
    String? updatedAt,
    Artwork? artwork,
    Artwork? seriesArtwork,
  }) = _Episode;

  factory Episode.fromJson(Map<String, dynamic> json) =>
      _$EpisodeFromJson(json);
}
