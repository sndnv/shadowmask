import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/content_rating.dart';

part 'series.freezed.dart';
part 'series.g.dart';

@freezed
abstract class Series with _$Series {
  const factory Series({
    required String id,
    required String title,
    int? year,
    String? overview,
    ContentRating? contentRating,
    @Default(false) bool manuallyEdited,
    String? addedAt,
    String? updatedAt,
    Artwork? artwork,
  }) = _Series;

  factory Series.fromJson(Map<String, dynamic> json) => _$SeriesFromJson(json);
}
