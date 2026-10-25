import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/diagnostics/report_detail.dart';
import 'package:shadowmask/model/diagnostics/report_summary.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/page.dart';

class ReportAdminApi {
  ReportAdminApi(this._api);

  final ApiClient _api;

  Future<Paged<ReportSummary>> reports({
    int offset = 0,
    int? limit,
    String? filter,
  }) => _api.getPage(
    withQuery('/api/v1/admin/diagnostics/reports', <String, String?>{
      ..._pageParams(offset, limit),
      if (filter != null && filter.isNotEmpty) 'filter': filter,
    }),
    ReportSummary.fromJson,
  );

  Future<ReportDetail> report(String id) => _api.getJson(
    '/api/v1/admin/diagnostics/reports/${_enc(id)}',
    ReportDetail.fromJson,
  );

  Future<void> delete(String id) =>
      _api.sendVoid('DELETE', '/api/v1/admin/diagnostics/reports/${_enc(id)}');

  Map<String, String?> _pageParams(int offset, int? limit) => <String, String?>{
    if (offset > 0) 'offset': offset.toString(),
    if (limit != null) 'limit': limit.toString(),
  };

  String _enc(String s) => Uri.encodeComponent(s);
}
