import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/model/library/cancelled_jobs.dart';
import 'package:shadowmask/model/library/clean_report.dart';
import 'package:shadowmask/model/library/purge_report.dart';
import 'package:shared_preferences/shared_preferences.dart';

LibraryApi _lib(MockClient mock) =>
    LibraryApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

MockClient _capture(
  void Function(http.Request req) sink, {
  String body = '',
  int status = 200,
}) => MockClient((http.Request req) async {
  sink(req);
  return http.Response(body, status);
});

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('createLibrary posts the library body', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 201),
    );

    await api.createLibrary(<String, dynamic>{'name': 'Movies'});

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries');
    expect(jsonDecode(seen.body), <String, dynamic>{'name': 'Movies'});
  });

  test('updateLibrary PUTs to the encoded id', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 200),
    );

    await api.updateLibrary('l 1', <String, dynamic>{'name': 'x'});

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/libraries/l%201');
  });

  test('triggerScan posts to the scan sub-resource', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.triggerScan('l1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l1/scan');
  });

  test('refreshMetadata posts to the refresh-metadata sub-resource', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.refreshMetadata('l1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l1/refresh-metadata');
  });

  test('resolveUnmatched wraps the target under a target key', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.resolveUnmatched('l1', 'u1', <String, dynamic>{
      'kind': 'existing',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
    });

    expect(seen.url.path, '/api/v1/libraries/l1/unmatched/u1/resolve');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'target': <String, dynamic>{
        'kind': 'existing',
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      },
    });
  });

  test('duplicates GETs the paged duplicates', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 0,
        }),
      ),
    );

    await api.duplicates('l1');

    expect(seen.url.path, '/api/v1/libraries/l1/duplicates');
  });

  test('purge posts the wipe choice and reads the counts', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'movies': 1,
          'series': 2,
          'seasons': 3,
          'episodes': 4,
          'subtitles': 5,
          'wiped_users': 6,
        }),
      ),
    );

    final PurgeReport report = await api.purge('l 1', wipeUserData: true);

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l%201/purge');
    expect(jsonDecode(seen.body), <String, dynamic>{'wipe_user_data': true});
    expect(
      report,
      const PurgeReport(
        movies: 1,
        series: 2,
        seasons: 3,
        episodes: 4,
        subtitles: 5,
        wipedUsers: 6,
      ),
    );
  });

  test('clean and cleanAll post the wipe choice and read the counts', () async {
    final List<http.Request> seen = <http.Request>[];
    final LibraryApi api = _lib(
      _capture(
        seen.add,
        body: jsonEncode(<String, dynamic>{
          'versions': 1,
          'movies': 2,
          'series': 3,
          'seasons': 4,
          'episodes': 5,
          'wiped_users': 6,
        }),
      ),
    );

    final CleanReport one = await api.clean('l 1', wipeUserData: true);
    final CleanReport every = await api.cleanAll(wipeUserData: false);

    expect(seen.map((http.Request r) => '${r.method} ${r.url.path}'), <String>[
      'POST /api/v1/libraries/l%201/clean',
      'POST /api/v1/admin/clean',
    ]);
    expect(seen.map((http.Request r) => jsonDecode(r.body)), <dynamic>[
      <String, dynamic>{'wipe_user_data': true},
      <String, dynamic>{'wipe_user_data': false},
    ]);
    const CleanReport counts = CleanReport(
      versions: 1,
      movies: 2,
      series: 3,
      seasons: 4,
      episodes: 5,
      wipedUsers: 6,
    );
    expect(one, counts);
    expect(every, counts);
  });

  test('a refused purge carries the jobs in the way', () async {
    final LibraryApi api = _lib(
      _capture(
        (_) {},
        status: 409,
        body: jsonEncode(<String, dynamic>{
          'error': <String, dynamic>{
            'code': 'library_busy',
            'message': 'the library is busy',
          },
          'jobs': <dynamic>[
            <String, dynamic>{'id': 'scan-1'},
            'not a job',
          ],
        }),
      ),
    );

    await expectLater(
      api.purge('l1', wipeUserData: false),
      throwsA(
        isA<ApiException>()
            .having((ApiException e) => e.code, 'code', 'library_busy')
            .having((ApiException e) => e.jobs, 'jobs', <Map<String, dynamic>>[
              <String, dynamic>{'id': 'scan-1'},
            ]),
      ),
    );
  });

  test('cancelJobs posts to the library and reads the counts', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'cancelled': 3,
          'signalled': 1,
          'running': 2,
        }),
      ),
    );

    final CancelledJobs outcome = await api.cancelJobs('l1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l1/jobs/cancel');
    expect(seen.body, isEmpty);
    expect(
      outcome,
      const CancelledJobs(cancelled: 3, signalled: 1, running: 2),
    );
  });

  test('cancelJobs names the operation whose blockers it cancels', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'cancelled': 1,
          'signalled': 0,
          'running': 0,
        }),
      ),
    );

    await api.cancelJobs('l1', blocking: 'clean');

    expect(jsonDecode(seen.body), <String, dynamic>{'blocking': 'clean'});
  });
}
