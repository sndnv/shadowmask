import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/refresh_metadata_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

class _Capture {
  String? path;
  String? body;
}

Future<void> _open(
  WidgetTester tester,
  ApiClient api, {
  required TitleKind kind,
  required String id,
  bool manuallyEdited = false,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => showRefreshMetadataDialog(
              context,
              admin: AdminApi(api),
              kind: kind,
              id: id,
              manuallyEdited: manuallyEdited,
            ),
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
}

ApiClient _api(_Capture seen) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.url.path.endsWith('/refresh')) {
      seen.path = req.url.path;
      seen.body = req.body;
      return http.Response('', 202);
    }
    return http.Response('{}', 200);
  }),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('refresh is a plain confirm with no id to supply', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen), kind: TitleKind.movie, id: 'm1');

    expect(find.text(Strings.confirmRefreshBody), findsOneWidget);
    expect(
      find.byType(TextField),
      findsNothing,
      reason: 'the Match ID field implied an id was needed; it never was',
    );
    expect(seen.path, isNull);

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(seen.path, '/api/v1/movies/m1/refresh');
    expect(seen.body, anyOf(isNull, isEmpty));
    expect(find.text(Strings.toastMetadataQueued), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a series refreshes on the series route', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen), kind: TitleKind.series, id: 's1');

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(seen.path, '/api/v1/series/s1/refresh');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an unedited title is never offered the destructive option', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen), kind: TitleKind.movie, id: 'm1');

    expect(find.text(Strings.forceRefresh), findsNothing);
    expect(find.text(Strings.refreshEditedNotice), findsNothing);
  });

  testWidgets('an edited title offers both choices, keeping edits by default', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(
      tester,
      _api(seen),
      kind: TitleKind.movie,
      id: 'm1',
      manuallyEdited: true,
    );

    expect(find.text(Strings.refreshEditedNotice), findsOneWidget);
    expect(find.text(Strings.keepEdits), findsOneWidget);
    expect(find.text(Strings.forceRefresh), findsOneWidget);
    expect(
      find.widgetWithText(FilledButton, Strings.keepEdits),
      findsOneWidget,
      reason: 'keeping edits is the safe default and gets the primary button',
    );

    await tester.tap(find.widgetWithText(FilledButton, Strings.keepEdits));
    await tester.pumpAndSettle();

    expect(seen.path, '/api/v1/movies/m1/refresh');
    expect(seen.body, anyOf(isNull, isEmpty));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('choosing to discard sends force', (WidgetTester tester) async {
    final _Capture seen = _Capture();
    await _open(
      tester,
      _api(seen),
      kind: TitleKind.movie,
      id: 'm1',
      manuallyEdited: true,
    );

    await tester.tap(find.widgetWithText(TextButton, Strings.forceRefresh));
    await tester.pumpAndSettle();

    expect(seen.body, contains('"force":true'));

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a series names the episodes it would also discard', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(
      tester,
      _api(seen),
      kind: TitleKind.series,
      id: 's1',
      manuallyEdited: true,
    );

    expect(find.text(Strings.refreshChoiceDiscardSeries), findsOneWidget);
    expect(find.text(Strings.refreshChoiceDiscardMovie), findsNothing);
  });

  testWidgets('closing the choice dialog refreshes nothing', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(
      tester,
      _api(seen),
      kind: TitleKind.movie,
      id: 'm1',
      manuallyEdited: true,
    );

    await tester.tap(find.byTooltip(Strings.close));
    await tester.pumpAndSettle();

    expect(seen.path, isNull);
  });

  testWidgets('a failed refresh reports as a toast, not inline', (
    WidgetTester tester,
  ) async {
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request req) async => req.url.path.endsWith('/refresh')
            ? http.Response('nope', 500)
            : http.Response('{}', 200),
      ),
    );
    await _open(tester, api, kind: TitleKind.movie, id: 'm1');

    await tester.tap(
      find.widgetWithText(FilledButton, Strings.refreshMetadata),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining(Strings.errorRefresh), findsOneWidget);
    expect(find.text(Strings.confirmRefreshBody), findsNothing);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });
}
