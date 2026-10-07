import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/version_job_lines.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

VersionJob _job(String id, String kind, String status, [String? subtitle]) =>
    VersionJob.fromJson(<String, dynamic>{
      'id': id,
      'kind': kind,
      'status': status,
      'created_at': '2026-10-06T10:00:00Z',
      'elapsed_ms': 65000,
      'subtitle_id': ?subtitle,
    });

VersionJob _failed(String id, String reason) =>
    VersionJob.fromJson(<String, dynamic>{
      'id': id,
      'kind': 'transcription',
      'status': 'failed',
      'created_at': '2026-10-06T10:00:00Z',
      'language': 'en',
      'last_error': reason,
    });

final VersionDetail _detail = VersionDetail.fromJson(<String, dynamic>{
  'id': 'v1',
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'l',
  'quality': 'fhd',
  'container': 'mkv',
});

Future<void> _pump(
  WidgetTester tester,
  List<VersionJob> jobs, {
  bool still = false,
}) => tester.pumpWidget(
  MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: MediaQuery(
      data: MediaQueryData(disableAnimations: still),
      child: Scaffold(
        body: SizedBox(
          width: 400,
          child: VersionJobLines(jobs: jobs, detail: _detail),
        ),
      ),
    ),
  ),
);

Color? _colour(WidgetTester tester, String text) =>
    tester.widget<Text>(find.text(text)).style?.color;

void main() {
  testWidgets('nothing shows without subtitle work', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <VersionJob>[_job('j1', 'trickplay', 'running')]);

    expect(find.byType(Text), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('running work carries a moving bar', (WidgetTester tester) async {
    await _pump(tester, <VersionJob>[
      _job('j1', 'transcription', 'running'),
      _job('j2', 'translation', 'queued'),
    ]);

    expect(find.text('Transcription'), findsOneWidget);
    expect(find.text('Running · 1m 05s'), findsOneWidget);
    expect(find.text('Translation'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
  });

  testWidgets('reduced motion drops the bar but keeps the words', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <VersionJob>[
      _job('j1', 'transcription', 'running'),
    ], still: true);

    expect(find.text('Running · 1m 05s'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets('each outcome has its own tone', (WidgetTester tester) async {
    await _pump(tester, <VersionJob>[
      _job('j1', 'transcription', 'failed'),
      _job('j2', 'translation', 'succeeded', 'machine:v1:es'),
      _job('j3', 'combine', 'succeeded'),
    ]);
    final Tokens t = Tokens.dark;

    expect(_colour(tester, Strings.statusFailed), t.danger);
    expect(_colour(tester, Strings.jobNoSubtitle), t.warn);
    expect(_colour(tester, Strings.statusSucceeded), t.ok);
  });

  testWidgets('statuses start in one column, a failure on one line', (
    WidgetTester tester,
  ) async {
    const String reason =
        'whisper exited with status 1 after reading the whole audio track '
        'and finding no speech it could transcribe';
    await _pump(tester, <VersionJob>[
      _failed('j1', 'permanent job failure: $reason'),
      _job('j2', 'combine', 'succeeded'),
      _job('j3', 'translation', 'running'),
    ]);

    final Rect title = tester.getRect(find.text('Transcription · English'));
    final Rect failed = tester.getRect(find.text('Failed · $reason'));
    final Rect succeeded = tester.getRect(find.text(Strings.statusSucceeded));
    final Rect running = tester.getRect(find.text('Running · 1m 05s'));
    final Rect bar = tester.getRect(find.byType(LinearProgressIndicator));

    expect(failed.left, succeeded.left);
    expect(failed.left, running.left);
    expect(bar.left, running.left);
    expect(failed.left, greaterThan(title.right));
    expect(failed.height, title.height);
    expect(find.textContaining('job failure'), findsNothing);
  });
}
