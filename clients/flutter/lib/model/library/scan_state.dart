import 'package:freezed_annotation/freezed_annotation.dart';

part 'scan_state.freezed.dart';
part 'scan_state.g.dart';

enum ScanStatus {
  @JsonValue('idle')
  idle,
  @JsonValue('queued')
  queued,
  @JsonValue('running')
  running,
  @JsonValue('failed')
  failed,
}

@freezed
abstract class ScanState with _$ScanState {
  const factory ScanState({
    required String libraryId,
    required ScanStatus status,
    @Default(0) double progress,
    String? startedAt,
    String? lastScannedAt,
    String? error,
  }) = _ScanState;

  factory ScanState.fromJson(Map<String, dynamic> json) =>
      _$ScanStateFromJson(json);
}
