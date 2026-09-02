import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/subtitle_dialogs.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

SubtitleFile _held(String fileId) => SubtitleFile(
  id: 'opensubtitles:v1:$fileId',
  language: 'en',
  format: SubtitleFormat.srt,
  source: SubtitleSource.openSubtitles,
);

Map<String, dynamic> _candidate(String fileId, String release) =>
    <String, dynamic>{
      'file_id': fileId,
      'language': 'en',
      'release_name': release,
      'format': 'srt',
      'download_count': 10,
    };

Future<List<http.Request>> _open(
  WidgetTester tester, {
  required List<SubtitleFile> existing,
  Completer<http.Response>? download,
}) async {
  final List<http.Request> seen = <http.Request>[];
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(req);
      if (req.url.path.endsWith('/subtitles/search')) {
        return http.Response(
          jsonEncode(<dynamic>[
            _candidate('42', 'Held.Release'),
            _candidate('99', 'Fresh.Release'),
          ]),
          200,
        );
      }
      if (download != null && req.url.path.endsWith('/subtitles/download')) {
        return download.future;
      }
      return http.Response('', 204);
    }),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: ToastHost(
        child: Scaffold(
          body: Builder(
            builder: (BuildContext context) => TextButton(
              onPressed: () => showSubtitleSearch(
                context,
                admin: AdminApi(api),
                versionId: 'v1',
                existing: existing,
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  await tester.tap(find.widgetWithText(FilledButton, Strings.searchSubtitles));
  await tester.pumpAndSettle();
  return seen;
}

Future<void> _openText(
  WidgetTester tester, {
  Map<String, dynamic>? body,
  int status = 200,
}) async {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient(
      (http.Request req) async =>
          http.Response(jsonEncode(body ?? <String, dynamic>{}), status),
    ),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: ToastHost(
        child: Scaffold(
          body: Builder(
            builder: (BuildContext context) => TextButton(
              onPressed: () => showSubtitleText(
                context,
                admin: AdminApi(api),
                versionId: 'v1',
                sub: _held('42'),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a candidate the version already holds cannot be downloaded', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _open(
      tester,
      existing: <SubtitleFile>[_held('42')],
    );

    final TextButton downloaded = tester.widget<TextButton>(
      find.widgetWithText(TextButton, Strings.downloaded),
    );
    expect(downloaded.onPressed, isNull);
    expect(find.widgetWithText(TextButton, Strings.download), findsOneWidget);
    expect(
      seen.where((http.Request r) => r.method == 'POST'),
      isEmpty,
      reason: 'the held candidate must not reach the provider',
    );
  });

  testWidgets(
    'a candidate the version lacks downloads and then reads as held',
    (WidgetTester tester) async {
      final List<http.Request> seen = await _open(
        tester,
        existing: <SubtitleFile>[],
      );

      expect(find.widgetWithText(TextButton, Strings.downloaded), findsNothing);
      await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
      await tester.pumpAndSettle();

      expect(
        seen
            .where((http.Request r) => r.method == 'POST')
            .map((http.Request r) => jsonDecode(r.body)['file_id']),
        <String>['42'],
      );
      expect(
        find.widgetWithText(TextButton, Strings.downloaded),
        findsOneWidget,
      );
      expect(find.widgetWithText(TextButton, Strings.download), findsOneWidget);
    },
  );

  testWidgets('a download in flight shows progress and takes no second tap', (
    WidgetTester tester,
  ) async {
    final Completer<http.Response> gate = Completer<http.Response>();
    final List<http.Request> seen = await _open(
      tester,
      existing: <SubtitleFile>[],
      download: gate,
    );

    await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(
      find.widgetWithText(TextButton, Strings.download),
      findsOneWidget,
      reason: 'only the row being downloaded loses its button',
    );
    final TextButton pending = tester.widget<TextButton>(
      find.ancestor(
        of: find.byType(CircularProgressIndicator),
        matching: find.byType(TextButton),
      ),
    );
    expect(pending.onPressed, isNull);

    await tester.tap(
      find.byType(CircularProgressIndicator),
      warnIfMissed: false,
    );
    await tester.pump();
    expect(
      seen.where((http.Request r) => r.method == 'POST'),
      hasLength(1),
      reason: 'the disabled control must not fire a second request',
    );

    gate.complete(http.Response('', 204));
    await tester.pumpAndSettle();

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.widgetWithText(TextButton, Strings.downloaded), findsOneWidget);
  });

  testWidgets('a failed download returns the row to Download', (
    WidgetTester tester,
  ) async {
    final Completer<http.Response> gate = Completer<http.Response>();
    await _open(tester, existing: <SubtitleFile>[], download: gate);

    await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
    await tester.pump();
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    gate.complete(http.Response('nope', 502));
    await tester.pumpAndSettle();

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.widgetWithText(TextButton, Strings.downloaded), findsNothing);
    expect(find.widgetWithText(TextButton, Strings.download), findsNWidgets(2));
  });

  testWidgets('a subtitle with no text says so rather than reading blank', (
    WidgetTester tester,
  ) async {
    await _openText(tester, body: <String, dynamic>{'content': '   \n'});

    expect(find.text(Strings.emptySubtitleText), findsOneWidget);
  });

  testWidgets('a subtitle that cannot be read reports the failure', (
    WidgetTester tester,
  ) async {
    await _openText(tester, status: 500);

    expect(find.textContaining(Strings.errorSubtitleText), findsOneWidget);
  });
}
