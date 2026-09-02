import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/common/title_ref.dart';

part 'person_profile.freezed.dart';
part 'person_profile.g.dart';

@freezed
abstract class FilmographyEntry with _$FilmographyEntry {
  const factory FilmographyEntry({
    required String titleId,
    required TitleKind kind,
    required String displayTitle,
    int? year,
    Artwork? artwork,
    required CreditRole role,
    String? character,
  }) = _FilmographyEntry;

  factory FilmographyEntry.fromJson(Map<String, dynamic> json) =>
      _$FilmographyEntryFromJson(json);
}

@freezed
abstract class PersonProfile with _$PersonProfile {
  const factory PersonProfile({
    required String id,
    required String name,
    String? biography,
    String? birthday,
    String? deathday,
    String? placeOfBirth,
    @Default(<String>[]) List<String> alsoKnownAs,
    Artwork? artwork,
    @Default(<FilmographyEntry>[]) List<FilmographyEntry> filmography,
  }) = _PersonProfile;

  factory PersonProfile.fromJson(Map<String, dynamic> json) =>
      _$PersonProfileFromJson(json);
}
