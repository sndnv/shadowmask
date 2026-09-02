import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/pages/admin/fetch_page.dart';
import 'package:shadowmask/util/library_labels.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _library() => <String, dynamic>{
  'id': 'ext',
  'name': 'Downloads',
  'kind': 'movie',
  'origin': 'external',
  'roots': <String>['/ext'],
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
        return http.Response(jsonEncode(<dynamic>[_library()]), 200);
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

  testWidgets('the year is not offered for a series download', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text(Strings.fieldYear), findsOneWidget);
    await tester.tap(find.text(libraryKindLabel(LibraryKind.movie)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(libraryKindLabel(LibraryKind.tv)).last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.fieldYear), findsNothing);
    expect(find.text(Strings.fieldSeason), findsOneWidget);
  });
}
