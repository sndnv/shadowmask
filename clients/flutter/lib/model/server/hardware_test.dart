import 'package:freezed_annotation/freezed_annotation.dart';

part 'hardware_test.freezed.dart';
part 'hardware_test.g.dart';

enum HardwareTestOutcome {
  @JsonValue('works')
  works,
  @JsonValue('failed')
  failed,
  @JsonValue('skipped')
  skipped,
}

@freezed
abstract class HardwareTest with _$HardwareTest {
  const factory HardwareTest({
    required HardwareTestOutcome outcome,
    bool? lowPower,
    int? elapsedMs,
    String? detail,
    String? lowPowerDetail,
  }) = _HardwareTest;

  factory HardwareTest.fromJson(Map<String, dynamic> json) =>
      _$HardwareTestFromJson(json);
}
