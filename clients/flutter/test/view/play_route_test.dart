import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/play_route.dart';
import 'package:shadowmask/view/title_link.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _version(String id, {bool available = true}) =>
    <String, dynamic>{
      'id': id,
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'lib',
      'quality': 'hd',
      'container': 'mp4',
      'available': available,
    };

CatalogApi _catalog({
  required List<Map<String, dynamic>> versions,
  List<String>? inProgress,
  List<String>? seen,
}) => CatalogApi(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen?.add(req.url.path);
      if (req.url.path.endsWith('/versions')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'items': versions,
            'total': versions.length,
          }),
          200,
        );
      }
      if (req.url.path.endsWith('/continue') && inProgress != null) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'in_progress': <dynamic>[
              for (final String id in inProgress)
                <String, dynamic>{
                  'card': <String, dynamic>{
                    'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                    'display_title': 'One',
                    'duration_ms': 1000,
                    'progress_percent': 10,
                  },
                  'progress': <String, dynamic>{'version_id': id},
                },
            ],
          }),
          200,
        );
      }
      return http.Response('', 500);
    }),
  ),
);

const TitleLink _movie = TitleLink(
  kind: TitleKind.movie,
  id: 'm1',
  label: 'One',
  queueEntryId: 'q4',
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('a movie with one version plays it and keeps its queue entry', () async {
    final List<String> seen = <String>[];
    final String route = await playRouteFor(
      _catalog(versions: <Map<String, dynamic>>[_version('v1')], seen: seen),
      'u1',
      _movie,
    );

    expect(route, watchRoute('v1', queue: 'q4'));
    expect(seen, contains('/api/v1/movies/m1/versions'));
  });

  test('several versions resume the one in progress', () async {
    final String route = await playRouteFor(
      _catalog(
        versions: <Map<String, dynamic>>[_version('v1'), _version('v2')],
        inProgress: <String>['v2'],
      ),
      'u1',
      _movie,
      controls: const <String, String?>{'audio': '1'},
    );

    expect(
      route,
      watchRoute('v2', controls: <String, String?>{'audio': '1'}, queue: 'q4'),
    );
  });

  test('a movie with nothing to play opens its page', () async {
    final String route = await playRouteFor(
      _catalog(
        versions: <Map<String, dynamic>>[_version('v1', available: false)],
      ),
      'u1',
      _movie,
    );

    expect(route, movieRoute('m1'));
  });

  test('an episode it cannot settle on opens the episode page', () async {
    final List<String> seen = <String>[];
    final String route = await playRouteFor(
      _catalog(
        versions: <Map<String, dynamic>>[_version('v1'), _version('v2')],
        seen: seen,
      ),
      'u1',
      const TitleLink(id: 'e2', seasonId: 'se1', label: 'Second'),
      seriesId: 's1',
    );

    expect(route, episodeRoute('e2', series: 's1'));
    expect(
      seen,
      contains('/api/v1/series/s1/seasons/se1/episodes/e2/versions'),
    );
  });
}
