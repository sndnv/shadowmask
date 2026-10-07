import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/player/add_subtitles_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

Map<String, dynamic> _file(String id, String language) => <String, dynamic>{
  'id': id,
  'language': language,
  'format': 'srt',
  'source': 'external',
};

VersionDetail _version(List<Map<String, dynamic>> files) =>
    VersionDetail.fromJson(<String, dynamic>{
      'id': 'v1',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      'library_id': 'l',
      'quality': 'fhd',
      'container': 'mkv',
      'audio': <dynamic>[
        <String, dynamic>{'index': 1, 'codec': 'eac3', 'language': 'en'},
        <String, dynamic>{'index': 2, 'codec': 'aac', 'language': 'es'},
      ],
      'subtitle_files': files,
    });

VersionJob _job(String status) => VersionJob.fromJson(<String, dynamic>{
  'id': 'j1',
  'kind': 'combine',
  'status': status,
  'ahead': 1,
  'created_at': '2026-10-06T10:00:00Z',
});

class _Harness {
  _Harness(List<Map<String, dynamic>> files)
    : version = ValueNotifier<VersionDetail?>(_version(files));

  final ValueNotifier<VersionDetail?> version;
  final ValueNotifier<List<VersionJob>> jobs = ValueNotifier<List<VersionJob>>(
    const <VersionJob>[],
  );
  final List<Uri> posted = <Uri>[];
  int reported = 0;

  void dispose() {
    version.dispose();
    jobs.dispose();
  }
}

Future<_Harness> _open(
  WidgetTester tester, {
  List<Map<String, dynamic>> files = const <Map<String, dynamic>>[],
  int? audioTrack,
  String? watchingFileId,
}) async {
  SharedPreferences.setMockInitialValues(<String, Object>{});
  final _Harness h = _Harness(files);
  addTearDown(h.dispose);
  final AdminApi admin = AdminApi(
    ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        h.posted.add(req.url);
        return http.Response(jsonEncode(<String, String>{'job_id': 'j1'}), 202);
      }),
    ),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      builder: (BuildContext context, Widget? child) =>
          ToastHost(child: child ?? const SizedBox.shrink()),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () => showAddSubtitlesDialog(
              context,
              admin: admin,
              version: h.version,
              jobs: h.jobs,
              onWork: () => h.reported++,
              audioTrack: audioTrack,
              watchingFileId: watchingFileId,
            ),
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  return h;
}

TextButton _action(WidgetTester tester, String label) =>
    tester.widget<TextButton>(find.widgetWithText(TextButton, label));

void main() {
  testWidgets('every way to add a subtitle is offered when it can work', (
    WidgetTester tester,
  ) async {
    await _open(tester);

    expect(find.text(Strings.addSubtitlesHeading), findsOneWidget);
    expect(_action(tester, Strings.searchSubtitles).onPressed, isNotNull);
    expect(_action(tester, Strings.transcribe).onPressed, isNotNull);
    expect(_action(tester, Strings.translate).onPressed, isNull);
    expect(_action(tester, Strings.combineSubtitles).onPressed, isNull);
  });

  testWidgets('the actions sit two by two, search and combine first', (
    WidgetTester tester,
  ) async {
    await _open(tester);
    Rect at(String label) =>
        tester.getRect(find.widgetWithText(TextButton, label));
    final Rect search = at(Strings.searchSubtitles);
    final Rect combine = at(Strings.combineSubtitles);
    final Rect transcribe = at(Strings.transcribe);
    final Rect translate = at(Strings.translate);

    expect(combine.top, search.top);
    expect(combine.left, greaterThanOrEqualTo(search.right));
    expect(translate.top, transcribe.top);
    expect(translate.left, greaterThanOrEqualTo(transcribe.right));
    expect(transcribe.top, greaterThanOrEqualTo(search.bottom));
    expect(transcribe.left, search.left);
    expect(translate.left, combine.left);
  });

  testWidgets('a subtitle that arrives opens translate and combine', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _open(
      tester,
      files: <Map<String, dynamic>>[_file('f1', 'en')],
    );
    expect(_action(tester, Strings.translate).onPressed, isNotNull);
    expect(_action(tester, Strings.combineSubtitles).onPressed, isNull);

    h.version.value = _version(<Map<String, dynamic>>[
      _file('f1', 'en'),
      _file('f2', 'fr'),
    ]);
    await tester.pump();

    expect(_action(tester, Strings.combineSubtitles).onPressed, isNotNull);
  });

  testWidgets('the work lines follow the watch while the dialog is open', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _open(tester);
    expect(find.byType(Divider), findsNothing);
    expect(find.text(Strings.jobsHeading), findsNothing);

    h.jobs.value = <VersionJob>[_job('queued')];
    await tester.pump();
    expect(find.text(Strings.jobsHeading), findsOneWidget);
    expect(find.text('Queued · 1 job ahead'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text(Strings.jobsHeading)).dy,
      lessThan(tester.getTopLeft(find.text('Queued · 1 job ahead')).dy),
    );

    h.jobs.value = <VersionJob>[_job('failed')];
    await tester.pump();
    expect(find.text(Strings.statusFailed), findsOneWidget);
  });

  testWidgets('the jobs heading explains what the list is for', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _open(tester);
    h.jobs.value = <VersionJob>[_job('queued')];
    await tester.pump();

    final Rect heading = tester.getRect(find.text(Strings.jobsHeading));
    final Rect help = tester.getRect(find.byType(FieldHelp));
    expect(help.left, greaterThan(heading.left));
    expect(help.center.dy, closeTo(heading.center.dy, 4));

    await tester.tap(find.byType(FieldHelp));
    await tester.pumpAndSettle();

    expect(find.text(Strings.subtitleJobsHelp), findsOneWidget);
  });

  testWidgets('started work is reported and the dialog stays open', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _open(
      tester,
      files: <Map<String, dynamic>>[_file('f1', 'en'), _file('f2', 'fr')],
    );

    await tester.tap(find.widgetWithText(TextButton, Strings.combineSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, Strings.save));
    await tester.pumpAndSettle();

    expect(h.posted.single.path, endsWith('/subtitles/f1/combine'));
    expect(h.reported, 1);
    expect(find.text(Strings.addSubtitlesHeading), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a cancelled form reports nothing', (WidgetTester tester) async {
    final _Harness h = await _open(tester);

    await tester.tap(find.widgetWithText(TextButton, Strings.transcribe));
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(4, 4));
    await tester.pumpAndSettle();

    expect(h.posted, isEmpty);
    expect(h.reported, 0);
  });

  testWidgets('transcribe starts on the track being heard', (
    WidgetTester tester,
  ) async {
    await _open(tester, audioTrack: 2);

    await tester.tap(find.widgetWithText(TextButton, Strings.transcribe));
    await tester.pumpAndSettle();

    expect(find.text('Spanish · aac · 2ch'), findsOneWidget);
  });

  testWidgets('translate starts from the subtitle being watched', (
    WidgetTester tester,
  ) async {
    await _open(
      tester,
      files: <Map<String, dynamic>>[_file('f1', 'en'), _file('f2', 'fr')],
      watchingFileId: 'f2',
    );

    await tester.tap(find.widgetWithText(TextButton, Strings.translate));
    await tester.pumpAndSettle();

    expect(find.text('French · srt · External'), findsOneWidget);
  });

  testWidgets('nothing shows before the version is known', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _open(tester);
    h.version.value = null;
    await tester.pump();

    expect(find.text(Strings.addSubtitlesHeading), findsOneWidget);
    expect(find.text(Strings.searchSubtitles), findsNothing);
  });
}
