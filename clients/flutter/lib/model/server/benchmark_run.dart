import 'package:freezed_annotation/freezed_annotation.dart';

part 'benchmark_run.freezed.dart';
part 'benchmark_run.g.dart';

enum BenchmarkOutcome {
  @JsonValue('ok')
  ok,
  @JsonValue('failed')
  failed,
  @JsonValue('too_slow')
  tooSlow,
}

@freezed
abstract class BenchmarkRun with _$BenchmarkRun {
  const factory BenchmarkRun({
    required int segment,
    required int startMs,
    required int durationMs,
    required String encoder,
    required BenchmarkOutcome outcome,
    required int elapsedMs,
    required double realtime,
    String? detail,
  }) = _BenchmarkRun;

  factory BenchmarkRun.fromJson(Map<String, dynamic> json) =>
      _$BenchmarkRunFromJson(json);
}
