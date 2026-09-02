import 'package:freezed_annotation/freezed_annotation.dart';

part 'detail_dimensions.freezed.dart';
part 'detail_dimensions.g.dart';

@freezed
abstract class Genre with _$Genre {
  const factory Genre({required String id, required String name}) = _Genre;

  factory Genre.fromJson(Map<String, dynamic> json) => _$GenreFromJson(json);
}

@freezed
abstract class Studio with _$Studio {
  const factory Studio({required String id, required String name}) = _Studio;

  factory Studio.fromJson(Map<String, dynamic> json) => _$StudioFromJson(json);
}

@freezed
abstract class Rating with _$Rating {
  const factory Rating({required String source, required double value}) =
      _Rating;

  factory Rating.fromJson(Map<String, dynamic> json) => _$RatingFromJson(json);
}

@freezed
abstract class ExternalId with _$ExternalId {
  const factory ExternalId({required String source, required String value}) =
      _ExternalId;

  factory ExternalId.fromJson(Map<String, dynamic> json) =>
      _$ExternalIdFromJson(json);
}

@freezed
abstract class PersonRef with _$PersonRef {
  const factory PersonRef({required String id, required String name}) =
      _PersonRef;

  factory PersonRef.fromJson(Map<String, dynamic> json) =>
      _$PersonRefFromJson(json);
}

enum CreditRole {
  @JsonValue('actor')
  actor,
  @JsonValue('director')
  director,
  @JsonValue('writer')
  writer,
}

@freezed
abstract class Credit with _$Credit {
  const factory Credit({
    required PersonRef person,
    required CreditRole role,
    String? character,
    @Default(0) int order,
  }) = _Credit;

  factory Credit.fromJson(Map<String, dynamic> json) => _$CreditFromJson(json);
}

enum ExtraKind {
  @JsonValue('trailer')
  trailer,
  @JsonValue('featurette')
  featurette,
  @JsonValue('behind_the_scenes')
  behindTheScenes,
}

@freezed
abstract class Extra with _$Extra {
  const factory Extra({required ExtraKind kind, required String title}) =
      _Extra;

  factory Extra.fromJson(Map<String, dynamic> json) => _$ExtraFromJson(json);
}
