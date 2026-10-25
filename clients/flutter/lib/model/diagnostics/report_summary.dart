import 'package:freezed_annotation/freezed_annotation.dart';

part 'report_summary.freezed.dart';
part 'report_summary.g.dart';

@freezed
abstract class ReportSummary with _$ReportSummary {
  const factory ReportSummary({
    required String id,
    required String createdAt,
    required String userId,
    required String playerId,
    required String app,
    required String platform,
    required String appVersion,
    String? note,
  }) = _ReportSummary;

  factory ReportSummary.fromJson(Map<String, dynamic> json) =>
      _$ReportSummaryFromJson(json);
}
