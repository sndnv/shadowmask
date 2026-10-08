import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_admin_api.dart';
import 'package:shadowmask/model/server/benchmark_report.dart';
import 'package:shadowmask/model/server/capabilities_report.dart';
import 'package:shadowmask/model/server/transcode_totals.dart';
import 'package:shared_preferences/shared_preferences.dart';

ServerAdminApi _server(
  void Function(http.Request req) sink, {
  String body = '',
  int status = 200,
}) => ServerAdminApi(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      sink(req);
      return http.Response(body, status);
    }),
  ),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('capabilities GETs the report', () async {
    late http.Request seen;
    final CapabilitiesReport report = await _server(
      (http.Request req) => seen = req,
      body: jsonEncode(<String, dynamic>{'state': 'checking'}),
    ).capabilities();

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/server/capabilities');
    expect(report.state, CheckState.checking);
  });

  test('recheck POSTs to the recheck route', () async {
    late http.Request seen;
    await _server((http.Request req) => seen = req, status: 202).recheck();

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/admin/server/capabilities/recheck');
  });

  test('benchmark GETs the running or last benchmark', () async {
    late http.Request seen;
    final BenchmarkReport report = await _server(
      (http.Request req) => seen = req,
      body: jsonEncode(<String, dynamic>{
        'state': 'done',
        'progress': <String, dynamic>{'done': 6, 'total': 6},
        'runs': <dynamic>[],
      }),
    ).benchmark();

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/server/benchmark');
    expect(report.state, BenchmarkState.done);
  });

  test('startBenchmark POSTs the version id', () async {
    late http.Request seen;
    await _server(
      (http.Request req) => seen = req,
      status: 202,
    ).startBenchmark('v 1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/admin/server/benchmark');
    expect(jsonDecode(seen.body), <String, dynamic>{'version_id': 'v 1'});
  });

  test('totals reads /metrics at the root', () async {
    late http.Request seen;
    final TranscodeTotals totals = await _server(
      (http.Request req) => seen = req,
      body: 'transcode_fallbacks_total 2\n',
    ).totals();

    expect(seen.method, 'GET');
    expect(seen.url.path, '/metrics');
    expect(totals.fallbacks, 2);
  });
}
