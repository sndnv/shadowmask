import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/content_rating.dart';

part 'movie.freezed.dart';
part 'movie.g.dart';

@freezed
abstract class Movie with _$Movie {
  const factory Movie({
    required String id,
    required String title,
    int? year,
    String? overview,
    int? runtimeMinutes,
    ContentRating? contentRating,
    @Default(false) bool manuallyEdited,
    String? addedAt,
    String? updatedAt,
    Artwork? artwork,
  }) = _Movie;

  factory Movie.fromJson(Map<String, dynamic> json) => _$MovieFromJson(json);
}
