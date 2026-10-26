import 'package:freezed_annotation/freezed_annotation.dart';

part 'clean_report.freezed.dart';
part 'clean_report.g.dart';

@freezed
abstract class CleanReport with _$CleanReport {
  const factory CleanReport({
    @Default(0) int versions,
    @Default(0) int movies,
    @Default(0) int series,
    @Default(0) int seasons,
    @Default(0) int episodes,
    @Default(0) int wipedUsers,
  }) = _CleanReport;

  factory CleanReport.fromJson(Map<String, dynamic> json) =>
      _$CleanReportFromJson(json);
}
