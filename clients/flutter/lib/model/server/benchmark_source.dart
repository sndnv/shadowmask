import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/catalog/version_detail.dart';

part 'benchmark_source.freezed.dart';
part 'benchmark_source.g.dart';

@freezed
abstract class BenchmarkSource with _$BenchmarkSource {
  const factory BenchmarkSource({
    String? codec,
    int? width,
    int? height,
    HdrFormat? hdr,
    @Default(0) int durationMs,
  }) = _BenchmarkSource;

  factory BenchmarkSource.fromJson(Map<String, dynamic> json) =>
      _$BenchmarkSourceFromJson(json);
}
