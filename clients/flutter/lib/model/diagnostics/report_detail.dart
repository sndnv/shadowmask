import 'package:shadowmask/model/diagnostics/report_summary.dart';

class ReportDetail {
  const ReportDetail({required this.summary, required this.payload});

  final ReportSummary summary;
  final Map<String, dynamic> payload;

  factory ReportDetail.fromJson(Map<String, dynamic> json) => ReportDetail(
    summary: ReportSummary.fromJson(json),
    payload: (json['payload'] as Map<String, dynamic>?) ?? <String, dynamic>{},
  );

  Map<String, dynamic> toJson() => <String, dynamic>{
    ...summary.toJson(),
    'payload': payload,
  };
}
