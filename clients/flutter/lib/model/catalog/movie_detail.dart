import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/content_rating.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';

part 'movie_detail.freezed.dart';
part 'movie_detail.g.dart';

@freezed
abstract class MovieDetail with _$MovieDetail {
  const factory MovieDetail({
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
    @Default(<Genre>[]) List<Genre> genres,
    @Default(<Credit>[]) List<Credit> credits,
    @Default(<Studio>[]) List<Studio> studios,
    @Default(<Rating>[]) List<Rating> ratings,
    @Default(<ExternalId>[]) List<ExternalId> externalIds,
    @Default(<Extra>[]) List<Extra> extras,
  }) = _MovieDetail;

  factory MovieDetail.fromJson(Map<String, dynamic> json) =>
      _$MovieDetailFromJson(json);
}
