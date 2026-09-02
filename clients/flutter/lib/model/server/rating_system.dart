import 'package:freezed_annotation/freezed_annotation.dart';

part 'rating_system.freezed.dart';
part 'rating_system.g.dart';

@freezed
abstract class RatingSystem with _$RatingSystem {
  const factory RatingSystem({
    required String system,
    @Default(<String>[]) List<String> codes,
  }) = _RatingSystem;

  factory RatingSystem.fromJson(Map<String, dynamic> json) =>
      _$RatingSystemFromJson(json);
}
