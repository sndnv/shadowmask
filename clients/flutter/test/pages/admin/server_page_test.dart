import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/benchmark_version_dialog.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/progress_meter.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/server_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Map<String, dynamic> _ready({Map<String, dynamic>? test, String? error}) =>
    <String, dynamic>{
      'state': 'ready',
      'checked_at': '2026-10-08T09:00:00Z',
      'host': <String, dynamic>{
        'cpu': '12th Gen Intel(R) Core(TM) i3-1220P',
        'cores': 10,
        'threads': 12,
      },
      'ffmpeg': <String, dynamic>{
        'version': error == null ? '5.1.6-0+deb12u1' : null,
        'error': error,
        'hwaccels': error == null ? <String>['vdpau', 'vaapi'] : <String>[],
        'encoders': <dynamic>[
          <String, dynamic>{'name': 'libx264', 'present': error == null},
          <String, dynamic>{'name': 'h264_vaapi', 'present': error == null},
        ],
        'filters': <dynamic>[
          <String, dynamic>{'name': 'tonemap_vaapi', 'present': false},
        ],
      },
      'hardware': <String, dynamic>{
        'mode': 'auto',
        'device': '/dev/dri/renderD128',
        'device_present': true,
        'in_use': true,
        'test':
            test ??
            <String, dynamic>{
              'outcome': 'works',
              'low_power': true,
              'elapsed_ms': 420,
            },
      },
    };

Map<String, dynamic> _run(
  String encoder,
  String outcome, {
  double realtime = 2,
  String? detail,
}) => <String, dynamic>{
  'segment': 450,
  'start_ms': 1800000,
  'duration_ms': 4000,
  'encoder': encoder,
  'outcome': outcome,
  'elapsed_ms': 2000,
  'realtime': realtime,
  'detail': detail,
};

Map<String, dynamic> _benchmark(
  String state, {
  int done = 0,
  List<Map<String, dynamic>> runs = const <Map<String, dynamic>>[],
  String? keyframesError,
}) => <String, dynamic>{
  'state': state,
  'version_id': state == 'idle' ? null : 'v1',
  'started_at': state == 'idle' ? null : '2026-10-08T09:00:00Z',
  'finished_at': state == 'done' ? '2026-10-08T09:01:00Z' : null,
  'source': state == 'idle'
      ? null
      : <String, dynamic>{
          'codec': 'hevc',
          'width': 3840,
          'height': 2160,
          'hdr': 'hdr10',
          'duration_ms': 7200000,
        },
  'keyframes_ms': keyframesError == null && state != 'idle' ? 1500 : null,
  'keyframes_error': keyframesError,
  'progress': <String, dynamic>{'done': done, 'total': 6},
  'runs': runs,
};

const String _kMetrics = '''
build_info{version="0.0.4"} 1
transcode_segments_total{encoder="vaapi",outcome="ok"} 20
transcode_segments_total{encoder="vaapi",outcome="failed"} 2
transcode_segment_duration_milliseconds_sum{encoder="vaapi"} 20000
transcode_segment_duration_milliseconds_count{encoder="vaapi"} 20
transcode_media_milliseconds_total{encoder="vaapi"} 80000
transcode_fallbacks_total 7
transcode_segments_abandoned_total 3
stream_starts_total{delivery="transcode",outcome="ok"} 9
stream_first_segment_milliseconds_sum{delivery="transcode"} 27000
stream_first_segment_milliseconds_count{delivery="transcode"} 9
stream_keyframe_probe_duration_milliseconds_sum 6000
stream_keyframe_probe_duration_milliseconds_count 4
stream_keyframe_probe_failures_total 1
sessions_started_total{mode="direct"} 5
sessions_started_total{mode="transcode"} 6
stream_bytes_sent_total{kind="file"} 307717731
stream_bytes_sent_total{kind="segment"} 56084426
stream_bytes_sent_total{kind="playlist"} 17534
''';

const String _kNothingStreamed = 'build_info{version="0.0.4"} 1\n';

class _Server {
  _Server({
    this.role = 'admin',
    List<Map<String, dynamic>>? capabilities,
    List<Map<String, dynamic>>? benchmarks,
    this.metrics = _kMetrics,
    this.metricsStatus = 200,
    this.versions = const <Map<String, dynamic>>[],
    this.versionsStatus = 200,
    this.startStatus = 202,
  }) : capabilities = capabilities ?? <Map<String, dynamic>>[_ready()],
       benchmarks = benchmarks ?? <Map<String, dynamic>>[_benchmark('idle')];

  final String role;
  final List<Map<String, dynamic>> capabilities;
  final List<Map<String, dynamic>> benchmarks;
  final String metrics;
  final int metricsStatus;
  final List<Map<String, dynamic>> versions;
  final int versionsStatus;
  final int startStatus;
  final List<http.Request> requests = <http.Request>[];

  int count(String method, String path) => requests
      .where((http.Request r) => r.method == method && r.url.path == path)
      .length;

  Map<String, dynamic> _next(List<Map<String, dynamic>> queue) =>
      queue.length > 1 ? queue.removeAt(0) : queue.first;

  http.Response _json(Object body, [int status = 200]) =>
      http.Response(jsonEncode(body), status);

  http.Response handle(http.Request req) {
    requests.add(req);
    return switch ((req.method, req.url.path)) {
      ('GET', '/api/v1/users/self') => _json(<String, dynamic>{
        'id': 'u1',
        'username': 'pat',
        'role': role,
      }),
      ('GET', '/api/v1/admin/server/capabilities') => _json(
        _next(capabilities),
      ),
      ('POST', '/api/v1/admin/server/capabilities/recheck') => http.Response(
        '',
        202,
      ),
      ('GET', '/api/v1/admin/server/benchmark') => _json(_next(benchmarks)),
      ('POST', '/api/v1/admin/server/benchmark') =>
        startStatus == 202
            ? http.Response('', 202)
            : _json(<String, dynamic>{
                'error': <String, dynamic>{
                  'code': 'benchmark_running',
                  'message': 'a benchmark is already running',
                },
              }, startStatus),
      ('GET', '/api/v1/admin/versions') => _json(<String, dynamic>{
        'items': versions,
        'total': versions.length,
        'offset': 0,
        'limit': 10,
      }, versionsStatus),
      ('GET', '/metrics') => http.Response(metrics, metricsStatus),
      _ => http.Response('{}', 200),
    };
  }
}

Map<String, dynamic> _version(String id, String path) => <String, dynamic>{
  'id': id,
  'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
  'library_id': 'lib1',
  'quality': 'uhd',
  'container': 'mkv',
  'path': path,
};

Future<_Server> _pump(
  WidgetTester tester, {
  _Server? server,
  Size size = const Size(1600, 3000),
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final _Server s = server ?? _Server();
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async => s.handle(req)),
  );
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (RouteSettings settings) => MaterialPageRoute<void>(
          builder: (_) => settings.name == null || settings.name == '/'
              ? ServerPage(api: api)
              : Text('went to ${settings.name}'),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return s;
}

final String _kCheckedAt = dateTimeText('2026-10-08T09:00:00Z')!;

Future<void> _openChooser(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.speed));
  await tester.pumpAndSettle();
}

Future<void> _search(WidgetTester tester, String text) async {
  await tester.enterText(
    find.descendant(
      of: find.byType(BenchmarkVersionDialog),
      matching: find.byType(TextField),
    ),
    text,
  );
  await tester.tap(find.byTooltip('Search'));
  await tester.pumpAndSettle();
}

Color? _colorOf(WidgetTester tester, String text) =>
    tester.widget<Text>(find.text(text)).style?.color;

Future<void> _openHelp(WidgetTester tester, String label) async {
  if (find.byType(Dialog).evaluate().isNotEmpty) {
    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();
  }
  await tester.tap(
    find.descendant(
      of: find.ancestor(of: find.text(label), matching: find.byType(Row)).first,
      matching: find.byTooltip('What is this?'),
    ),
  );
  await tester.pumpAndSettle();
}

OutlinePill _pill(WidgetTester tester, String name) => tester.widget(
  find.ancestor(of: find.text(name), matching: find.byType(OutlinePill)),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the capabilities show the host, ffmpeg and hardware', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(find.text('Checked'), findsOneWidget);
    expect(find.text(_kCheckedAt), findsOneWidget);
    expect(find.text('12th Gen Intel(R) Core(TM) i3-1220P'), findsOneWidget);
    expect(find.text('10'), findsOneWidget);
    expect(find.text('12'), findsOneWidget);
    expect(find.text('5.1.6-0+deb12u1'), findsOneWidget);
    expect(_pill(tester, 'vdpau').active, isTrue);
    expect(_pill(tester, 'vdpau').icon, Icons.check);
    expect(_pill(tester, 'libx264').active, isTrue);
    expect(_pill(tester, 'libx264').icon, Icons.check);
    expect(_pill(tester, 'h264_vaapi').active, isTrue);
    expect(_pill(tester, 'tonemap_vaapi').active, isFalse);
    expect(_pill(tester, 'tonemap_vaapi').icon, Icons.close);
    expect(find.byTooltip('Missing'), findsOneWidget);
    expect(find.text('/dev/dri/renderD128'), findsOneWidget);
    expect(find.text('Works in low-power mode, 420 ms'), findsOneWidget);
  });

  testWidgets(
    'the capabilities start as close to their title as the benchmark',
    (WidgetTester tester) async {
      await _pump(
        tester,
        server: _Server(
          benchmarks: <Map<String, dynamic>>[_benchmark('done', done: 6)],
        ),
      );

      final double hostGap =
          tester.getTopLeft(find.text('Host')).dy -
          tester.getBottomLeft(find.text('Capabilities')).dy;
      final double statusGap =
          tester.getTopLeft(find.text('Status')).dy -
          tester.getBottomLeft(find.text('Benchmark').first).dy;
      expect(hostGap, lessThanOrEqualTo(statusGap));
    },
  );

  testWidgets('help buttons explain a fact, a pill list and a column', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _openHelp(tester, 'Mode');

    expect(find.textContaining('auto: the GPU (VAAPI)'), findsOneWidget);

    await _openHelp(tester, 'Encoders');

    expect(find.textContaining('libx264 on the CPU'), findsOneWidget);

    await _openHelp(tester, 'DIRECT BYTES');

    expect(
      find.text('Bytes sent for direct play since the server started.'),
      findsOneWidget,
    );
  });

  testWidgets('realtime is red below 1× and green from 2×', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      server: _Server(
        metrics: _kNothingStreamed,
        benchmarks: <Map<String, dynamic>>[
          _benchmark(
            'done',
            done: 6,
            runs: <Map<String, dynamic>>[
              _run('vaapi', 'ok', realtime: 0.5),
              _run('software', 'ok', realtime: 2),
            ],
          ),
        ],
      ),
    );

    expect(_colorOf(tester, '0.50×'), Tokens.dark.danger);
    expect(_colorOf(tester, '2.00×'), Tokens.dark.ok);
    expect(_colorOf(tester, 'software'), Tokens.dark.danger);
    expect(
      tester
          .widgetList<Text>(find.text('vaapi'))
          .map((Text text) => text.style?.color),
      contains(Tokens.dark.ok),
    );
  });

  testWidgets('remux keeps the normal colour', (WidgetTester tester) async {
    await _pump(
      tester,
      server: _Server(
        metrics: 'transcode_segments_total{encoder="remux",outcome="ok"} 5\n',
      ),
    );

    expect(_colorOf(tester, 'remux'), isNull);
  });

  testWidgets('the capabilities sit in three columns that stack when narrow', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    final Offset host = tester.getTopLeft(find.text('Host'));
    final Offset hardware = tester.getTopLeft(find.text('Hardware encoding'));
    final Offset ffmpeg = tester.getTopLeft(find.text('FFmpeg'));
    expect(hardware.dy, host.dy);
    expect(ffmpeg.dy, host.dy);
    expect(host.dx, lessThan(hardware.dx));
    expect(hardware.dx, lessThan(ffmpeg.dx));

    await _pump(tester, size: const Size(600, 3000));

    final Offset narrowHost = tester.getTopLeft(find.text('Host'));
    final Offset narrowHardware = tester.getTopLeft(
      find.text('Hardware encoding'),
    );
    final Offset narrowFfmpeg = tester.getTopLeft(find.text('FFmpeg'));
    expect(narrowHardware.dx, narrowHost.dx);
    expect(narrowFfmpeg.dx, narrowHost.dx);
    expect(narrowHost.dy, lessThan(narrowHardware.dy));
    expect(narrowHardware.dy, lessThan(narrowFfmpeg.dy));
  });

  testWidgets('a failed test encode shows both of ffmpeg\'s errors', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      server: _Server(
        metrics: _kNothingStreamed,
        capabilities: <Map<String, dynamic>>[
          _ready(
            test: <String, dynamic>{
              'outcome': 'failed',
              'detail': 'No usable encoding entrypoint',
              'low_power_detail': 'Function not implemented',
            },
          ),
        ],
      ),
    );

    expect(find.text('Failed'), findsOneWidget);
    expect(find.text('ffmpeg error'), findsOneWidget);
    expect(find.text('No usable encoding entrypoint'), findsOneWidget);
    expect(find.text('Low-power error'), findsOneWidget);
    expect(find.text('Function not implemented'), findsOneWidget);
  });

  testWidgets('a skipped test and a missing ffmpeg show why', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      server: _Server(
        capabilities: <Map<String, dynamic>>[
          _ready(
            error: 'ffmpeg could not be started',
            test: <String, dynamic>{
              'outcome': 'skipped',
              'detail': 'hardware transcoding is off',
            },
          ),
        ],
      ),
    );

    expect(find.text('ffmpeg could not be started'), findsOneWidget);
    expect(find.text('None'), findsOneWidget);
    expect(find.text('Skipped, hardware transcoding is off'), findsOneWidget);
    expect(find.text('ffmpeg error'), findsNothing);
  });

  testWidgets('a running check is polled until it is ready', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      server: _Server(
        capabilities: <Map<String, dynamic>>[
          <String, dynamic>{'state': 'unchecked'},
          <String, dynamic>{'state': 'checking'},
          _ready(),
        ],
      ),
    );

    expect(find.text('Not yet'), findsOneWidget);

    await tester.tap(find.text('Re-check'));
    await tester.pumpAndSettle();

    expect(
      server.count('POST', '/api/v1/admin/server/capabilities/recheck'),
      1,
    );
    expect(find.text('Checking…'), findsOneWidget);

    await tester.pump(kServerPoll);
    await tester.pumpAndSettle();

    expect(find.text(_kCheckedAt), findsOneWidget);
    expect(find.text('Checking…'), findsNothing);
    final int reads = server.count('GET', '/api/v1/admin/server/capabilities');
    expect(reads, 3);

    await tester.pump(kServerPoll * 3);

    expect(server.count('GET', '/api/v1/admin/server/capabilities'), reads);
  });

  testWidgets('no benchmark yet says so', (WidgetTester tester) async {
    await _pump(tester);

    expect(find.text('No benchmark has run yet.'), findsOneWidget);
  });

  testWidgets('a running benchmark shows progress until it is done', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      server: _Server(
        metrics: _kNothingStreamed,
        benchmarks: <Map<String, dynamic>>[
          _benchmark(
            'running',
            done: 2,
            runs: <Map<String, dynamic>>[
              _run(
                'vaapi',
                'failed',
                realtime: 13.33,
                detail: 'No usable encoding entrypoint',
              ),
              _run('software', 'ok', realtime: 1.85),
            ],
          ),
          _benchmark(
            'done',
            done: 6,
            runs: <Map<String, dynamic>>[
              _run('software', 'ok', realtime: 1.85),
              _run('software', 'too_slow', realtime: 0.07),
            ],
          ),
        ],
      ),
    );

    expect(find.text('Running, 2 of 6 runs'), findsOneWidget);
    expect(find.byType(ProgressMeter), findsOneWidget);
    expect(find.text('33%'), findsOneWidget);
    expect(find.text('HEVC · 3840×2160 · HDR10 · 2h 00m'), findsOneWidget);
    expect(find.text('1.50 s'), findsOneWidget);
    expect(find.text('#450 at 30:00'), findsNWidgets(2));
    expect(find.text('1.85×'), findsOneWidget);
    expect(_colorOf(tester, '1.85×'), Tokens.dark.warn);
    expect(_colorOf(tester, 'software'), Tokens.dark.danger);
    expect(find.text('13.33×'), findsNothing);
    expect(find.text('No usable encoding entrypoint'), findsOneWidget);
    expect(
      rowTintOf(tester, 'No usable encoding entrypoint'),
      Tokens.dark.rowDanger,
    );

    await tester.pump(kServerPoll);
    await tester.pumpAndSettle();

    expect(find.text('Done'), findsOneWidget);
    expect(find.byType(ProgressMeter), findsNothing);
    expect(find.text('Too slow'), findsOneWidget);
    expect(rowTintOf(tester, 'Too slow'), Tokens.dark.rowWarn);
    final int reads = server.count('GET', '/api/v1/admin/server/benchmark');

    await tester.pump(kServerPoll * 3);

    expect(server.count('GET', '/api/v1/admin/server/benchmark'), reads);
  });

  testWidgets('a run\'s detail shows its first line and opens in full', (
    WidgetTester tester,
  ) async {
    const String detail =
        'No usable encoding entrypoint\nError initializing output stream';
    await _pump(
      tester,
      server: _Server(
        metrics: _kNothingStreamed,
        benchmarks: <Map<String, dynamic>>[
          _benchmark(
            'done',
            done: 6,
            runs: <Map<String, dynamic>>[
              _run('vaapi', 'failed', detail: detail),
              _run('software', 'ok'),
            ],
          ),
        ],
      ),
    );

    expect(find.text('No usable encoding entrypoint'), findsOneWidget);
    expect(find.text(detail), findsNothing);

    await tester.tap(find.text('No usable encoding entrypoint'));
    await tester.pumpAndSettle();

    expect(find.text('ffmpeg error'), findsOneWidget);
    expect(find.text(detail), findsOneWidget);
  });

  testWidgets(
    'the benchmark info sits beside its runs and stacks when narrow',
    (WidgetTester tester) async {
      final List<Map<String, dynamic>> done = <Map<String, dynamic>>[
        _benchmark(
          'done',
          done: 6,
          runs: <Map<String, dynamic>>[_run('software', 'ok')],
        ),
      ];
      await _pump(tester, server: _Server(benchmarks: done));

      final Offset status = tester.getTopLeft(find.text('Status'));
      final Offset run = tester.getTopLeft(find.text('#450 at 30:00'));
      expect(status.dx, lessThan(run.dx));
      expect(run.dy, lessThan(tester.getTopLeft(find.text('Source')).dy));

      await _pump(
        tester,
        size: const Size(600, 3000),
        server: _Server(benchmarks: done),
      );

      final Offset narrowStatus = tester.getTopLeft(find.text('Status'));
      final Offset narrowRun = tester.getTopLeft(find.text('#450 at 30:00'));
      expect(narrowStatus.dy, lessThan(narrowRun.dy));
    },
  );

  testWidgets('a failed keyframe read is shown', (WidgetTester tester) async {
    await _pump(
      tester,
      server: _Server(
        benchmarks: <Map<String, dynamic>>[
          _benchmark('done', done: 6, keyframesError: 'ffprobe failed'),
        ],
      ),
    );

    expect(find.text('ffprobe failed'), findsOneWidget);
    expect(find.text('No runs yet.'), findsOneWidget);
  });

  testWidgets('the benchmarked version links to its page', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      server: _Server(
        benchmarks: <Map<String, dynamic>>[_benchmark('done', done: 6)],
      ),
    );

    await tester.tap(find.text('v1'));
    await tester.pumpAndSettle();

    expect(find.text('went to /version?id=v1'), findsOneWidget);
  });

  testWidgets('a search finds versions and one starts a benchmark', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      server: _Server(
        versions: <Map<String, dynamic>>[
          _version('v1', '/media/Neon Harbor (2018)/Neon Harbor.mkv'),
        ],
        benchmarks: <Map<String, dynamic>>[
          _benchmark('idle'),
          _benchmark('running'),
          _benchmark('done', done: 6),
        ],
      ),
    );

    await _openChooser(tester);
    await _search(tester, ' neon ');

    final http.Request search = server.requests.lastWhere(
      (http.Request r) => r.url.path == '/api/v1/admin/versions',
    );
    expect(search.url.queryParameters['filter'], 'neon');
    expect(search.url.queryParameters['limit'], '10');

    await tester.tap(
      find.text('UHD · mkv · /media/Neon Harbor (2018)/Neon Harbor.mkv'),
    );
    await tester.pump();
    await tester.pump();

    final http.Request start = server.requests.lastWhere(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/admin/server/benchmark',
    );
    expect(jsonDecode(start.body), <String, dynamic>{'version_id': 'v1'});
    expect(find.text('Benchmark started.'), findsOneWidget);

    await tester.pumpAndSettle();

    expect(find.byType(BenchmarkVersionDialog), findsNothing);
    expect(find.text('Running, 0 of 6 runs'), findsOneWidget);

    await tester.pump(kServerPoll);
    await tester.pumpAndSettle();
    await tester.pump(kToastDuration);
    await tester.pumpAndSettle();
  });

  testWidgets('a second benchmark is refused and shows the one running', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(
      tester,
      server: _Server(
        startStatus: 409,
        benchmarks: <Map<String, dynamic>>[
          _benchmark('idle'),
          _benchmark('running'),
          _benchmark('done', done: 6),
        ],
        versions: <Map<String, dynamic>>[_version('v1', '/media/a.mkv')],
      ),
    );
    expect(find.text('No benchmark has run yet.'), findsOneWidget);

    await _openChooser(tester);
    await _search(tester, 'a');
    await tester.tap(find.text('UHD · mkv · /media/a.mkv'));
    await tester.pump();
    await tester.pump();

    expect(
      find.text(
        'Could not start the benchmark. A benchmark is already running.',
      ),
      findsOneWidget,
    );
    await tester.pumpAndSettle();
    expect(server.count('GET', '/api/v1/admin/server/benchmark'), 2);
    expect(find.text('Running, 0 of 6 runs'), findsOneWidget);

    await tester.pump(kServerPoll);
    await tester.pumpAndSettle();
    await tester.pump(kErrorToastDuration);
    await tester.pumpAndSettle();
  });

  testWidgets('the chooser asks for a query and says when nothing matches', (
    WidgetTester tester,
  ) async {
    await _pump(tester);
    await _openChooser(tester);

    await _search(tester, '  ');

    expect(
      find.text('Enter a path, title, library or quality to search for.'),
      findsOneWidget,
    );

    await _search(tester, 'nothing');

    expect(find.text('No versions match the filter.'), findsOneWidget);
  });

  testWidgets('a failed search says so', (WidgetTester tester) async {
    await _pump(tester, server: _Server(versionsStatus: 500));
    await _openChooser(tester);

    await _search(tester, 'neon');

    expect(find.text('Search failed.'), findsOneWidget);
  });

  testWidgets('closing the chooser starts nothing', (
    WidgetTester tester,
  ) async {
    final _Server server = await _pump(tester);
    await _openChooser(tester);

    await tester.tapAt(const Offset(5, 5));
    await tester.pumpAndSettle();

    expect(find.byType(BenchmarkVersionDialog), findsNothing);
    expect(server.count('POST', '/api/v1/admin/server/benchmark'), 0);
  });

  testWidgets('the totals come from /metrics', (WidgetTester tester) async {
    final _Server server = await _pump(tester);

    expect(find.text('Metrics'), findsOneWidget);
    expect(find.text('SOFTWARE FALLBACKS'), findsOneWidget);
    expect(find.text('KEYFRAME FAILURES'), findsOneWidget);
    expect(find.text('SESSIONS'), findsOneWidget);
    expect(find.text('STREAM STARTS'), findsOneWidget);
    expect(find.text('vaapi'), findsNWidgets(2));
    expect(find.text('20'), findsOneWidget);
    expect(find.text('2'), findsOneWidget);
    expect(find.text('1.00 s'), findsOneWidget);
    expect(find.text('4.00×'), findsOneWidget);
    expect(find.text('7'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('direct'), findsOneWidget);
    expect(find.text('5'), findsOneWidget);
    expect(find.text('—'), findsNWidgets(3));
    expect(find.text('transcode'), findsOneWidget);
    expect(find.text('6'), findsOneWidget);
    expect(find.text('9'), findsOneWidget);
    expect(find.text('3.00 s'), findsOneWidget);
    expect(find.text('4'), findsOneWidget);
    expect(find.text('1.50 s'), findsOneWidget);
    expect(find.text('1'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('direct')).dy,
      lessThan(tester.getTopLeft(find.text('transcode')).dy),
    );

    await tester.tap(find.text('Refresh'));
    await tester.pumpAndSettle();

    expect(server.count('GET', '/metrics'), 2);
    expect(find.text('0.3 GB'), findsOneWidget);
    expect(find.text('0.1 GB'), findsOneWidget);
    expect(find.text('DIRECT BYTES'), findsOneWidget);
    expect(_colorOf(tester, '4.00×'), Tokens.dark.ok);
    expect(
      tester
          .widgetList<Text>(find.text('vaapi'))
          .map((Text text) => text.style?.color),
      contains(Tokens.dark.ok),
    );
  });

  testWidgets('an encoder without timings shows no average', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      server: _Server(
        metrics: 'transcode_segments_total{encoder="remux",outcome="ok"} 5\n',
      ),
    );

    expect(find.text('remux'), findsOneWidget);
    expect(find.text('—'), findsNWidgets(3));
  });

  testWidgets('nothing streamed yet says so', (WidgetTester tester) async {
    await _pump(tester, server: _Server(metrics: _kNothingStreamed));

    expect(
      find.text('Nothing has streamed since the server started.'),
      findsOneWidget,
    );
  });

  testWidgets('an unreadable /metrics leaves the rest of the page working', (
    WidgetTester tester,
  ) async {
    await _pump(tester, server: _Server(metricsStatus: 404));

    expect(
      find.text('Could not read /metrics. A reverse proxy may be blocking it.'),
      findsOneWidget,
    );
    expect(find.text('12th Gen Intel(R) Core(TM) i3-1220P'), findsOneWidget);
    expect(find.text('No benchmark has run yet.'), findsOneWidget);
  });

  testWidgets('a /metrics behind a proxy login stays in its block', (
    WidgetTester tester,
  ) async {
    for (final int status in <int>[401, 403]) {
      await _pump(tester, server: _Server(metricsStatus: status));

      expect(
        find.text(
          'Could not read /metrics. A reverse proxy may be blocking it.',
        ),
        findsOneWidget,
        reason: '$status',
      );
      expect(find.text(Strings.signInRequired), findsNothing);
      expect(find.text(Strings.notAuthorized), findsNothing);
      expect(find.text('12th Gen Intel(R) Core(TM) i3-1220P'), findsOneWidget);
    }
  });

  testWidgets('a non-admin is not authorized', (WidgetTester tester) async {
    final _Server server = await _pump(tester, server: _Server(role: 'user'));

    expect(find.text('Not Authorized.'), findsOneWidget);
    expect(server.count('GET', '/api/v1/admin/server/capabilities'), 0);
  });

  testWidgets('a phone renders the page without overflowing', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      size: const Size(390, 4000),
      server: _Server(
        benchmarks: <Map<String, dynamic>>[
          _benchmark(
            'done',
            done: 6,
            runs: <Map<String, dynamic>>[_run('software', 'ok')],
          ),
        ],
      ),
    );

    expect(tester.takeException(), isNull);
  });
}
