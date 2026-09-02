import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/content_rating.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';

part 'series_detail.freezed.dart';
part 'series_detail.g.dart';

@freezed
abstract class SeriesDetail with _$SeriesDetail {
  const factory SeriesDetail({
    required String id,
    required String title,
    int? year,
    String? overview,
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
    @Default(0) int episodesTotal,
    @Default(0) int episodesWithAvailableVersion,
  }) = _SeriesDetail;

  factory SeriesDetail.fromJson(Map<String, dynamic> json) =>
      _$SeriesDetailFromJson(json);
}
