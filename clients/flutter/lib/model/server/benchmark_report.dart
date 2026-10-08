import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/server/benchmark_progress.dart';
import 'package:shadowmask/model/server/benchmark_run.dart';
import 'package:shadowmask/model/server/benchmark_source.dart';

part 'benchmark_report.freezed.dart';
part 'benchmark_report.g.dart';

enum BenchmarkState {
  @JsonValue('idle')
  idle,
  @JsonValue('running')
  running,
  @JsonValue('done')
  done,
}

@freezed
abstract class BenchmarkReport with _$BenchmarkReport {
  const factory BenchmarkReport({
    required BenchmarkState state,
    String? versionId,
    String? startedAt,
    String? finishedAt,
    BenchmarkSource? source,
    int? keyframesMs,
    String? keyframesError,
    @Default(BenchmarkProgress()) BenchmarkProgress progress,
    @Default(<BenchmarkRun>[]) List<BenchmarkRun> runs,
  }) = _BenchmarkReport;

  factory BenchmarkReport.fromJson(Map<String, dynamic> json) =>
      _$BenchmarkReportFromJson(json);
}
