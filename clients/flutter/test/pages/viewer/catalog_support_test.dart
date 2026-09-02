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
}
