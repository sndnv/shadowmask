import 'package:freezed_annotation/freezed_annotation.dart';

part 'device_target.freezed.dart';
part 'device_target.g.dart';

enum TargetKind {
  @JsonValue('roku')
  roku,
  @JsonValue('android')
  android,
}

@freezed
abstract class DeviceTarget with _$DeviceTarget {
  const factory DeviceTarget({
    @JsonKey(unknownEnumValue: JsonKey.nullForUndefinedEnumValue)
    required TargetKind? kind,
    required List<String> addresses,
    required int port,
    String? appId,
    String? reportedAt,
  }) = _DeviceTarget;

  factory DeviceTarget.fromJson(Map<String, dynamic> json) =>
      _$DeviceTargetFromJson(json);
}
