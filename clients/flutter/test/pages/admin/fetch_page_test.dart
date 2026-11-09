import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/fetch_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _library(String id, String name, String kind) =>
    <String, dynamic>{
      'id': id,
      'name': name,
      'kind': kind,
      'origin': 'external',
      'roots': <String>['/$id'],
      'watcher': 'manual',
      'metadata_sources': <String>[],
      'sort_articles': <String>[],
      'created_at': '2026-08-17T09:00:00Z',
      'updated_at': '2026-08-17T09:00:00Z',
    };

Future<List<Map<String, dynamic>>> _pump(WidgetTester tester) async {
  final List<Map<String, dynamic>> posted = <Map<String, dynamic>>[];
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      if (req.url.path == '/api/v1/users/self') {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': 'admin',
          }),
          200,
        );
      }
      if (req.url.path == '/api/v1/libraries') {
        return http.Response(
          jsonEncode(<dynamic>[
            _library('ext', 'Downloads', 'movie'),
            _library('shows', 'Shows', 'tv'),
          ]),
          200,
        );
      }
      if (req.url.path.endsWith('/fetch') && req.method == 'POST') {
        posted.add(jsonDecode(req.body) as Map<String, dynamic>);
        return http.Response('', 202);
      }
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 50,
        }),
        200,
      );
    }),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => FetchPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return posted;
}

Finder _field(String label) => find.descendant(
  of: find.ancestor(of: find.text(label), matching: find.byType(FieldLabel)),
  matching: find.byType(TextField),
);

Future<void> _fill(
  WidgetTester tester, {
  required String url,
  required String title,
  String? year,
}) async {
  await tester.enterText(_field(Strings.fieldSourceUrl), url);
  await tester.enterText(_field(Strings.fieldTitle), title);
  if (year != null) {
    await tester.enterText(_field(Strings.fieldYear), year);
  }
  await tester.pumpAndSettle();
}

Future<void> _submit(WidgetTester tester) async {
  await tester.tap(find.widgetWithText(FilledButton, Strings.fetch));
  await tester.pumpAndSettle();
}

Future<void> _switchLibrary(WidgetTester tester, String from, String to) async {
  await tester.tap(find.text(from).last);
  await tester.pumpAndSettle();
  await tester.tap(find.text(to).last);
  await tester.pumpAndSettle();
}

Future<void> _pickTvLibrary(WidgetTester tester) =>
    _switchLibrary(tester, 'Downloads (Movie)', 'Shows (TV)');

Future<void> _fillEpisode(WidgetTester tester) async {
  await tester.enterText(
    _field(Strings.fieldSourceUrl),
    'https://example.com/watch',
  );
  await tester.enterText(_field(Strings.fieldSeries), 'Great Show');
  await tester.enterText(_field(Strings.fieldSeason), '1');
  await tester.enterText(_field(Strings.fieldEpisode), '2');
  await tester.enterText(_field(Strings.fieldEpisodeTitle), 'Pilot');
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a movie fetch carries the year it was given', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _fill(
      tester,
      url: 'https://example.com/watch',
      title: 'The Matrix',
      year: '1999',
    );
    await _submit(tester);

    expect(posted, hasLength(1));
    expect(posted.single['year'], 1999);
    expect(posted.single['title'], 'The Matrix');
    expect(posted.single['library_id'], 'ext');
    expect(posted.single.containsKey('kind'), isFalse);
    expect(posted.single.containsKey('episode_title'), isFalse);
  });

  testWidgets('the library decides the kind, so there is no kind to pick', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.fieldKind), findsNothing);
    expect(find.text('Downloads (Movie)'), findsOneWidget);
  });

  testWidgets('a year that is not a year blocks the request', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _fill(
      tester,
      url: 'https://example.com/watch',
      title: 'The Matrix',
      year: 'nineteen',
    );
    await _submit(tester);

    expect(posted, isEmpty);
    expect(find.text(Strings.invalidYear), findsOneWidget);
  });

  testWidgets('a fetch with no year is still accepted', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _fill(tester, url: 'https://example.com/watch', title: 'Some Movie');
    await _submit(tester);

    expect(posted, hasLength(1));
    expect(posted.single.containsKey('year'), isFalse);
  });

  testWidgets('a TV library asks for the series, numbers and episode title', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.fieldYear), findsOneWidget);
    expect(find.text(Strings.fieldSeason), findsNothing);
    expect(_field(Strings.fieldEpisodeTitle), findsNothing);
    await _pickTvLibrary(tester);

    expect(find.text(Strings.fieldYear), findsNothing);
    expect(_field(Strings.fieldTitle), findsNothing);
    expect(_field(Strings.fieldSeries), findsOneWidget);
    expect(find.text(Strings.fieldSeason), findsOneWidget);
    expect(find.text(Strings.fieldEpisode), findsOneWidget);
    expect(_field(Strings.fieldEpisodeTitle), findsOneWidget);
  });

  testWidgets('a TV fetch carries its numbers and episode title and no kind', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _pickTvLibrary(tester);
    await _fillEpisode(tester);
    await _submit(tester);

    expect(posted, hasLength(1));
    expect(posted.single['library_id'], 'shows');
    expect(posted.single['title'], 'Great Show');
    expect(posted.single['season'], 1);
    expect(posted.single['episode'], 2);
    expect(posted.single['episode_title'], 'Pilot');
    expect(posted.single.containsKey('kind'), isFalse);
    expect(posted.single.containsKey('year'), isFalse);
  });

  testWidgets('a missing episode title alone blocks the request', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _pickTvLibrary(tester);
    await _fillEpisode(tester);
    await tester.enterText(_field(Strings.fieldEpisodeTitle), '   ');
    await tester.pumpAndSettle();
    await _submit(tester);

    expect(posted, isEmpty);
    expect(find.text(Strings.requiredEpisodeTitle), findsOneWidget);
    expect(find.text(Strings.requiredSeason), findsNothing);
    expect(find.text(Strings.requiredEpisode), findsNothing);
    expect(find.text(Strings.requiredSeries), findsNothing);
  });

  testWidgets('a year typed for a movie library is not sent for an episode', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await tester.enterText(_field(Strings.fieldYear), '1999');
    await _pickTvLibrary(tester);
    await _fillEpisode(tester);
    await _submit(tester);

    expect(posted, hasLength(1));
    expect(posted.single.containsKey('year'), isFalse);
  });

  testWidgets('a bad movie year does not outlive a switch to TV', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await tester.enterText(_field(Strings.fieldYear), 'nineteen');
    await _submit(tester);

    expect(find.text(Strings.invalidYear), findsOneWidget);
    expect(find.text(Strings.requiredTitle), findsOneWidget);
    expect(find.text(Strings.requiredSeries), findsNothing);

    await _pickTvLibrary(tester);
    await _switchLibrary(tester, 'Shows (TV)', 'Downloads (Movie)');

    expect(find.text(Strings.invalidYear), findsNothing);

    await _pickTvLibrary(tester);
    await _fillEpisode(tester);
    await _submit(tester);

    expect(posted, hasLength(1));
  });

  testWidgets('an empty series is named as such, and a switch clears errors', (
    WidgetTester tester,
  ) async {
    final List<Map<String, dynamic>> posted = await _pump(tester);

    await _pickTvLibrary(tester);
    await _submit(tester);

    expect(posted, isEmpty);
    expect(find.text(Strings.requiredSeries), findsOneWidget);
    expect(find.text(Strings.requiredTitle), findsNothing);
    expect(find.text(Strings.requiredSeason), findsOneWidget);
    expect(find.text(Strings.requiredEpisode), findsOneWidget);
    expect(find.text(Strings.requiredEpisodeTitle), findsOneWidget);

    await _switchLibrary(tester, 'Shows (TV)', 'Downloads (Movie)');
    await _pickTvLibrary(tester);

    expect(find.text(Strings.requiredSeries), findsNothing);
    expect(find.text(Strings.requiredSeason), findsNothing);
    expect(find.text(Strings.requiredEpisode), findsNothing);
    expect(find.text(Strings.requiredEpisodeTitle), findsNothing);
  });
}
