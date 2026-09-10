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

Finder _download() => find.byTooltip(Strings.downloadVersion);
Finder _chevron() => find.byTooltip(Strings.fullVersionDetails);

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

  testWidgets('download sits by the version text, actions stay on the right', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    final double text = tester.getCenter(find.byType(Text).first).dx;
    final double download = tester.getCenter(_download()).dx;
    final double captions = tester
        .getCenter(find.byIcon(Icons.closed_caption_outlined))
        .dx;
    final double play = tester.getCenter(find.text(Strings.play)).dx;
    final double chevron = tester.getCenter(_chevron()).dx;

    expect(
      <bool>[
        text < download,
        download < captions,
        captions < play,
        play < chevron,
      ],
      <bool>[true, true, true, true],
      reason:
          'left to right the row is version text, download, captions, play, '
          'chevron (got $text, $download, $captions, $play, $chevron)',
    );
    expect(
      download,
      lessThan(captions / 2),
      reason:
          'download hugs the text on the left, it does not join the '
          'cluster of controls on the right',
    );
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

    await tester.tap(_download());
    await tester.pumpAndSettle();

    expect(seen, contains('POST /api/v1/versions/v1/download'));
    expect(find.text(Strings.toastDownloadStarted), findsOneWidget);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a failed link surfaces an error instead of a success', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[], failDownload: true));

    await tester.tap(_download());
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastDownloadStarted), findsNothing);
    expect(find.textContaining(Strings.errorDownload), findsOneWidget);
    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('an unavailable version offers no download', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]), available: false);

    expect(_download(), findsNothing);
    expect(find.text(Strings.unavailable), findsOneWidget);
  });

  testWidgets('the picker says sidecar subtitles are not included', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    expect(find.text(Strings.downloadNoSubtitles), findsOneWidget);
  });

  testWidgets(
    'a resumable row uses the same segmented pair as the title page',
    (WidgetTester tester) async {
      await _pump(tester, _api(<String>[], resumeMs: 2880000));

      final Finder resume = find.widgetWithText(
        FilledButton,
        Strings.resumeAction,
      );
      final Finder dismiss = find.byTooltip(Strings.dismissResume);
      expect(resume, findsOneWidget);
      expect(dismiss, findsOneWidget);
      expect(
        find.text(Strings.dismiss),
        findsNothing,
        reason:
            'the dismiss half is the icon-only segment, not a labelled button',
      );
      expect(
        tester.getRect(resume).right,
        moreOrLessEquals(tester.getRect(dismiss).left, epsilon: 0.01),
        reason: 'the two halves share an edge, exactly as on the title page',
      );
      expect(find.byTooltip(Strings.resume(40)), findsOneWidget);
    },
  );

  testWidgets('a handoff that never happened is not reported as success', (
    WidgetTester tester,
  ) async {
    // The toast used to fire on the request succeeding, so on every platform
    // but the web it announced a download that was never started.
    startDownload = (String url, String filename) async => false;
    await _pump(tester, _api(<String>[]));

    await tester.tap(_download());
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastDownloadStarted), findsNothing);
    expect(find.text(Strings.errorDownload), findsOneWidget);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the signed link is what gets handed off', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(<String>[]));

    await tester.tap(_download());
    await tester.pumpAndSettle();

    expect(started, <String>['http://test/download/tok-123']);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
