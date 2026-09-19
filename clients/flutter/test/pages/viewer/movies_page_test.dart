import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/pages/viewer/movies_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

const int _total = 5;
const int _pageSize = 2;

Map<String, dynamic> _movie(int n) => <String, dynamic>{
  'id': 'm$n',
  'title': 'Movie $n',
  'year': 2000 + n,
  'artwork': <String, dynamic>{},
};

Map<String, dynamic> _page(int offset) {
  final int end = (offset + _pageSize) > _total ? _total : offset + _pageSize;
  return <String, dynamic>{
    'items': <Map<String, dynamic>>[
      for (int n = offset; n < end; n++) _movie(n),
    ],
    'total': _total,
    'offset': offset,
    'limit': _pageSize,
  };
}

List<Map<String, dynamic>> _states(
  String body,
  String? inProgressId,
  int percent,
) {
  final List<dynamic> titles =
      (jsonDecode(body) as Map<String, dynamic>)['titles'] as List<dynamic>;
  return <Map<String, dynamic>>[
    for (final dynamic t in titles)
      <String, dynamic>{
        'title': t,
        'watched': false,
        'progress_percent': (t as Map<String, dynamic>)['id'] == inProgressId
            ? percent
            : 0,
      },
  ];
}

ApiClient _api(
  List<int> offsets, {
  bool failAfterFirst = false,
  bool stallAfterFirst = false,
  String? inProgress,
  int percent = 40,
  List<String>? paths,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    final String path = req.url.path;
    paths?.add(path);
    if (path.endsWith('/state/batch')) {
      return http.Response(
        jsonEncode(_states(req.body, inProgress, percent)),
        200,
      );
    }
    if (path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (path == '/api/v1/movies') {
      final int offset =
          int.tryParse(req.url.queryParameters['offset'] ?? '') ?? 0;
      offsets.add(offset);
      if (failAfterFirst && offsets.length > 1) {
        return http.Response('', 500);
      }
      if (stallAfterFirst && offsets.length > 1) {
        return Completer<http.Response>().future;
      }
      return http.Response(jsonEncode(_page(offset)), 200);
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

Map<String, dynamic> _library(String id, String name, String kind) =>
    <String, dynamic>{
      'id': id,
      'name': name,
      'kind': kind,
      'watcher': 'manual',
      'created_at': '',
      'updated_at': '',
    };

ApiClient _apiWithLibraries() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    final String path = req.url.path;
    if (path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (path == '/api/v1/movies') {
      return http.Response(jsonEncode(_page(0)), 200);
    }
    if (path == '/api/v1/libraries') {
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          _library('lib1', 'Films', 'movie'),
          _library('lib2', 'Shows', 'tv'),
        ]),
        200,
      );
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

Future<void> _pump(
  WidgetTester tester,
  ApiClient api, {
  bool settle = true,
  Size window = const Size(1400, 1000),
}) async {
  tester.view.physicalSize = window;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => MoviesPage(
            api: api,
            query: ListQuery.fromArgs(const <String, String>{}),
          ),
        ),
      ),
    ),
  );
  if (settle) {
    await tester.pumpAndSettle();
    return;
  }
  for (int i = 0; i < 8; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an in-progress movie carries its progress into the list', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets, inProgress: 'm1'));

    final Iterable<CatalogCardTile> tiles = tester.widgetList<CatalogCardTile>(
      find.byType(CatalogCardTile),
    );
    expect(
      tiles
          .where((CatalogCardTile t) => t.card.progressPercent != null)
          .map((CatalogCardTile t) => t.card.ref.id),
      <String>['m1'],
      reason: 'only the title the state batch reported is marked',
    );
    expect(find.byTooltip('Movie 1\n2001\nResume at 40%'), findsWidgets);
  });

  testWidgets('paging the grid never reaches for the whole-catalog feed', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    final List<String> paths = <String>[];
    await _pump(tester, _api(offsets, inProgress: 'm1', paths: paths));

    expect(
      paths.where((String p) => p.endsWith('/continue')),
      isEmpty,
      reason:
          'the grid asks about the 50 titles it is showing, and /continue '
          'derives its answer from every title in the catalog',
    );
    expect(
      paths.where((String p) => p.endsWith('/state/batch')).length,
      offsets.length,
      reason: 'one bounded state lookup per page, including every load more',
    );
  });

  testWidgets('the pager is gone from the movies list', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets));

    expect(find.byType(Pagination), findsNothing);
    expect(find.text(Strings.next), findsNothing);
    expect(find.text(Strings.previous), findsNothing);
  });

  testWidgets('pages are appended until the total is reached', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets));

    expect(offsets, <int>[0, 2, 4]);
    expect(find.text('Movie 0'), findsOneWidget);
    expect(find.text('Movie 4'), findsOneWidget);
  });

  testWidgets('the crumb keeps the real total, not the loaded count', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets));

    expect(
      find.text(Strings.countLabel(Strings.navigationMovies, _total)),
      findsOneWidget,
    );
  });

  testWidgets('placeholders share the grid, so a part-filled row has no hole', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets, stallAfterFirst: true), settle: false);

    final Finder grid = find.byType(CardGrid);
    expect(grid, findsOneWidget);
    expect(
      find.descendant(of: grid, matching: find.byType(CatalogCardTile)),
      findsNWidgets(_pageSize),
    );
    expect(
      find.descendant(of: grid, matching: find.byType(SkeletonCard)),
      findsWidgets,
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a failed page offers a retry and keeps what loaded', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(tester, _api(offsets, failAfterFirst: true));

    expect(find.text(Strings.couldNotLoadMore), findsOneWidget);
    expect(find.text(Strings.retry), findsOneWidget);
    expect(find.text('Movie 0'), findsOneWidget);
    expect(offsets, <int>[0, 2]);
  });

  testWidgets('the list lays out on every phone shape without overflowing', (
    WidgetTester tester,
  ) async {
    // Portrait and landscape, and the narrowest phone still sold, because an
    // overflow only shows up at the width that produces it.
    for (final Size window in <Size>[
      const Size(360, 640),
      const Size(393, 851),
      const Size(411, 914),
      const Size(851, 393),
    ]) {
      final List<int> offsets = <int>[];
      await _pump(tester, _api(offsets), window: window);

      expect(find.text('Movie 0'), findsOneWidget, reason: '$window');
      expect(tester.takeException(), isNull, reason: '$window');
    }
  });

  testWidgets('a loading phone list lays out without overflowing', (
    WidgetTester tester,
  ) async {
    final List<int> offsets = <int>[];
    await _pump(
      tester,
      _api(offsets, stallAfterFirst: true),
      settle: false,
      window: const Size(360, 800),
    );

    expect(find.byType(SkeletonCard), findsWidgets);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the library filter leaves out libraries that hold no movies', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _apiWithLibraries());

    await tester.tap(
      find.text('${Strings.libraryLabel}: ${Strings.filterAll}'),
    );
    await tester.pumpAndSettle();

    expect(find.text('Films'), findsOneWidget);
    expect(
      find.text('Shows'),
      findsNothing,
      reason: 'a tv library can only ever come back empty here',
    );
  });
}
