import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/job/job_node.dart';
import 'package:shadowmask/model/job/jobs_feed.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shared_preferences/shared_preferences.dart';

AdminApi _admin(MockClient mock) =>
    AdminApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

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

  test('jobs GETs a page and parses both tab counts', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[
            <String, dynamic>{
              'id': 'j1',
              'kind': 'fetch',
              'status': 'queued',
              'priority': 'normal',
              'created_at': 'a',
              'updated_at': 'b',
            },
          ],
          'total': 9,
          'offset': 0,
          'limit': 50,
          'active_total': 3,
          'all_total': 9,
        }),
      ),
    );

    final JobsFeed feed = await admin.jobs();

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/jobs');
    expect(feed.page.items, hasLength(1));
    expect(feed.page.items.first.id, 'j1');
    expect(feed.activeTotal, 3);
    expect(feed.allTotal, 9);
  });

  test('jobs sends the state, filter and paging it was given', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 50,
        }),
      ),
    );

    await admin.jobs(
      offset: 50,
      limit: 25,
      filter: 'art work',
      activeOnly: true,
    );

    expect(seen.url.queryParameters['offset'], '50');
    expect(seen.url.queryParameters['limit'], '25');
    expect(seen.url.queryParameters['filter'], 'art work');
    expect(seen.url.queryParameters['state'], 'active');
  });

  test('jobs omits state and filter when they are not set', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 50,
        }),
      ),
    );

    await admin.jobs(filter: '');

    expect(seen.url.queryParameters.containsKey('state'), isFalse);
    expect(seen.url.queryParameters.containsKey('filter'), isFalse);
  });

  test('jobChildren GETs the children sub-resource with depth', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[
            <String, dynamic>{
              'id': 'j2',
              'kind': 'artwork',
              'status': 'queued',
              'priority': 'normal',
              'created_at': 'a',
              'updated_at': 'b',
              'parent_id': 'j 1',
              'depth': 2,
            },
          ],
          'total': 1,
          'offset': 0,
          'limit': 50,
        }),
      ),
    );

    final Paged<JobNode> children = await admin.jobChildren('j 1');

    expect(seen.url.path, '/api/v1/admin/jobs/j%201/children');
    expect(children.items.first.job.id, 'j2');
    expect(children.items.first.depth, 2);
  });

  test('cancelJob posts to the cancel sub-resource', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.cancelJob('j 1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/admin/jobs/j%201/cancel');
  });

  test('jobLog parses the lines envelope', () async {
    final AdminApi admin = _admin(
      _capture(
        (_) {},
        body: jsonEncode(<String, dynamic>{
          'lines': <String>['one', 'two'],
        }),
      ),
    );

    final lines = await admin.jobLog('j1');

    expect(lines, <String>['one', 'two']);
  });

  test('versions GETs the paged admin versions', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[
            <String, dynamic>{
              'id': 'v1',
              'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
              'library_id': 'l1',
              'quality': 'hd',
              'container': 'mkv',
              'path': '/movies/x.mkv',
            },
          ],
          'total': 1,
          'offset': 0,
          'limit': 20,
        }),
      ),
    );

    final page = await admin.versions();

    expect(seen.url.path, '/api/v1/admin/versions');
    expect(page.items.single.path, '/movies/x.mkv');
  });

  test('transcribe omits a null audio index but sends the language', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.transcribe('v1', sourceLanguage: 'eng');

    expect(seen.url.path, '/api/v1/admin/versions/v1/transcribe');
    expect(jsonDecode(seen.body), <String, dynamic>{'source_language': 'eng'});
  });

  test('translate sends source subtitle and target language', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.translate('v1', sourceSubtitleId: 's1', targetLanguage: 'spa');

    expect(seen.url.path, '/api/v1/admin/versions/v1/translate');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'source_subtitle_id': 's1',
      'target_language': 'spa',
    });
  });

  test('upscale sends the target height', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.upscale('v1', targetHeight: 2160);

    expect(seen.url.path, '/api/v1/admin/versions/v1/upscale');
    expect(jsonDecode(seen.body), <String, dynamic>{'target_height': 2160});
  });

  test('combineSubtitles sends top and bottom ids', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.combineSubtitles(
      'v1',
      topSubtitleId: 't',
      bottomSubtitleId: 'b',
    );

    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/combine');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'top_subtitle_id': 't',
      'bottom_subtitle_id': 'b',
    });
  });

  test('relink posts to the version relink route with a target', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.relink('v1', <String, dynamic>{
      'kind': 'provider',
      'source': 'tmdb',
      'value': 'movie/603',
    });

    expect(seen.url.path, '/api/v1/versions/v1/relink');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'target': <String, dynamic>{
        'kind': 'provider',
        'source': 'tmdb',
        'value': 'movie/603',
      },
    });
  });

  test('subtitleSearch forwards q and language query params', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, body: jsonEncode(<dynamic>[])),
    );

    await admin.subtitleSearch('v1', query: 'matrix', language: 'eng');

    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/search');
    expect(seen.url.queryParameters['q'], 'matrix');
    expect(seen.url.queryParameters['language'], 'eng');
  });

  test('subtitleDownload sends the file id', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.subtitleDownload('v1', fileId: 'f1', language: 'eng');

    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/download');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'file_id': 'f1',
      'language': 'eng',
    });
  });

  test('subtitleText GETs the subtitle and unwraps the content', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{'content': 'Hallo'}),
      ),
    );

    final String text = await admin.subtitleText('v1', 's1');

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/s1');
    expect(text, 'Hallo');
  });

  test('renameSubtitle PUTs the subtitle with a language', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 204),
    );

    await admin.renameSubtitle('v1', 's1', language: 'fra');

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/s1');
    expect(jsonDecode(seen.body), <String, dynamic>{'language': 'fra'});
  });

  test('deleteSubtitle DELETEs the subtitle', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 204),
    );

    await admin.deleteSubtitle('v1', 's1');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/admin/versions/v1/subtitles/s1');
  });

  test('deleteVersion DELETEs the admin version route', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 204),
    );

    await admin.deleteVersion('v1');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/admin/versions/v1');
  });

  test(
    'refreshMetadata routes movies and series to their own collection',
    () async {
      final List<String> paths = <String>[];
      final AdminApi admin = _admin(
        _capture((http.Request req) => paths.add(req.url.path), status: 202),
      );

      await admin.refreshMetadata(TitleKind.movie, 'm1');
      await admin.refreshMetadata(TitleKind.series, 's1');

      expect(paths, <String>[
        '/api/v1/movies/m1/refresh',
        '/api/v1/series/s1/refresh',
      ]);
    },
  );

  test('refreshMetadata sends no body when nothing is asked for', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.refreshMetadata(TitleKind.movie, 'm1');

    expect(seen.body, isEmpty);
  });

  test('refreshMetadata carries the force flag when asked', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.refreshMetadata(TitleKind.series, 's1', force: true);

    expect(jsonDecode(seen.body), <String, dynamic>{'force': true});
  });

  test('editMovie PUTs the body to the movie itself', () async {
    late http.Request seen;
    final AdminApi admin = _admin(_capture((http.Request req) => seen = req));

    await admin.editMovie('m1', <String, dynamic>{'title': 'Edited'});

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/movies/m1');
    expect(jsonDecode(seen.body), <String, dynamic>{'title': 'Edited'});
  });

  test('editSeries PUTs the body to the series itself', () async {
    late http.Request seen;
    final AdminApi admin = _admin(_capture((http.Request req) => seen = req));

    await admin.editSeries('s1', <String, dynamic>{'title': 'Edited'});

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/series/s1');
  });

  test('editEpisode PUTs under its series and season', () async {
    late http.Request seen;
    final AdminApi admin = _admin(_capture((http.Request req) => seen = req));

    await admin.editEpisode('s1', 'se1', 'e1', <String, dynamic>{
      'title': 'Edited',
    });

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/series/s1/seasons/se1/episodes/e1');
  });

  test('refreshMetadata refuses a kind that has no refresh route', () async {
    final List<String> paths = <String>[];
    final AdminApi admin = _admin(
      _capture((http.Request req) => paths.add(req.url.path), status: 202),
    );

    expect(
      () => admin.refreshMetadata(TitleKind.episode, 'e1'),
      throwsArgumentError,
    );
    expect(paths, isEmpty);
  });

  test('createFetch posts the fetch request body', () async {
    late http.Request seen;
    final AdminApi admin = _admin(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await admin.createFetch(<String, dynamic>{
      'source_url': 'https://x',
      'kind': 'movie',
      'library_id': 'l1',
      'title': 'The Matrix',
    });

    expect(seen.url.path, '/api/v1/admin/fetch');
    expect(
      (jsonDecode(seen.body) as Map<String, dynamic>)['title'],
      'The Matrix',
    );
  });
}
