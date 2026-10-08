import 'package:freezed_annotation/freezed_annotation.dart';

part 'benchmark_progress.freezed.dart';
part 'benchmark_progress.g.dart';

@freezed
abstract class BenchmarkProgress with _$BenchmarkProgress {
  const factory BenchmarkProgress({
    @Default(0) int done,
    @Default(0) int total,
  }) = _BenchmarkProgress;

  factory BenchmarkProgress.fromJson(Map<String, dynamic> json) =>
      _$BenchmarkProgressFromJson(json);
}
