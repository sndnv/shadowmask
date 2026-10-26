import 'package:freezed_annotation/freezed_annotation.dart';

part 'purge_report.freezed.dart';
part 'purge_report.g.dart';

@freezed
abstract class PurgeReport with _$PurgeReport {
  const factory PurgeReport({
    @Default(0) int movies,
    @Default(0) int series,
    @Default(0) int seasons,
    @Default(0) int episodes,
    @Default(0) int subtitles,
    @Default(0) int wipedUsers,
  }) = _PurgeReport;

  factory PurgeReport.fromJson(Map<String, dynamic> json) =>
      _$PurgeReportFromJson(json);
}
