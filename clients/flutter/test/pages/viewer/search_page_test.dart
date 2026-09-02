import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/viewer/search_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _hit(int n) => <String, dynamic>{
  'type': 'movie',
  'id': 'm$n',
  'title': 'Movie $n',
  'year': 2000 + n,
};

ApiClient _api(List<String> seen, {int hits = 0}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    seen.add('${req.url.path}?${req.url.query}');
    if (req.url.path.endsWith('/state/batch')) {
      final List<dynamic> titles =
          (jsonDecode(req.body) as Map<String, dynamic>)['titles']
              as List<dynamic>;
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          for (final dynamic t in titles)
            <String, dynamic>{
              'title': t,
              'watched': false,
              'progress_percent': 0,
            },
        ]),
        200,
      );
    }
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'user',
        }),
        200,
      );
    }
    if (req.url.path == '/api/v1/libraries') {
      // Without one the empty state reports "no libraries" instead of
      // "no results", which is a different sentence for a different problem.
      return http.Response(
        jsonEncode(<Map<String, dynamic>>[
          <String, dynamic>{'id': 'l1', 'name': 'Films', 'kind': 'movie'},
        ]),
        200,
      );
    }
    if (req.url.path == '/api/v1/search') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': <Map<String, dynamic>>[
            for (int n = 0; n < hits; n++) _hit(n),
          ],
          'total': hits,
          'offset': 0,
          'limit': 24,
        }),
        200,
      );
    }
    return http.Response(jsonEncode(<dynamic>[]), 200);
  }),
);

Future<List<String>> _pump(
  WidgetTester tester, {
  int hits = 0,
  String? query,
}) async {
  final List<String> seen = <String>[];
  tester.view.physicalSize = const Size(1400, 1000);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => SearchPage(
            api: _api(seen, hits: hits),
            query: query,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return seen;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('an unused search says what it can find', (
    WidgetTester tester,
  ) async {
    final List<String> seen = await _pump(tester);

    // The page used to be a text box above a zero-height box, which reads as
    // a page that failed rather than one waiting for input.
    expect(find.text(Strings.searchOpening), findsOneWidget);
    expect(
      seen.where((String s) => s.startsWith('/api/v1/search')),
      isEmpty,
      reason: 'nothing to search for yet',
    );
  });

  testWidgets('the field is named for a screen reader', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester);

    expect(
      tester.getSemantics(find.byType(TextField)).label,
      contains(Strings.searchFieldLabel),
    );

    semantics.dispose();
  });

  testWidgets('a query drops the opening line and shows what it found', (
    WidgetTester tester,
  ) async {
    final List<String> seen = await _pump(tester, hits: 3, query: 'blade');

    expect(seen.where((String s) => s.contains('q=blade')), isNotEmpty);
    expect(find.text(Strings.searchOpening), findsNothing);
    expect(find.byType(CatalogCardTile), findsNWidgets(3));
  });

  testWidgets('a query with no hits explains itself', (
    WidgetTester tester,
  ) async {
    await _pump(tester, query: 'nothing');

    expect(find.text(Strings.searchOpening), findsNothing);
    expect(find.text(Strings.noResultsFound), findsOneWidget);
  });

  testWidgets('a blank query is not a search', (WidgetTester tester) async {
    final List<String> seen = await _pump(tester, query: '   ');

    expect(find.text(Strings.searchOpening), findsOneWidget);
    expect(seen.where((String s) => s.startsWith('/api/v1/search')), isEmpty);
  });
}
