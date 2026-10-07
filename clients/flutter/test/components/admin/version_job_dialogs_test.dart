import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/version_job_dialogs.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

SubtitleFile _sub(String id, String language) => SubtitleFile(
  id: id,
  language: language,
  format: SubtitleFormat.srt,
  source: SubtitleSource.external,
);

VideoTrack _video(int width, int height) =>
    VideoTrack(index: 0, codec: 'h264', width: width, height: height);

Future<List<http.Request>> _pump(
  WidgetTester tester,
  Future<void> Function(BuildContext context, AdminApi admin) open,
) async {
  final List<http.Request> seen = <http.Request>[];
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      seen.add(req);
      return http.Response(jsonEncode(<String, String>{'job_id': 'j1'}), 202);
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
            onPressed: () => open(context, AdminApi(api)),
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  return seen;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('upscale offers only the heights above the current one', () {
    expect(upscaleOptions(null).length, 5);
    expect(
      upscaleOptions(_video(1280, 720)).map(((int, String) o) => o.$1),
      <int>[1080, 1440, 2160],
    );
    expect(upscaleOptions(_video(3840, 2160)), isEmpty);
  });

  testWidgets('upscale posts the height picked from the filtered list', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showUpscaleDialog(
        context,
        admin: admin,
        versionId: 'v1',
        video: _video(1280, 720),
      ),
    );

    expect(find.textContaining('1280×720'), findsOneWidget);
    expect(find.text(Strings.height720), findsNothing);
    expect(find.text(Strings.height1080), findsOneWidget);

    await tester.tap(find.text(Strings.height1080));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.height2160).last);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(seen, hasLength(1));
    expect(seen.single.url.path, '/api/v1/versions/v1/upscale');
    expect(seen.single.url.queryParameters, <String, String>{
      'target_height': '2160',
    });

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a version at the top resolution says so and cannot submit', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showUpscaleDialog(
        context,
        admin: admin,
        versionId: 'v1',
        video: _video(3840, 2160),
      ),
    );

    expect(find.text(Strings.alreadyMaxResolution), findsOneWidget);
    expect(find.text(Strings.fieldTargetHeight), findsNothing);

    final FilledButton save = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, Strings.save),
    );
    expect(save.onPressed, isNull);
    expect(seen, isEmpty);
  });

  testWidgets('transcribe posts the track it was opened from', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showTranscribeDialog(
        context,
        admin: admin,
        versionId: 'v1',
        audioTrackIndex: 2,
      ),
    );

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(seen.single.url.path, '/api/v1/versions/v1/subtitles/transcribe');
    expect(seen.single.url.queryParameters, <String, String>{
      'audio_track_index': '2',
    });

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('transcribe lets the player pick among several audio tracks', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showTranscribeDialog(
        context,
        admin: admin,
        versionId: 'v1',
        audioTrackIndex: 1,
        tracks: const <AudioTrack>[
          AudioTrack(index: 1, codec: 'eac3', channels: 6, language: 'en'),
          AudioTrack(index: 2, codec: 'aac', language: 'es'),
        ],
        queuedToast: Strings.toastQueued,
      ),
    );

    expect(find.text(Strings.fieldAudioTrack), findsOneWidget);
    await tester.tap(find.text('English · eac3 · 6ch'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Spanish · aac · 2ch').last);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(seen.single.url.queryParameters, <String, String>{
      'audio_track_index': '2',
    });
    expect(find.text(Strings.toastQueued), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a single audio track needs no picker', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showTranscribeDialog(
        context,
        admin: admin,
        versionId: 'v1',
        audioTrackIndex: 1,
        tracks: const <AudioTrack>[AudioTrack(index: 1, codec: 'aac')],
      ),
    );

    expect(find.text(Strings.fieldAudioTrack), findsNothing);
  });

  testWidgets('translate lets the player pick which subtitle to translate', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showTranslateDialog(
        context,
        admin: admin,
        versionId: 'v1',
        source: _sub('s1', 'en'),
        sources: <SubtitleFile>[_sub('s1', 'en'), _sub('s2', 'fr')],
      ),
    );

    expect(find.text(Strings.fieldSourceSubtitle), findsOneWidget);
    await tester.tap(find.text('English · srt · External'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('French · srt · External').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.optionNoLanguage));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Arabic').last);
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(seen.single.url.pathSegments.skip(2), <String>[
      'versions',
      'v1',
      'subtitles',
      's2',
      'translate',
    ]);
    expect(seen.single.url.queryParameters, <String, String>{
      'target_language': 'ar',
    });
    expect(find.text(Strings.toastQueuedTrackJobs), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('combining one subtitle with itself reports inline', (
    WidgetTester tester,
  ) async {
    final List<http.Request> seen = await _pump(
      tester,
      (BuildContext context, AdminApi admin) => showCombineDialog(
        context,
        admin: admin,
        versionId: 'v1',
        subs: <SubtitleFile>[_sub('s1', 'en')],
      ),
    );

    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(find.text(Strings.requiredDistinctSubtitles), findsOneWidget);
    expect(seen, isEmpty);
  });
}
