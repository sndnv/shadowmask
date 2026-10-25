import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/report_admin_api.dart';
import 'package:shadowmask/model/diagnostics/report_detail.dart';
import 'package:shadowmask/model/diagnostics/report_summary.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shared_preferences/shared_preferences.dart';

const Map<String, dynamic> _summary = <String, dynamic>{
  'id': 'r1',
  'created_at': '2026-10-24T09:00:00Z',
  'user_id': 'u1',
  'player_id': 'p-9',
  'app': 'flutter',
  'platform': 'android 14',
  'app_version': '0.0.7',
  'note': null,
};

ReportAdminApi _reports(
  void Function(http.Request req) sink, {
  String body = '',
  int status = 200,
}) => ReportAdminApi(
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

  test('reports GETs a page of summaries', () async {
    late http.Request seen;
    final Paged<ReportSummary> page = await _reports(
      (http.Request req) => seen = req,
      body: jsonEncode(<String, dynamic>{
        'items': <Map<String, dynamic>>[_summary],
        'total': 1,
        'offset': 50,
        'limit': 50,
      }),
    ).reports(offset: 50, limit: 50);

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/diagnostics/reports');
    expect(seen.url.queryParameters, <String, String>{
      'offset': '50',
      'limit': '50',
    });
    expect(page.items.single.playerId, 'p-9');
    expect(page.items.single.appVersion, '0.0.7');
    expect(page.items.single.note, isNull);
  });

  test('reports sends a filter only when there is one', () async {
    final List<http.Request> seen = <http.Request>[];
    final ReportAdminApi reports = _reports(
      seen.add,
      body: jsonEncode(<String, dynamic>{'items': <dynamic>[], 'total': 0}),
    );

    await reports.reports(filter: 'roku');
    await reports.reports(filter: '');
    await reports.reports();

    expect(seen[0].url.queryParameters, <String, String>{'filter': 'roku'});
    expect(seen[1].url.queryParameters, isEmpty);
    expect(seen[2].url.queryParameters, isEmpty);
  });

  test('report GETs one report with its payload', () async {
    late http.Request seen;
    final ReportDetail report = await _reports(
      (http.Request req) => seen = req,
      body: jsonEncode(<String, dynamic>{
        ..._summary,
        'payload': <String, dynamic>{'schema': 1},
      }),
    ).report('r/1');

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/diagnostics/reports/r%2F1');
    expect(report.summary.id, 'r1');
    expect(report.payload, <String, dynamic>{'schema': 1});
    expect(report.toJson()['payload'], <String, dynamic>{'schema': 1});
    expect(report.toJson()['player_id'], 'p-9');
  });

  test('delete DELETEs the report', () async {
    late http.Request seen;
    await _reports((http.Request req) => seen = req, status: 204).delete('r1');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/admin/diagnostics/reports/r1');
  });
}
