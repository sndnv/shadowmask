import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/section_heading.dart';
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

ApiClient _api(
  List<String> seen, {
  int hits = 0,
  List<Map<String, dynamic>>? items,
}) => ApiClient(
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
      final List<Map<String, dynamic>> results =
          items ??
          <Map<String, dynamic>>[for (int n = 0; n < hits; n++) _hit(n)];
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': results,
          'total': results.length,
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
  List<Map<String, dynamic>>? items,
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
            api: _api(seen, hits: hits, items: items),
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

  testWidgets('a lone person keeps its card size beside other results', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      query: 'thing',
      items: <Map<String, dynamic>>[
        <String, dynamic>{'type': 'person', 'id': 'p1', 'name': 'Pat Person'},
        <String, dynamic>{
          'type': 'episode',
          'id': 'e1',
          'season_id': 'se1',
          'number': 3,
          'title': 'An Episode',
        },
      ],
    );

    // The group was sized to exactly one card and the rail's two-column floor
    // then halved it, so a single person came out at 72px next to a 260px
    // episode. Measured, because the two look alike until you measure them.
    expect(
      tester
          .getSize(
            find.ancestor(
              of: find.text('Pat Person'),
              matching: find.byType(CatalogCardTile),
            ),
          )
          .width,
      kPosterCardWidth,
    );
  });

  testWidgets('results group by type, titles before people', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      query: 'thing',
      items: <Map<String, dynamic>>[
        <String, dynamic>{'type': 'person', 'id': 'p1', 'name': 'Pat Person'},
        <String, dynamic>{
          'type': 'episode',
          'id': 'e1',
          'season_id': 'se1',
          'number': 3,
          'title': 'An Episode',
        },
        <String, dynamic>{'type': 'series', 'id': 's1', 'title': 'A Series'},
        <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'A Movie'},
      ],
    );

    bool before(String first, String second) {
      final Offset a = tester.getTopLeft(
        find.text(Strings.countLabel(first, 1)),
      );
      final Offset b = tester.getTopLeft(
        find.text(Strings.countLabel(second, 1)),
      );
      return a.dy < b.dy || (a.dy == b.dy && a.dx < b.dx);
    }

    expect(before(Strings.typeMovie, Strings.typeSeries), isTrue);
    expect(before(Strings.typeSeries, Strings.typeEpisode), isTrue);
    expect(
      before(Strings.typeEpisode, Strings.typePerson),
      isTrue,
      reason:
          'someone hunting for a title should meet it before cast and '
          'episodes, which is the whole point of the split; blocks may share '
          'a row, so this is reading order rather than vertical order',
    );
  });

  testWidgets('thin blocks share a row instead of stacking', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      query: 'sta',
      items: <Map<String, dynamic>>[
        <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'A Movie'},
        <String, dynamic>{'type': 'series', 'id': 's1', 'title': 'A Series'},
        <String, dynamic>{'type': 'person', 'id': 'p1', 'name': 'Pat Person'},
      ],
    );

    double top(String noun) =>
        tester.getTopLeft(find.text(Strings.countLabel(noun, 1))).dy;
    double left(String noun) =>
        tester.getTopLeft(find.text(Strings.countLabel(noun, 1))).dx;

    expect(
      top(Strings.typeMovie),
      top(Strings.typeSeries),
      reason: 'one hit each leaves room to sit side by side',
    );
    expect(left(Strings.typeMovie), lessThan(left(Strings.typeSeries)));
    expect(
      left(Strings.typeSeries),
      lessThan(left(Strings.typePerson)),
      reason: 'packing must not disturb the reading order',
    );
  });

  testWidgets('a block wide enough to need the row keeps it', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      query: 'many',
      items: <Map<String, dynamic>>[
        for (int n = 0; n < 8; n++)
          <String, dynamic>{'type': 'movie', 'id': 'm$n', 'title': 'Movie $n'},
        <String, dynamic>{'type': 'person', 'id': 'p1', 'name': 'Pat Person'},
      ],
    );

    expect(
      tester.getTopLeft(find.text(Strings.countLabel(Strings.typeMovie, 8))).dy,
      lessThan(
        tester
            .getTopLeft(find.text(Strings.countLabel(Strings.typePerson, 1)))
            .dy,
      ),
      reason: 'a full grid takes the width, so the rail drops below it',
    );
  });

  testWidgets('a type with no matches gets no block at all', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      query: 'thing',
      items: <Map<String, dynamic>>[
        <String, dynamic>{'type': 'movie', 'id': 'm1', 'title': 'A Movie'},
      ],
    );

    expect(find.text(Strings.countLabel(Strings.typeMovie, 1)), findsOneWidget);
    expect(
      find.byType(CardRail),
      findsNothing,
      reason: 'an empty heading over an empty row reads as a broken section',
    );
    expect(find.byType(SectionHeading), findsOneWidget);
  });
}
