import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/history_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _entry(String id, String? at, int playCount) =>
    <String, dynamic>{
      'user_id': 'u1',
      'title': <String, dynamic>{'type': 'movie', 'id': id},
      'watched': true,
      'play_count': playCount,
      'last_watched_at': at,
      'completed': true,
    };

Future<({List<String> deleted, List<String> paths})> _pump(
  WidgetTester tester, {
  List<Map<String, dynamic>>? items,
}) async {
  final List<String> deleted = <String>[];
  final List<String> paths = <String>[];
  tester.view.physicalSize = const Size(1400, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      paths.add(req.url.path);
      if (req.method == 'DELETE') {
        deleted.add(req.url.path);
        return http.Response('', 204);
      }
      if (req.url.path == '/api/v1/users/u1/history') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'items': items ?? <Map<String, dynamic>>[],
            'total': (items ?? const <Map<String, dynamic>>[]).length,
            'offset': 0,
            'limit': 50,
          }),
          200,
        );
      }
      if (req.url.path == '/api/v1/titles/batch') {
        return http.Response(
          jsonEncode(<dynamic>[
            for (final Map<String, dynamic> i
                in items ?? const <Map<String, dynamic>>[])
              <String, dynamic>{
                'type': 'movie',
                'id': (i['title'] as Map<String, dynamic>)['id'],
                'title': 'Movie ${(i['title'] as Map<String, dynamic>)['id']}',
                'year': 1999,
              },
          ]),
          200,
        );
      }
      return http.Response(jsonEncode(<dynamic>[]), 200);
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: ToastHost(
            child: SingleChildScrollView(
              child: HistoryBlock(api: CatalogApi(api), userId: 'u1'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return (deleted: deleted, paths: paths);
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a single view reads as watched on a date', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      items: <Map<String, dynamic>>[_entry('m1', '2026-08-21T09:34:11Z', 1)],
    );

    expect(find.textContaining('Watched'), findsOneWidget);
    expect(find.textContaining('seen'), findsNothing);
  });

  testWidgets('repeat views are counted on a badge, not in the caption', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      items: <Map<String, dynamic>>[_entry('m1', '2026-08-21T09:34:11Z', 3)],
    );

    expect(find.text(Strings.timesWatched(3)), findsOneWidget);
    expect(
      find.textContaining('3'),
      findsOneWidget,
      reason: 'the count belongs on the artwork; the caption is the date alone',
    );
  });

  testWidgets('a single view gets no badge', (WidgetTester tester) async {
    await _pump(
      tester,
      items: <Map<String, dynamic>>[_entry('m1', '2026-08-21T09:34:11Z', 1)],
    );

    expect(find.text(Strings.timesWatched(1)), findsNothing);
  });

  testWidgets('nothing is ever captioned as in progress', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      items: <Map<String, dynamic>>[
        _entry('m1', null, 1),
        _entry('m2', '2026-08-21T09:34:11Z', 1),
      ],
    );

    expect(find.textContaining('In progress'), findsNothing);
  });

  testWidgets('a card can be removed from history on its own', (
    WidgetTester tester,
  ) async {
    final ({List<String> deleted, List<String> paths}) run = await _pump(
      tester,
      items: <Map<String, dynamic>>[_entry('m1', '2026-08-21T09:34:11Z', 1)],
    );

    await tester.tap(find.byTooltip(Strings.removeFromHistory));
    await tester.pumpAndSettle();

    expect(run.deleted, <String>['/api/v1/users/u1/history/m1']);

    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });

  testWidgets('clearing asks first and then wipes the whole list', (
    WidgetTester tester,
  ) async {
    final ({List<String> deleted, List<String> paths}) run = await _pump(
      tester,
      items: <Map<String, dynamic>>[_entry('m1', '2026-08-21T09:34:11Z', 1)],
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.clearHistory));
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmClearHistory), findsOneWidget);
    expect(run.deleted, isEmpty);

    await tester.tap(find.widgetWithText(FilledButton, Strings.clearHistory));
    await tester.pumpAndSettle();

    expect(run.deleted, <String>['/api/v1/users/u1/history']);

    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });
}
