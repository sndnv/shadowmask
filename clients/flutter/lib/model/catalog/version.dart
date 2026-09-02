import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';

part 'version.freezed.dart';
part 'version.g.dart';

@freezed
abstract class Version with _$Version {
  const factory Version({
    required String id,
    required TitleRef title,
    required String libraryId,
    required Quality quality,
    required String container,
    @Default(0) int sizeBytes,
    @Default(0) int durationMs,
    @Default(true) bool available,
    String? addedAt,
    String? updatedAt,
    String? path,
  }) = _Version;

  factory Version.fromJson(Map<String, dynamic> json) =>
      _$VersionFromJson(json);
}
