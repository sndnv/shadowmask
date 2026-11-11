import 'package:freezed_annotation/freezed_annotation.dart';

part 'server_feature.freezed.dart';
part 'server_feature.g.dart';

@freezed
abstract class ServerFeature with _$ServerFeature {
  const factory ServerFeature({
    required String name,
    @Default(false) bool available,
    @Default(false) bool enabled,
  }) = _ServerFeature;

  factory ServerFeature.fromJson(Map<String, dynamic> json) =>
      _$ServerFeatureFromJson(json);
}
