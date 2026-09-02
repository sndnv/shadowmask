import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';

part 'season.freezed.dart';
part 'season.g.dart';

@freezed
abstract class Season with _$Season {
  const factory Season({
    required String id,
    required String seriesId,
    String? seriesTitle,
    Artwork? seriesArtwork,
    required int number,
    String? title,
    String? overview,
    String? addedAt,
    String? updatedAt,
    Artwork? artwork,
  }) = _Season;

  factory Season.fromJson(Map<String, dynamic> json) => _$SeasonFromJson(json);
}
