import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/server/benchmark_report.dart';
import 'package:shadowmask/model/server/capabilities_report.dart';
import 'package:shadowmask/model/server/playback_prep_report.dart';
import 'package:shadowmask/model/server/server_log_page.dart';
import 'package:shadowmask/model/server/transcode_totals.dart';
import 'package:shadowmask/nav/routes.dart';
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

  Future<PlaybackPrepReport> playbackPrep() => _api.getJson(
    '/api/v1/admin/server/playback-prep',
    PlaybackPrepReport.fromJson,
  );

  Future<TranscodeTotals> totals() async => TranscodeTotals.fromSamples(
    parseMetrics(await _api.publicText('/metrics')),
  );

  Future<ServerLogPage> logs({
    int? limit,
    String? level,
    String? before,
    String? after,
  }) => _api.getJson(
    withQuery('/api/v1/admin/server/logs', <String, String?>{
      'limit': limit?.toString(),
      'level': level,
      'before': before,
      'after': after,
    }),
    ServerLogPage.fromJson,
  );
}
