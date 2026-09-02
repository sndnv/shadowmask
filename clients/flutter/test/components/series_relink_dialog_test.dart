import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/series_relink_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

class _Capture {
  String? path;
  Map<String, dynamic>? target;
}

ApiClient _api(_Capture seen) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.url.path.endsWith('/relink')) {
      seen.path = req.url.path;
      seen.target =
          (jsonDecode(req.body) as Map<String, dynamic>)['target']
              as Map<String, dynamic>;
      return http.Response('', 202);
    }
    return http.Response('{}', 200);
  }),
);

Future<void> _open(WidgetTester tester, ApiClient api) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => showSeriesRelinkDialog(
              context,
              admin: AdminApi(api),
              seriesId: 'sh1',
              title: 'Stargate SG-1',
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

Future<void> _submit(WidgetTester tester, String id) async {
  await tester.enterText(find.byType(TextField), id);
  await tester.tap(find.widgetWithText(FilledButton, Strings.relink).first);
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(FilledButton, Strings.relink).last);
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a bare id is sent qualified to the tv endpoint', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen));

    await _submit(tester, '4629');

    expect(seen.path, '/api/v1/series/sh1/relink');
    expect(seen.target, <String, dynamic>{
      'kind': 'provider',
      'source': 'tmdb',
      'value': 'tv/4629',
      // ignore: require_trailing_commas
    }, reason: 'a bare id used to mean movie/4629, which was a different film');
    expect(find.text(Strings.toastRelinkQueued), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an imdb id is passed through as an imdb id', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen));

    await _submit(tester, 'tt0118480');

    expect(seen.target?['source'], 'imdb');
    expect(seen.target?['value'], 'tt0118480');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an empty id is rejected inline without a request', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen));

    await tester.tap(find.widgetWithText(FilledButton, Strings.relink));
    await tester.pumpAndSettle();

    expect(find.text(Strings.requiredTmdbId), findsOneWidget);
    expect(seen.path, isNull);
  });

  testWidgets('the confirmation names the show and says what is replaced', (
    WidgetTester tester,
  ) async {
    final _Capture seen = _Capture();
    await _open(tester, _api(seen));

    await tester.enterText(find.byType(TextField), '4629');
    await tester.tap(find.widgetWithText(FilledButton, Strings.relink));
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.confirmRelinkSeries('Stargate SG-1')),
      findsOneWidget,
    );
    expect(seen.path, isNull, reason: 'nothing is sent until it is confirmed');
  });
}
