import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/pages/admin/library_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _library() => <String, dynamic>{
  'id': 'lib1',
  'name': 'Films',
  'kind': 'movie',
  'origin': 'local',
  'roots': <String>['/media/films'],
  'watcher': 'manual',
  'metadata_sources': <String>[],
  'sort_articles': <String>['the'],
  'created_at': '2026-08-17T09:00:00Z',
  'updated_at': '2026-08-17T09:00:00Z',
};

Future<void> _pump(
  WidgetTester tester,
  ApiClient api, {
  double width = 1600,
}) async {
  tester.view.physicalSize = Size(width, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (_) => MaterialPageRoute<void>(
          builder: (_) => LibraryPage(api: api, libraryId: 'lib1'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

http.Response _detailRoute(http.Request req) {
  final RegExp detail = RegExp(r'^/api/v1/libraries/[^/]*$');
  if (detail.hasMatch(req.url.path)) {
    return http.Response(jsonEncode(_library()), 200);
  }
  if (req.url.path == '/api/v1/libraries/lib1/scan') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'library_id': 'lib1',
        'status': 'idle',
        'progress': 0,
        'last_scanned_at': '2026-08-17T09:24:11Z',
      }),
      200,
    );
  }
  return http.Response(jsonEncode(<dynamic>[]), 200);
}

ApiClient _api(http.Response Function(http.Request req) route) => ApiClient(
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
    return route(req);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the detail page carries its actions on the library card', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute));

    final Finder card = find.ancestor(
      of: find.text('Films'),
      matching: find.byType(SectionBlock),
    );
    for (final String label in <String>[
      Strings.delete,
      Strings.refreshMetadata,
      Strings.edit,
    ]) {
      expect(
        find.descendant(
          of: card,
          matching: find.widgetWithText(OutlinedButton, label),
        ),
        findsOneWidget,
        reason: 'missing the $label action',
      );
    }
    expect(find.byType(PageActions), findsNothing);
  });

  testWidgets('the three card actions still fit a narrow window', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute), width: 900);

    expect(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
      findsOneWidget,
    );
  });

  testWidgets('Edit library opens the shared form prefilled', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(_detailRoute));

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.edit));
    await tester.pumpAndSettle();

    expect(find.text(Strings.editLibrary), findsWidgets);
    expect(find.widgetWithText(TextField, 'Films'), findsOneWidget);
  });

  testWidgets('Refresh metadata confirms, then posts and does not scan', (
    WidgetTester tester,
  ) async {
    final List<String> posted = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'POST') {
          posted.add(req.url.path);
          return http.Response('', 202);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(
      find.widgetWithText(OutlinedButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();
    expect(find.text(Strings.confirmRefreshLibraryBody), findsOneWidget);

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(posted, <String>['/api/v1/libraries/lib1/refresh-metadata']);
    expect(find.text(Strings.toastMetadataQueued), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('deleting from the detail page returns to the library list', (
    WidgetTester tester,
  ) async {
    bool deleted = false;
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.method == 'DELETE') {
          deleted = true;
          return http.Response('', 204);
        }
        return _detailRoute(req);
      }),
    );

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.delete));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.delete).last);
    await tester.pumpAndSettle();

    expect(deleted, isTrue);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the scan block refreshes its state without queueing a scan', (
    WidgetTester tester,
  ) async {
    final List<String> requested = <String>[];
    await _pump(
      tester,
      _api((http.Request req) {
        requested.add('${req.method} ${req.url.path}');
        return _detailRoute(req);
      }),
    );

    const String scan = 'GET /api/v1/libraries/lib1/scan';
    expect(requested.where((String r) => r == scan), hasLength(1));

    final Finder block = find.ancestor(
      of: find.text(Strings.scanHeading),
      matching: find.byType(SectionBlock),
    );
    await tester.tap(
      find.descendant(
        of: block,
        matching: find.widgetWithText(OutlinedButton, Strings.refresh),
      ),
    );
    await tester.pumpAndSettle();

    expect(requested.where((String r) => r == scan), hasLength(2));
    expect(
      requested.where((String r) => r == 'POST /api/v1/libraries/lib1/scan'),
      isEmpty,
      reason: 'refresh must not queue a scan',
    );
  });

  testWidgets('the last scan reads as a date rather than as sent', (
    WidgetTester tester,
  ) async {
    // The server sends ISO 8601 and the client owns the rendering, the same
    // way it owns the words.
    await _pump(tester, _api(_detailRoute));

    expect(find.textContaining('2026-08-17T09:24'), findsNothing);
    expect(find.text(dateTimeText('2026-08-17T09:24:11Z')!), findsOneWidget);
  });

  testWidgets('a duplicate offers Dismiss and nothing else', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/duplicates') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <Map<String, dynamic>>[
                <String, dynamic>{
                  'id': 'dup1',
                  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                  'paths': <String>['/media/a.mkv', '/media/b.mkv'],
                },
              ],
              'total': 1,
              'offset': 0,
              'limit': 50,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    final Finder block = find.ancestor(
      of: find.text(Strings.duplicatesHeading),
      matching: find.byType(SectionBlock),
    );
    expect(
      find.descendant(
        of: block,
        matching: find.widgetWithText(TextButton, Strings.dismiss),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: block,
        matching: find.widgetWithText(TextButton, Strings.resolve),
      ),
      findsNothing,
      reason: 'Resolve did the same thing as Dismiss and was removed',
    );
  });

  testWidgets('the versions heading counts every version, not the page', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api((http.Request req) {
        if (req.url.path == '/api/v1/libraries/lib1/versions') {
          return http.Response(
            jsonEncode(<String, dynamic>{
              'items': <Map<String, dynamic>>[
                <String, dynamic>{
                  'id': 'v1',
                  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
                  'library_id': 'lib1',
                  'quality': 'fhd',
                  'container': 'mkv',
                  'path': '/media/a.mkv',
                  'size_bytes': 1,
                  'available': true,
                },
              ],
              'total': 12,
              'offset': 0,
              'limit': 1,
            }),
            200,
          );
        }
        return _detailRoute(req);
      }),
    );

    expect(find.text(Strings.versionsWithCount(12)), findsOneWidget);
    expect(find.text(Strings.versionsWithCount(1)), findsNothing);
  });
}
