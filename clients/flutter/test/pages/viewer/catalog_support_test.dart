import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shared_preferences/shared_preferences.dart';

CatalogCard _card(TitleKind kind, String id) => CatalogCard(
  ref: TitleRef(type: kind, id: id),
  title: id,
  route: '/$id',
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('series and season cards are tagged from the rollup', () async {
    final List<String> paths = <String>[];
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        paths.add(req.url.path);
        if (req.url.path.endsWith('/state/rollup')) {
          return http.Response(
            jsonEncode(<dynamic>[
              <String, dynamic>{
                'target': <String, dynamic>{'type': 'series', 'id': 'sr1'},
                'watched': true,
              },
              <String, dynamic>{
                'target': <String, dynamic>{'type': 'season', 'id': 'se1'},
                'watched': false,
              },
            ]),
            200,
            headers: <String, String>{'content-type': 'application/json'},
          );
        }
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
              'watched': true,
            },
          ]),
          200,
          headers: <String, String>{'content-type': 'application/json'},
        );
      }),
    );

    final List<CatalogCard> cards = <CatalogCard>[
      _card(TitleKind.movie, 'm1'),
      _card(TitleKind.series, 'sr1'),
      _card(TitleKind.season, 'se1'),
      _card(TitleKind.collection, 'c1'),
    ];
    await tagWatched(CatalogApi(api), 'u1', cards);

    expect(cards[0].watched, isTrue);
    expect(cards[1].watched, isTrue);
    expect(cards[2].watched, isFalse);
    expect(cards[3].watched, isFalse);
    expect(paths.where((String p) => p.endsWith('/state/rollup')).length, 1);
  });

  test(
    'a season card counts its episodes; a series card keeps its own',
    () async {
      final ApiClient api = ApiClient(
        baseUrl: 'http://test',
        httpClient: MockClient(
          (http.Request req) async => http.Response(
            jsonEncode(<dynamic>[
              <String, dynamic>{
                'target': <String, dynamic>{'type': 'series', 'id': 'sr1'},
                'total_episodes': 30,
              },
              <String, dynamic>{
                'target': <String, dynamic>{'type': 'season', 'id': 'se1'},
                'total_episodes': 10,
              },
              <String, dynamic>{
                'target': <String, dynamic>{'type': 'season', 'id': 'se2'},
                'total_episodes': 0,
              },
            ]),
            200,
            headers: <String, String>{'content-type': 'application/json'},
          ),
        ),
      );

      final CatalogCard series = CatalogCard(
        ref: const TitleRef(type: TitleKind.series, id: 'sr1'),
        title: 'Skyline',
        subtitle: '2008 · 3 seasons',
        route: '/sr1',
      );
      final List<CatalogCard> cards = <CatalogCard>[
        series,
        _card(TitleKind.season, 'se1'),
        _card(TitleKind.season, 'se2'),
      ];
      await tagWatched(CatalogApi(api), 'u1', cards);

      expect(series.subtitle, '2008 · 3 seasons');
      expect(cards[1].subtitle, '10 episodes');
      expect(cards[2].subtitle, isNull, reason: 'a count of 0 is left out');
    },
  );

  test('no rollup call when no card rolls up', () async {
    final List<String> paths = <String>[];
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        paths.add(req.url.path);
        return http.Response(
          jsonEncode(<dynamic>[]),
          200,
          headers: <String, String>{'content-type': 'application/json'},
        );
      }),
    );

    await tagWatched(CatalogApi(api), 'u1', <CatalogCard>[
      _card(TitleKind.movie, 'm1'),
    ]);

    expect(paths.any((String p) => p.endsWith('/state/rollup')), isFalse);
  });

  test('a failing rollup leaves the leaf tags intact', () async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        if (req.url.path.endsWith('/state/rollup')) {
          return http.Response('boom', 500);
        }
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
              'watched': true,
            },
          ]),
          200,
          headers: <String, String>{'content-type': 'application/json'},
        );
      }),
    );

    final List<CatalogCard> cards = <CatalogCard>[
      _card(TitleKind.movie, 'm1'),
      _card(TitleKind.series, 'sr1'),
    ];
    await tagWatched(CatalogApi(api), 'u1', cards);

    expect(cards[0].watched, isTrue);
    expect(cards[1].watched, isFalse);
  });

  ApiClient unwatchedEverywhere({bool fails = false}) => ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      if (fails) {
        return http.Response('boom', 500);
      }
      final List<dynamic> asked =
          (jsonDecode(req.body) as Map<String, dynamic>)['titles']
              as List<dynamic>;
      return http.Response(
        jsonEncode(<dynamic>[
          for (final dynamic t in asked)
            <String, dynamic>{'title': t, 'watched': false},
        ]),
        200,
        headers: <String, String>{'content-type': 'application/json'},
      );
    }),
  );

  test(
    'clearing takes the mark off a title the server calls unwatched',
    () async {
      final List<CatalogCard> kept = <CatalogCard>[
        _card(TitleKind.movie, 'm1')..watched = true,
      ];
      final List<CatalogCard> cleared = <CatalogCard>[
        _card(TitleKind.movie, 'm1')..watched = true,
      ];

      await tagWatched(CatalogApi(unwatchedEverywhere()), 'u1', kept);
      await tagWatched(
        CatalogApi(unwatchedEverywhere()),
        'u1',
        cleared,
        clearUnwatched: true,
      );

      expect(kept.single.watched, isTrue, reason: 'tagging alone only adds');
      expect(cleared.single.watched, isFalse);
    },
  );

  test('clearing leaves a title it never asked about alone', () async {
    final List<CatalogCard> cards = <CatalogCard>[
      for (int n = 0; n <= 200; n++)
        _card(TitleKind.movie, 'm$n')..watched = true,
    ];

    await tagWatched(
      CatalogApi(unwatchedEverywhere()),
      'u1',
      cards,
      clearUnwatched: true,
    );

    expect(cards.first.watched, isFalse);
    expect(cards.last.watched, isTrue, reason: 'one batch asks about 200');
    expect(cards.last.stateKnown, isFalse);
  });

  test('clearing leaves every mark alone when the state call fails', () async {
    final List<CatalogCard> cards = <CatalogCard>[
      _card(TitleKind.movie, 'm1')..watched = true,
    ];

    await tagWatched(
      CatalogApi(unwatchedEverywhere(fails: true)),
      'u1',
      cards,
      clearUnwatched: true,
    );

    expect(cards.single.watched, isTrue);
  });
}
