import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/absolute_url.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _json(String id, {bool available = true}) =>
    <String, dynamic>{
      'id': id,
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'l1',
      'quality': 'fhd',
      'container': 'mkv',
      'size_bytes': 8 * 1048576,
      'duration_ms': 7200000,
      'available': available,
    };

ApiClient _api(
  List<String> seen, {
  bool failDownload = false,
  int resumeMs = 0,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    seen.add('${req.method} ${req.url.path}');
    if (req.url.path.contains('/progress/')) {
      return http.Response(
        jsonEncode(<String, dynamic>{'position_ms': resumeMs}),
        200,
      );
    }
    if (req.url.path.endsWith('/download')) {
      if (failDownload) {
        return http.Response('{"message":"nope"}', 500);
      }
      return http.Response(
        jsonEncode(<String, dynamic>{
          'url': '/download/tok-123',
          'filename': 'Big Buck Bunny.mkv',
          'size_bytes': 8388608,
          'expires_at': '2026-08-24T12:00:00Z',
        }),
        200,
      );
    }
    if (req.url.path.contains('/versions/')) {
      return http.Response(jsonEncode(_json('v1')..remove('available')), 200);
    }
    return http.Response('{}', 200);
  }),
);

Future<void> _pump(
  WidgetTester tester,
  ApiClient api, {
  bool available = true,
}) async {
  tester.view.physicalSize = const Size(1200, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ToastHost(
          child: Scaffold(
            body: SingleChildScrollView(
              child: VersionPicker(
                catalog: CatalogApi(api),
                playback: PlaybackApi(api),
                userId: 'u1',
                versions: <Version>[
                  Version.fromJson(_json('v1', available: available)),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Finder _more() => find.byTooltip(Strings.columnActions);
Finder _chevron() => find.byTooltip(Strings.fullVersionDetails);

Future<void> _expand(WidgetTester tester) async {
  await tester.tap(_chevron());
  await tester.pumpAndSettle();
}

Future<void> _openMore(WidgetTester tester) async {
  await _expand(tester);
  await tester.tap(_more());
  await tester.pumpAndSettle();
}

Future<void> _download(WidgetTester tester) async {
  await _openMore(tester);
  await tester.tap(find.text(Strings.downloadVersion));
  await tester.pumpAndSettle();
}

void main() {
  final DownloadStarter platform = startDownload;
  final List<String> started = <String>[];

  setUp(() {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    started.clear();
    startDownload = (String url, String filename) async {
      started.add(url);
      return true;
    };
  });
  tearDown(() => startDownload = platform);

  test('an absolute url survives a base with or without a trailing slash', () {
    expect(absoluteUrl('http://h', '/download/t'), 'http://h/download/t');
    expect(absoluteUrl('http://h/', '/download/t'), 'http://h/download/t');
    expect(absoluteUrl('http://h', 'download/t'), 'http://h/download/t');
    expect(absoluteUrl('http://h/', 'download/t'), 'http://h/download/t');
  });

  testWidgets('a closed row shows its text, CC count and chevron only', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    final double text = tester.getCenter(find.byType(Text).first).dx;
    final double captions = tester
        .getCenter(find.byIcon(Icons.closed_caption_outlined))
        .dx;
    final double chevron = tester.getCenter(_chevron()).dx;

    expect(
      <bool>[text < captions, captions < chevron],
      <bool>[true, true],
      reason:
          'left to right the row is version text, captions, chevron '
          '(got $text, $captions, $chevron)',
    );
    expect(find.text(Strings.play), findsNothing);
    expect(_more(), findsNothing);
  });

  testWidgets('an open row puts play on the left and the menu on the right', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    await _expand(tester);

    final Rect row = tester.getRect(find.byType(VersionPicker));
    final Rect play = tester.getRect(
      find.widgetWithText(OutlinedButton, Strings.play),
    );
    final Rect more = tester.getRect(_more());
    expect(play.left - row.left, lessThan(row.right - play.right));
    expect(row.right - more.right, lessThan(more.left - row.left));
    expect(play.center.dy, moreOrLessEquals(more.center.dy, epsilon: 1));
  });

  testWidgets('the chevron still opens the detail body', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));
    expect(find.text(Strings.subtitlesHeading.toUpperCase()), findsNothing);

    await tester.tap(_chevron());
    await tester.pumpAndSettle();

    expect(find.text(Strings.subtitlesHeading.toUpperCase()), findsOneWidget);
  });

  testWidgets('downloading asks the server for a link for that version', (
    WidgetTester tester,
  ) async {
    final List<String> seen = <String>[];
    await _pump(tester, _api(seen));

    await _download(tester);

    expect(seen, contains('POST /api/v1/versions/v1/download'));
    expect(find.text(Strings.toastDownloadStarted), findsOneWidget);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a failed link surfaces an error instead of a success', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[], failDownload: true));

    await _download(tester);

    expect(find.text(Strings.toastDownloadStarted), findsNothing);
    expect(find.textContaining(Strings.errorDownload), findsOneWidget);
    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an unavailable version offers no download', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]), available: false);

    expect(find.text(Strings.unavailable), findsOneWidget);
    await _expand(tester);
    expect(_more(), findsNothing, reason: 'nothing to download or open');
  });

  testWidgets('a resumable row offers Resume and dismisses from the menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[], resumeMs: 2880000));

    await _expand(tester);

    expect(
      find.widgetWithText(FilledButton, Strings.resumeAction),
      findsOneWidget,
    );
    expect(find.byTooltip(Strings.resume(40)), findsOneWidget);
    expect(find.byTooltip(Strings.dismissResume), findsNothing);

    await tester.tap(_more());
    await tester.pumpAndSettle();

    expect(find.text(Strings.dismissResume), findsOneWidget);
  });

  testWidgets('a handoff that never happened is not reported as success', (
    WidgetTester tester,
  ) async {
    // The toast used to fire on the request succeeding, so on every platform
    // but the web it announced a download that was never started.
    startDownload = (String url, String filename) async => false;
    await _pump(tester, _api(<String>[]));

    await _download(tester);

    expect(find.text(Strings.toastDownloadStarted), findsNothing);
    expect(find.text(Strings.errorDownload), findsOneWidget);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the signed link is what gets handed off', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    await _download(tester);

    expect(started, <String>['http://test/download/tok-123']);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
