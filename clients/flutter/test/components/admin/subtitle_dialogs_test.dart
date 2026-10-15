import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/admin/subtitle_dialogs.dart';
import 'package:shadowmask/components/language_dropdown.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/start_ellipsis_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shared_preferences/shared_preferences.dart';

SubtitleFile _held(String fileId) => SubtitleFile(
  id: 'opensubtitles:v1:$fileId',
  language: 'en',
  format: SubtitleFormat.srt,
  source: SubtitleSource.openSubtitles,
);

Map<String, dynamic> _candidate(
  String fileId,
  String release, {
  int? downloads,
}) => <String, dynamic>{
  'file_id': fileId,
  'language': 'en',
  'release_name': release,
  'format': 'srt',
  'download_count': ?downloads,
};

enum _Close { button, escape, outside }

Future<void> _close(WidgetTester tester, _Close how) async {
  switch (how) {
    case _Close.button:
      await tester.tap(find.byTooltip(Strings.close));
    case _Close.escape:
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    case _Close.outside:
      await tester.tapAt(const Offset(4, 4));
  }
  await tester.pumpAndSettle();
}

const String _kLongRelease =
    'Paper.Skies.2019.REMASTERED.1080p.BluRay.x264.DTS-HD.MA.5.1-SOMEGROUP';

Future<List<http.Request>> _open(
  WidgetTester tester, {
  required List<SubtitleFile> existing,
  Completer<http.Response>? download,
  List<bool>? results,
  String? path,
  String freshRelease = 'Fresh.Release',
}) async {
  final List<http.Request> seen = <http.Request>[];
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(req);
      if (req.url.path.endsWith('/subtitles/search')) {
        return http.Response(
          jsonEncode(<dynamic>[
            _candidate('42', 'Held.Release', downloads: 1234),
            _candidate('99', freshRelease),
          ]),
          200,
        );
      }
      if (req.url.path.endsWith('/subtitles/download')) {
        return download?.future ?? http.Response('{}', 201);
      }
      return http.Response('', 204);
    }),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () async {
              final bool changed = await showSubtitleSearch(
                context,
                admin: AdminApi(api),
                versionId: 'v1',
                existing: existing,
                path: path,
              );
              results?.add(changed);
            },
            child: const Text('open'),
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
      expect(find.text(Strings.toastDownloaded), findsOneWidget);
    },
  );

  testWidgets('a file the server already held reads as already added', (
    WidgetTester tester,
  ) async {
    final Completer<http.Response> gate = Completer<http.Response>()
      ..complete(http.Response('{}', 200));
    await _open(tester, existing: <SubtitleFile>[], download: gate);

    await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
    await tester.pumpAndSettle();

    expect(find.widgetWithText(TextButton, Strings.downloaded), findsOneWidget);
    expect(find.text(Strings.toastAlreadyAdded), findsOneWidget);
    expect(find.text(Strings.toastDownloaded), findsNothing);
  });

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

  testWidgets('a picked language reaches the search as its two-letter code', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _open(
      tester,
      existing: <SubtitleFile>[],
    );

    await tester.tap(find.text(Strings.optionAnyLanguage).first);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('English').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('English').last);
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, Strings.searchSubtitles),
    );
    await tester.pumpAndSettle();

    final http.Request search = seen.lastWhere(
      (http.Request r) => r.url.path.endsWith('/subtitles/search'),
    );
    expect(search.url.queryParameters['language'], 'en');
  });

  final ValueVariant<_Close> closes = ValueVariant<_Close>(
    _Close.values.toSet(),
  );

  testWidgets('a download is reported however the dialog closes', (
    WidgetTester tester,
  ) async {
    final List<bool> results = <bool>[];
    await _open(tester, existing: <SubtitleFile>[], results: results);

    await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
    await tester.pumpAndSettle();
    await _close(tester, closes.currentValue!);

    expect(results, <bool>[true]);
  }, variant: closes);

  testWidgets('a download still running at close is reported once it lands', (
    WidgetTester tester,
  ) async {
    final Completer<http.Response> gate = Completer<http.Response>();
    final List<bool> results = <bool>[];
    await _open(
      tester,
      existing: <SubtitleFile>[],
      download: gate,
      results: results,
    );

    await tester.tap(find.widgetWithText(TextButton, Strings.download).first);
    await tester.pump();
    await _close(tester, _Close.escape);

    expect(results, isEmpty, reason: 'the caller waits for the download');

    gate.complete(http.Response('', 204));
    await tester.pumpAndSettle();

    expect(results, <bool>[true]);
  });

  testWidgets('closing without a download reports no change', (
    WidgetTester tester,
  ) async {
    final List<bool> results = <bool>[];
    await _open(tester, existing: <SubtitleFile>[], results: results);

    await _close(tester, closes.currentValue!);

    expect(results, <bool>[false]);
  }, variant: closes);

  testWidgets('a download count leads the details, compact', (
    WidgetTester tester,
  ) async {
    await _open(tester, existing: <SubtitleFile>[_held('42')]);

    expect(find.byType(OutlinePill), findsOneWidget);
    expect(find.text('1.23K'), findsOneWidget);
    expect(find.byTooltip('1,234 downloads'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('English · srt').first).dx,
      tester.getTopRight(find.byType(OutlinePill)).dx + Space.s2,
    );
    expect(
      tester.getTopLeft(find.text('English · srt').last).dx,
      tester.getTopLeft(find.text('Fresh.Release')).dx,
      reason: 'a result with no count starts with its details',
    );
    expect(
      tester.getTopRight(find.text(Strings.downloaded)).dx,
      tester
              .getTopRight(find.widgetWithText(TextButton, Strings.downloaded))
              .dx -
          (kButtonPaddingX - Space.s2),
      reason: 'the label sits where the row padding put it',
    );
  });

  final ValueVariant<Size> screens = ValueVariant<Size>(<Size>{
    const Size(800, 600),
    const Size(360, 740),
  });

  testWidgets('a result reads as its name above its details and button', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = screens.currentValue!;
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _open(tester, existing: <SubtitleFile>[_held('42')]);

    final Rect name = tester.getRect(find.text('Held.Release'));
    final Rect details = tester.getRect(find.text('English · srt').first);
    final Rect pill = tester.getRect(find.byType(OutlinePill));
    final Rect button = tester.getRect(
      find.widgetWithText(TextButton, Strings.downloaded),
    );
    final double end = tester
        .getRect(find.widgetWithText(FilledButton, Strings.searchSubtitles))
        .right;

    expect(button.top, greaterThanOrEqualTo(name.bottom));
    expect(pill.left, name.left);
    expect(pill.center.dy, moreOrLessEquals(button.center.dy, epsilon: 1));
    expect(button.right, end, reason: 'the button sits flush right');
    expect(details.right, lessThan(button.left));
  }, variant: screens);

  final ValueVariant<double> fonts = ValueVariant<double>(<double>{0.85, 1.15});

  testWidgets('the query, language and button match at any font size', (
    WidgetTester tester,
  ) async {
    tester.platformDispatcher.textScaleFactorTestValue = fonts.currentValue!;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    await _open(tester, existing: <SubtitleFile>[]);

    final Rect query = tester.getRect(find.byType(TextField));
    final Rect language = tester.getRect(find.byType(LanguageDropdown));
    final Rect button = tester.getRect(
      find.widgetWithText(FilledButton, Strings.searchSubtitles),
    );
    expect(query.top, language.top);
    expect(query.bottom, language.bottom);
    expect(query.top, button.top);
    expect(query.bottom, button.bottom);
  }, variant: fonts);

  testWidgets('a long name is cut and read in full on a long press', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 740);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _open(
      tester,
      existing: <SubtitleFile>[],
      freshRelease: _kLongRelease,
    );

    final Finder cut = find.textContaining('…');
    expect(cut, findsOneWidget);
    expect(find.text(_kLongRelease), findsNothing);

    await tester.longPress(cut);
    await tester.pumpAndSettle();

    expect(find.text(_kLongRelease), findsOneWidget);
  });

  testWidgets('the search names its version by file name, above the query', (
    WidgetTester tester,
  ) async {
    await _open(
      tester,
      existing: <SubtitleFile>[],
      path: '/media/movies/Paper Skies (2019)/Paper.Skies.2019.1080p.mkv',
    );

    final Finder name = find.text('Paper.Skies.2019.1080p');
    expect(name, findsOneWidget);
    expect(
      tester.getRect(name).bottom,
      lessThanOrEqualTo(tester.getRect(find.byType(LabelledTextField)).top),
    );
    expect(find.byType(StartEllipsisText), findsNWidgets(3));
  });

  testWidgets('a version with no path names nothing above the query', (
    WidgetTester tester,
  ) async {
    await _open(tester, existing: <SubtitleFile>[]);

    expect(
      find.byType(StartEllipsisText),
      findsNWidgets(2),
      reason: 'only the two results are named',
    );
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
