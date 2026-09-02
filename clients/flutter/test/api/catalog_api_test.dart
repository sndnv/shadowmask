import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shared_preferences/shared_preferences.dart';

CatalogApi _catalog(MockClient mock) =>
    CatalogApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test(
    'movies encodes sort, order, genres and offset then parses the page',
    () async {
      late Uri seen;
      final CatalogApi catalog = _catalog(
        MockClient((http.Request req) async {
          seen = req.url;
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <dynamic>[
                <String, dynamic>{'id': 'm1', 'title': 'Alpha', 'year': 2020},
              ],
              'total': 1,
              'offset': 20,
              'limit': 50,
            }),
            200,
          );
        }),
      );

      final page = await catalog.movies(
        sort: 'title',
        order: 'desc',
        genres: <String>['Action', 'Drama'],
        offset: 20,
      );

      expect(seen.path, '/api/v1/movies');
      expect(seen.queryParameters['sort'], 'title');
      expect(seen.queryParameters['order'], 'desc');
      expect(seen.queryParameters['genres'], 'Action,Drama');
      expect(seen.queryParameters['offset'], '20');
      expect(page.total, 1);
      expect(page.items.single.title, 'Alpha');
    },
  );

  test('genres scopes the list to a kind when one is given', () async {
    final List<Uri> seen = <Uri>[];
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        seen.add(req.url);
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{'id': 'g1', 'name': 'Action'},
          ]),
          200,
        );
      }),
    );

    await catalog.genres(kind: 'series');
    await catalog.genres();

    expect(seen.first.path, '/api/v1/genres');
    expect(seen.first.queryParameters['kind'], 'series');
    expect(seen.last.queryParameters.containsKey('kind'), isFalse);
  });

  test('stateBatch posts the title refs and parses the states', () async {
    Map<String, dynamic>? body;
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        body = jsonDecode(req.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
              'favorite': true,
              'watchlisted': false,
              'watched': true,
              'completed': false,
            },
          ]),
          200,
        );
      }),
    );

    final states = await catalog.stateBatch('u1', <TitleRef>[
      const TitleRef(type: TitleKind.movie, id: 'm1'),
    ]);

    expect((body!['titles'] as List<dynamic>).first, <String, dynamic>{
      'type': 'movie',
      'id': 'm1',
    });
    expect(states.single.watched, isTrue);
    expect(states.single.favorite, isTrue);
  });

  test('search parses typed results into cards preserving order', () async {
    final CatalogApi catalog = _catalog(
      MockClient((http.Request _) async {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'items': <dynamic>[
              <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'Alpha'},
              <String, dynamic>{'type': 'person', 'id': 'p1', 'name': 'Ada'},
            ],
            'total': 2,
            'offset': 0,
            'limit': 50,
          }),
          200,
        );
      }),
    );

    final page = await catalog.search(q: 'a');

    expect(page.items[0].ref.type, TitleKind.movie);
    expect(page.items[1].ref.type, TitleKind.person);
    expect(page.items[1].title, 'Ada');
  });

  test('setWatched puts the type and watched flag on the reference', () async {
    late http.Request seen;
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        seen = req;
        return http.Response('', 204);
      }),
    );

    await catalog.setWatched(
      'u1',
      const TitleRef(type: TitleKind.series, id: 's1'),
      true,
    );

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/users/u1/watched/s1');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'type': 'series',
      'watched': true,
    });
  });

  test('addToWatchlist puts type; removeFavorite deletes', () async {
    final List<http.Request> seen = <http.Request>[];
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        seen.add(req);
        return http.Response('', 204);
      }),
    );

    const TitleRef ref = TitleRef(type: TitleKind.movie, id: 'm1');
    await catalog.addToWatchlist('u1', ref);
    await catalog.removeFavorite('u1', ref);

    expect(seen[0].method, 'PUT');
    expect(seen[0].url.path, '/api/v1/users/u1/watchlist/m1');
    expect(jsonDecode(seen[0].body), <String, dynamic>{'type': 'movie'});
    expect(seen[1].method, 'DELETE');
    expect(seen[1].url.path, '/api/v1/users/u1/favorites/m1');
  });

  test('stateRollup posts targets and parses the rollup', () async {
    Map<String, dynamic>? body;
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        body = jsonDecode(req.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'target': <String, dynamic>{'type': 'series', 'id': 's1'},
              'watched': true,
              'completed': false,
              'watched_episodes': 3,
              'total_episodes': 10,
            },
          ]),
          200,
        );
      }),
    );

    final rollups = await catalog.stateRollup('u1', <TitleRef>[
      const TitleRef(type: TitleKind.series, id: 's1'),
    ]);

    expect((body!['targets'] as List<dynamic>).first, <String, dynamic>{
      'type': 'series',
      'id': 's1',
    });
    expect(rollups.single.watched, isTrue);
    expect(rollups.single.watchedEpisodes, 3);
    expect(rollups.single.totalEpisodes, 10);
  });

  test('titleCards posts refs and maps the tagged cards', () async {
    Map<String, dynamic>? body;
    final CatalogApi catalog = _catalog(
      MockClient((http.Request req) async {
        body = jsonDecode(req.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'Alpha'},
          ]),
          200,
        );
      }),
    );

    final cards = await catalog.titleCards(<TitleRef>[
      const TitleRef(type: TitleKind.movie, id: 'm1'),
    ]);

    expect(body!['titles'], <dynamic>[
      <String, dynamic>{'type': 'movie', 'id': 'm1'},
    ]);
    expect(cards.single.ref.type, TitleKind.movie);
    expect(cards.single.title, 'Alpha');
  });
}
