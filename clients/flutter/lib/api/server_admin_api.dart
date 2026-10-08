import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/server/benchmark_report.dart';
import 'package:shadowmask/model/server/capabilities_report.dart';
import 'package:shadowmask/model/server/transcode_totals.dart';
import 'package:shadowmask/util/prometheus.dart';

class ServerAdminApi {
  ServerAdminApi(this._api);

  final ApiClient _api;

  Future<CapabilitiesReport> capabilities() => _api.getJson(
    '/api/v1/admin/server/capabilities',
    CapabilitiesReport.fromJson,
  );

  Future<void> recheck() =>
      _api.sendVoid('POST', '/api/v1/admin/server/capabilities/recheck');

  Future<BenchmarkReport> benchmark() =>
      _api.getJson('/api/v1/admin/server/benchmark', BenchmarkReport.fromJson);

  Future<void> startBenchmark(String versionId) => _api.sendVoid(
    'POST',
    '/api/v1/admin/server/benchmark',
    body: <String, dynamic>{'version_id': versionId},
  );

  Future<TranscodeTotals> totals() async => TranscodeTotals.fromSamples(
    parseMetrics(await _api.publicText('/metrics')),
  );
}
