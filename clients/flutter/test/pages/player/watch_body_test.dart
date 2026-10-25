import 'dart:async';
import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/util/scoped_value.dart';
import 'package:shadowmask/components/player/overlay_bar.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/player_sheet.dart';
import 'package:shadowmask/components/player/report_dialog.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/timeline.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/view/player_shortcuts.dart';
import 'package:shadowmask/view/subtitle_size.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/model/auth/auth_tokens.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/immersive_scope.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/pages/player/watch_body.dart';
import 'package:shadowmask/player/playback_runner.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/watches/buffer_watch.dart';
import 'package:shadowmask/player/watches/stall_watch.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/version_jobs_watch.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/fake_player_controller.dart';

http.Response? _resumeAt40s(http.Request r) =>
    r.method == 'GET' && r.url.path.contains('/progress/')
    ? http.Response(jsonEncode(<String, dynamic>{'position_ms': 40000}), 200)
    : null;

List<int> _reported(List<http.Request> seen) => <int>[
  for (final http.Request r in seen)
    if (r.method == 'POST' && r.url.path.endsWith('/progress'))
      (jsonDecode(r.body) as Map<String, dynamic>)['position_ms'] as int,
];

http.Response _route(http.Request req) {
  final String path = req.url.path;
  if (req.method == 'GET' && path == '/api/v1/server/info') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'version': '1',
        'features': <String>[],
        'profile_version': 1,
      }),
      200,
    );
  }
  if (req.method == 'GET' && path.contains('/progress/')) {
    return http.Response(jsonEncode(<String, dynamic>{'position_ms': 0}), 200);
  }
  if (req.method == 'GET' && path == '/api/v1/versions/v1') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'v1',
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
        'library_id': 'lib',
        'quality': 'hd',
        'container': 'mp4',
        'duration_ms': 100000,
        'video': <dynamic>[
          <String, dynamic>{
            'index': 0,
            'codec': 'h264',
            'width': 1920,
            'height': 1080,
            'frame_rate': 24.0,
          },
        ],
        'audio': <dynamic>[
          <String, dynamic>{
            'index': 1,
            'codec': 'aac',
            'channels': 2,
            'language': 'en',
          },
        ],
        'subtitles': <dynamic>[
          <String, dynamic>{'index': 2, 'language': 'en', 'format': 'srt'},
        ],
      }),
      200,
    );
  }
  if (req.method == 'POST' && path == '/api/v1/sessions') {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'session_id': 's1',
        'mode': 'direct',
        'manifest_url': '/stream/tok/file',
        'heartbeat_interval_s': 10,
        'selected': <String, dynamic>{'audio_track': 1, 'subtitle_track': null},
        'trickplay': <dynamic>[],
      }),
      201,
    );
  }
  if (req.method == 'POST' && path.endsWith('/update')) {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'manifest_url': '/stream/tok2/master.m3u8',
        'mode': 'transcode',
        'selected': <String, dynamic>{},
      }),
      200,
    );
  }
  return http.Response('', 204);
}

http.Response _renegotiated(String mode, String manifestUrl) => http.Response(
  jsonEncode(<String, dynamic>{
    'manifest_url': manifestUrl,
    'mode': mode,
    'selected': <String, dynamic>{},
  }),
  200,
);

http.Response _sequentialSession(int originMs, String? container) =>
    http.Response(
      jsonEncode(<String, dynamic>{
        'session_id': 's1',
        'mode': 'remux',
        'manifest_url': '/stream/tok/master.m3u8',
        'origin_ms': originMs,
        'sequential': true,
        'container': ?container,
        'heartbeat_interval_s': 10,
        'selected': <String, dynamic>{'audio_track': 1, 'subtitle_track': null},
        'trickplay': <dynamic>[],
      }),
      201,
    );

http.Response _seekRestart(int originMs, String? container) => http.Response(
  jsonEncode(<String, dynamic>{
    'manifest_url': '/stream/tok3/master.m3u8',
    'mode': 'remux',
    'origin_ms': originMs,
    'sequential': true,
    'container': ?container,
    'selected': <String, dynamic>{},
  }),
  200,
);

Map<String, dynamic> _ep(String id, int number, String title) =>
    <String, dynamic>{
      'id': id,
      'season_id': 'se1',
      'series_id': 's1',
      'series_title': 'The Show',
      'season_number': 1,
      'number': number,
      'title': title,
      'added_at': '2026-08-17T09:00:00Z',
      'updated_at': '2026-08-17T09:00:00Z',
      'artwork': <String, dynamic>{},
      'series_artwork': <String, dynamic>{
        'backdrops': <dynamic>[
          <String, dynamic>{
            'base': '/artwork/show',
            'widths': <int>[kBackdropWidth],
          },
        ],
      },
    };

http.Response _episodeVersion() => http.Response(
  jsonEncode(<String, dynamic>{
    'id': 'v1',
    'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
    'library_id': 'lib',
    'quality': 'hd',
    'container': 'mp4',
    'duration_ms': 100000,
    'audio': <dynamic>[
      <String, dynamic>{
        'index': 1,
        'codec': 'aac',
        'channels': 2,
        'language': 'en',
      },
    ],
    'subtitles': <dynamic>[
      <String, dynamic>{'index': 2, 'language': 'en', 'format': 'srt'},
    ],
  }),
  200,
);

http.Response? _episodic(String path) {
  if (path.endsWith('/versions')) {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'items': <dynamic>[
          <String, dynamic>{
            'id': 'v2',
            'title': <String, dynamic>{'type': 'episode', 'id': 'e2'},
            'library_id': 'lib',
            'quality': 'hd',
            'container': 'mp4',
            'duration_ms': 100000,
            'available': true,
          },
        ],
        'total': 1,
      }),
      200,
    );
  }
  if (path.endsWith('/episodes')) {
    return http.Response(
      jsonEncode(<dynamic>[_ep('e1', 1, 'Pilot'), _ep('e2', 2, 'Second')]),
      200,
    );
  }
  if (path.contains('/episodes/')) {
    return http.Response(jsonEncode(_ep('e1', 1, 'Pilot')), 200);
  }
  if (path.endsWith('/seasons')) {
    return http.Response(
      jsonEncode(<dynamic>[
        <String, dynamic>{'id': 'se1', 'series_id': 's1', 'number': 1},
      ]),
      200,
    );
  }
  if (path.contains('/seasons/')) {
    return http.Response(
      jsonEncode(<String, dynamic>{
        'id': 'se1',
        'series_id': 's1',
        'number': 1,
      }),
      200,
    );
  }
  if (RegExp(r'^/api/v1/series/[^/]+$').hasMatch(path)) {
    return http.Response(
      jsonEncode(<String, dynamic>{'id': 's1', 'title': 'The Show'}),
      200,
    );
  }
  return null;
}

const String _mutedKey = 'shadowmask.player.muted';

const String _timeoutKey = 'shadowmask.player.network_timeout';

const String _bufferKey = 'shadowmask.player.buffer_seconds';

const String _waitKey = 'shadowmask.player.wait_for_buffer';

const int _stallDelay = kStallSeconds;

Future<void> _settleAroundSpinner(WidgetTester tester) async {
  for (int i = 0; i < 12; i++) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

Future<void> _settled(WidgetTester tester) =>
    tester.pump(const Duration(milliseconds: kSeekSettleWaitMs));

Future<void> _shown(WidgetTester tester) async {
  await tester.pump();
  await tester.pump();
}

final Matcher _noPlayOrPause = isNot(anyElement(anyOf('play', 'pause')));

const SelfUser _user = SelfUser(id: 'u1', username: 'u', role: UserRole.user);

const SelfUser _linkedAdmin = SelfUser(
  id: 'u1',
  username: 'u',
  role: UserRole.player,
  accountRole: UserRole.admin,
);

http.Response? _noJobs(http.Request r) => r.url.path.endsWith('/jobs')
    ? http.Response(jsonEncode(<dynamic>[]), 200)
    : null;

Widget _shellShaped(ApiClient api, FakePlayerController fake, bool bounded) =>
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      navigatorObservers: <NavigatorObserver>[appRouteObserver],
      home: Scaffold(
        body: CustomScrollView(
          slivers: <Widget>[
            SliverToBoxAdapter(
              child: ConstrainedBox(
                constraints: bounded
                    ? const BoxConstraints.tightFor(height: 500)
                    : const BoxConstraints(minHeight: 500),
                child: WatchBody(
                  api: api,
                  user: _user,
                  versionId: 'v1',
                  initialControls: const PlaybackControls(),
                  controllerFactory: (_) => fake,
                  wide: bounded,
                  onToggleWide: () {},
                ),
              ),
            ),
          ],
        ),
      ),
    );

Widget _app(
  FakePlayerController fake, {
  List<http.Request>? seen,
  bool wide = false,
  bool episodic = false,
  VoidCallback? onToggleWide,
  PlaybackControls controls = const PlaybackControls(),
  List<String>? pushed,
  bool sequential = false,
  int originMs = 0,
  int seekOriginMs = 0,
  String? container,
  bool touch = false,
  Future<http.Response> Function(int call)? onUpdate,
  SelfUser user = _user,
  http.Response? Function(http.Request r)? intercept,
  Duration? Function(http.Request r)? lag,
  bool toasts = false,
}) {
  int updates = 0;
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) {
      seen?.add(r);
      final http.Response? caught = intercept?.call(r);
      if (caught != null) {
        final Duration? wait = lag?.call(r);
        return wait == null
            ? Future<http.Response>.value(caught)
            : Future<http.Response>.delayed(wait, () => caught);
      }
      if (onUpdate != null && r.url.path.endsWith('/update')) {
        return onUpdate(updates++);
      }
      if (sequential) {
        if (r.method == 'POST' && r.url.path == '/api/v1/sessions') {
          return Future<http.Response>.value(
            _sequentialSession(originMs, container),
          );
        }
        if (r.method == 'POST' && r.url.path.endsWith('/seek')) {
          return Future<http.Response>.value(
            _seekRestart(seekOriginMs, container),
          );
        }
      }
      if (episodic) {
        if (r.url.path == '/api/v1/versions/v1') {
          return Future<http.Response>.value(_episodeVersion());
        }
        final http.Response? hit = _episodic(r.url.path);
        if (hit != null) {
          return Future<http.Response>.value(hit);
        }
      }
      return Future<http.Response>.value(_route(r));
    }),
  );
  return MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    navigatorObservers: <NavigatorObserver>[appRouteObserver],
    builder: toasts
        ? (BuildContext context, Widget? child) => ToastHost(child: child!)
        : null,
    onGenerateRoute: pushed == null
        ? null
        : (RouteSettings settings) {
            pushed.add(settings.name ?? '');
            return MaterialPageRoute<void>(
              settings: settings,
              builder: (BuildContext _) => const SizedBox(),
            );
          },
    home: Scaffold(
      body: WatchBody(
        api: api,
        user: user,
        versionId: 'v1',
        initialControls: controls,
        controllerFactory: (_) => fake,
        wide: wide,
        touch: touch,
        onToggleWide: onToggleWide ?? () {},
        appVersion: () async => '0.0.7',
      ),
    ),
  );
}

Widget _gathering(
  FakePlayerController fake, {
  List<http.Request>? seen,
  bool touch = false,
}) => _app(
  fake,
  seen: seen,
  touch: touch,
  intercept: (http.Request r) =>
      r.url.path == '/api/v1/users/u1/progress/v1' ? _route(r) : null,
  lag: (http.Request r) => const Duration(seconds: 1),
);

bool _started(List<http.Request> seen) => seen.any(
  (http.Request r) => r.method == 'POST' && r.url.path == '/api/v1/sessions',
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a backdrop carried in from the details page is never dropped', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    const String carried = 'http://test/artwork/1/1920';
    final ScopedValue<String?> backdrop = ScopedValue<String?>(carried);
    addTearDown(backdrop.dispose);
    final List<String?> seen = <String?>[];
    backdrop.addListener(() => seen.add(backdrop.value));

    await tester.pumpWidget(
      BackdropScope(url: backdrop, child: _app(fake, episodic: true)),
    );
    await tester.pumpAndSettle();

    expect(
      seen,
      isEmpty,
      reason:
          'any write at all, even the same url, re-owns it and flickers '
          'the wash on the way back out',
    );
    expect(backdrop.value, carried);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a player landed on cold publishes a backdrop of its own', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final ScopedValue<String?> backdrop = ScopedValue<String?>(null);
    addTearDown(backdrop.dispose);

    await tester.pumpWidget(
      BackdropScope(url: backdrop, child: _app(fake, episodic: true)),
    );
    await tester.pumpAndSettle();

    expect(
      backdrop.value,
      'http://test/artwork/show/$kBackdropWidth',
      reason: 'nothing was carried in, so the player supplies the artwork',
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('boots a session and renders the transport and clock', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(fake.calls, containsAllInOrder(<String>['attach:direct', 'play']));
    expect(find.byIcon(Icons.pause), findsOneWidget);
    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.text('0:00 / 1:40'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a start that failed boots on retry', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    int starts = 0;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request r) async {
        if (r.method == 'POST' &&
            r.url.path == '/api/v1/sessions' &&
            starts++ == 0) {
          return http.Response('', 500);
        }
        return _route(r);
      }),
    );
    await tester.pumpWidget(_shellShaped(api, fake, false));
    await tester.pumpAndSettle();
    expect(find.text(Strings.couldNotStartPlayback), findsOneWidget);

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(fake.calls, contains('attach:direct'));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the wide screen control reports a toggle', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    int toggles = 0;
    await tester.pumpWidget(_app(fake, onToggleWide: () => toggles++));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.width_wide), findsOneWidget);
    await tester.tap(find.byIcon(Icons.width_wide));
    expect(toggles, 1);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the wide screen control offers the way back when wide', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, wide: true));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.width_normal), findsOneWidget);
    expect(find.byIcon(Icons.width_wide), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the video fits a bounded viewport instead of overflowing it', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 500);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, wide: true));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      tester.getBottomLeft(find.byType(PlayerFrame)).dy,
      lessThanOrEqualTo(500),
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('going wide keeps the same player, so playback is not cut', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1600, 500);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final FakePlayerController fake = FakePlayerController();
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient(
        (http.Request r) => Future<http.Response>.value(_route(r)),
      ),
    );

    await tester.pumpWidget(_shellShaped(api, fake, false));
    await tester.pumpAndSettle();
    final State<PlayerFrame> before = tester.state<State<PlayerFrame>>(
      find.byType(PlayerFrame),
    );

    await tester.pumpWidget(_shellShaped(api, fake, true));
    await tester.pumpAndSettle();

    expect(
      identical(
        tester.state<State<PlayerFrame>>(find.byType(PlayerFrame)),
        before,
      ),
      isTrue,
      reason: 'a rebuilt frame recreates the video element and stops playback',
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('fullscreen hides the wide screen control', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.width_wide), findsOneWidget);

    await tester.tap(find.byIcon(Icons.fullscreen));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.width_wide), findsNothing);
    expect(find.byIcon(Icons.width_normal), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the transport button drives the controller', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.pause));
    await tester.pump();
    expect(fake.calls.last, 'pause');

    await tester.tap(find.byIcon(Icons.play_arrow).first);
    await tester.pump();
    expect(fake.calls.last, 'play');

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a policy mute surfaces an unmute control that clears it', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.volume_off), findsNothing);

    fake.mutePolicy();
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.volume_off), findsOneWidget);

    await tester.tap(find.byIcon(Icons.volume_off));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('muted:false'));
    expect(find.byIcon(Icons.volume_off), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('tapping a policy-muted video unmutes instead of pausing', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    fake.mutePolicy();
    await tester.pumpAndSettle();

    fake.calls.clear();
    await tester.tapAt(
      tester.getCenter(find.byType(PlayerFrame)) + const Offset(0, 80),
    );
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('unmute'));
    expect(fake.calls, _noPlayOrPause);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('each overlay button opens only its own group', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerQuality), findsOneWidget);
    expect(find.text(Strings.playerSpeed), findsOneWidget);
    expect(find.text(Strings.playerDiagnostics), findsNothing);

    await tester.tap(find.byIcon(Icons.audiotrack).first);
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerStereoDownmix), findsOneWidget);
    expect(find.text(Strings.playerQuality), findsNothing);

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerSubtitles), findsWidgets);

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerDiagnostics), findsOneWidget);
    expect(find.text(Strings.playerPictureInPicture), findsOneWidget);
    expect(find.text(Strings.playerSpeed), findsNothing);

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerDiagnostics), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('subtitle offset and burn in appear only with a track chosen', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerOffset), findsNothing);
    expect(find.text(Strings.playerBurnIn), findsNothing);

    await tester.tap(find.text(Strings.playerNoSubtitles).last);
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerOffset), findsOneWidget);
    expect(find.text(Strings.playerBurnIn), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'the subtitle size is applied at start and saved from the panel',
    (WidgetTester tester) async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.player.subtitle_size': 'large',
      });
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      expect(fake.subtitleSize, SubtitleSize.large);

      await tester.tap(find.byIcon(Icons.subtitles).first);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerNoSubtitles).last);
      await tester.pumpAndSettle();
      await tester.tap(find.textContaining('English').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.subtitleSizeLarge).last);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.subtitleSizeSmall).last);
      await tester.pumpAndSettle();

      expect(fake.subtitleSize, SubtitleSize.small);
      expect(
        (await SharedPreferences.getInstance()).getString(
          'shadowmask.player.subtitle_size',
        ),
        'small',
      );

      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('a plain viewer is offered no subtitle work and polls none', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();

    expect(find.text(Strings.addSubtitles), findsNothing);
    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/jobs')),
      isEmpty,
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a device linked to an admin works on subtitles in the player', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, user: _linkedAdmin, intercept: _noJobs),
    );
    await tester.pumpAndSettle();
    fake.pause();
    await _shown(tester);
    await tester.pump(const Duration(minutes: 1));
    bool readJobs(http.Request r) => r.url.path == '/api/v1/versions/v1/jobs';
    expect(seen.where(readJobs), isEmpty);

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();

    expect(find.text(Strings.searchSubtitles), findsNothing);
    expect(seen.where(readJobs), isEmpty);

    await tester.tap(find.text(Strings.addSubtitles));
    await tester.pumpAndSettle();

    expect(find.text(Strings.addSubtitlesHeading), findsOneWidget);
    TextButton action(String label) =>
        tester.widget<TextButton>(find.widgetWithText(TextButton, label));
    expect(action(Strings.searchSubtitles).onPressed, isNotNull);
    expect(action(Strings.transcribe).onPressed, isNotNull);
    expect(action(Strings.translate).onPressed, isNull);
    expect(action(Strings.combineSubtitles).onPressed, isNull);
    expect(seen.where(readJobs), hasLength(1));

    await tester.pump(const Duration(minutes: 1));
    expect(seen.where(readJobs), hasLength(1));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('finished work puts the new subtitle in the player menu', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    int polls = 0;
    bool translated = false;
    await tester.pumpWidget(
      _app(
        fake,
        user: _linkedAdmin,
        intercept: (http.Request r) {
          if (r.url.path == '/api/v1/versions/v1/jobs') {
            final String status = polls++ == 0 ? 'running' : 'succeeded';
            translated = status == 'succeeded';
            return http.Response(
              jsonEncode(<dynamic>[
                <String, dynamic>{
                  'id': 'j1',
                  'kind': 'translation',
                  'status': status,
                  'created_at': '2026-10-06T10:00:00Z',
                  'elapsed_ms': 3000,
                  'language': 'es',
                  'subtitle_id': 'machine:v1:es',
                },
              ]),
              200,
            );
          }
          if (translated && r.url.path == '/api/v1/versions/v1') {
            final Map<String, dynamic> body =
                jsonDecode(_route(r).body) as Map<String, dynamic>;
            body['subtitle_files'] = <dynamic>[
              <String, dynamic>{
                'id': 'machine:v1:es',
                'language': 'es',
                'format': 'srt',
                'source': 'machine_translated',
              },
            ];
            return http.Response(jsonEncode(body), 200);
          }
          return null;
        },
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    expect(polls, 0);

    Future<void> settleDialog() async {
      for (int i = 0; i < 10; i++) {
        await tester.pump(const Duration(milliseconds: 50));
      }
    }

    await tester.tap(find.text(Strings.addSubtitles));
    await settleDialog();
    expect(polls, 1);
    expect(find.text('Running · 3s'), findsOneWidget);

    await tester.tapAt(const Offset(4, 4));
    await settleDialog();
    expect(find.text(Strings.addSubtitlesHeading), findsNothing);

    await tester.pump(kJobsBusyPoll);
    await settleDialog();
    expect(polls, 2);

    await tester.tap(find.text(Strings.addSubtitles));
    await tester.pumpAndSettle();
    expect(find.text('Ready · Spanish (translated) added'), findsOneWidget);

    await tester.tapAt(const Offset(4, 4));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles).last);
    await tester.pumpAndSettle();
    expect(find.textContaining('Spanish · Translated'), findsWidgets);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('work finishing under an open dialog never reads as empty', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    int polls = 0;
    bool translated = false;
    await tester.pumpWidget(
      _app(
        fake,
        user: _linkedAdmin,
        intercept: (http.Request r) {
          if (r.url.path == '/api/v1/versions/v1/jobs') {
            final String status = polls++ == 0 ? 'running' : 'succeeded';
            translated = status == 'succeeded';
            return http.Response(
              jsonEncode(<dynamic>[
                <String, dynamic>{
                  'id': 'j1',
                  'kind': 'translation',
                  'status': status,
                  'created_at': '2026-10-06T10:00:00Z',
                  'elapsed_ms': 3000,
                  'language': 'es',
                  'subtitle_id': 'machine:v1:es',
                },
              ]),
              200,
            );
          }
          if (translated && r.url.path == '/api/v1/versions/v1') {
            final Map<String, dynamic> body =
                jsonDecode(_route(r).body) as Map<String, dynamic>;
            body['subtitle_files'] = <dynamic>[
              <String, dynamic>{
                'id': 'machine:v1:es',
                'language': 'es',
                'format': 'srt',
                'source': 'machine_translated',
              },
            ];
            return http.Response(jsonEncode(body), 200);
          }
          return null;
        },
        lag: (http.Request r) =>
            translated && r.url.path == '/api/v1/versions/v1'
            ? const Duration(seconds: 2)
            : null,
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.addSubtitles));
    for (int i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(find.text('Running · 3s'), findsOneWidget);

    final Finder ready = find.text('Ready · Spanish (translated) added');
    await tester.pump(kJobsBusyPoll);
    for (int frame = 0; frame < 60 && ready.evaluate().isEmpty; frame++) {
      expect(find.text('Running · 3s'), findsOneWidget);
      expect(find.text(Strings.jobNoSubtitle), findsNothing);
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(polls, 2);
    expect(ready, findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a click outside the panel dismisses it', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerQuality), findsOneWidget);

    await tester.tapAt(const Offset(40, 40));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerQuality), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a click on the controls dismisses the panel and still acts', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.tap(
      find.descendant(
        of: find.byType(OverlayBar),
        matching: find.byIcon(Icons.pause),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerQuality), findsNothing);
    expect(fake.calls, contains('pause'));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a panel button still closes its own panel and switches', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    final Finder audio = find.descendant(
      of: find.byType(OverlayBar),
      matching: find.byIcon(Icons.audiotrack),
    );
    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(audio);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerQuality), findsNothing);
    expect(find.text(Strings.playerStereoDownmix), findsOneWidget);

    await tester.tap(audio);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerStereoDownmix), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('on touch the options button opens the sheet of sections', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();

    expect(find.byType(PlayerSheet), findsOneWidget);
    expect(find.text(Strings.playerOptions), findsOneWidget);
    for (final PlayerPanel panel in PlayerPanel.values) {
      expect(find.text(panel.title), findsOneWidget);
    }

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a subtitle track is chosen through the sheet', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pumpAndSettle();

    expect(find.byTooltip(Strings.playerOffsetEarlier), findsOneWidget);
    expect(find.text(Strings.playerSubtitles), findsWidgets);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'a track chosen through the sheet while playing keeps its place',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final Completer<http.Response> slow = Completer<http.Response>();
      await tester.pumpWidget(
        _app(fake, touch: true, onUpdate: (int _) => slow.future),
      );
      await tester.pumpAndSettle();
      fake.at(positionMs: 1000);
      await tester.pump();
      await tester.tap(find.byIcon(Icons.tune));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerSubtitles));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerNoSubtitles));
      await tester.pumpAndSettle();

      await tester.tap(find.textContaining('English').last);
      await tester.pump();

      expect(find.text(Strings.applyingPlaybackChanges), findsOneWidget);
      expect(find.byTooltip(Strings.playerOffsetEarlier), findsOneWidget);

      slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
      await tester.pumpAndSettle();

      expect(find.text(Strings.applyingPlaybackChanges), findsNothing);
      expect(find.byTooltip(Strings.playerOffsetEarlier), findsOneWidget);

      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('a tap beside the sheet closes it and leaves the controls up', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();

    await tester.tapAt(
      tester.getCenter(find.byType(PlayerFrame)) - const Offset(200, 0),
    );
    await tester.pumpAndSettle();
    await tester.pump(kDoubleTapTimeout);

    expect(find.byType(PlayerSheet), findsNothing);
    expect(
      tester
          .widget<AnimatedOpacity>(
            find
                .ancestor(
                  of: find.byType(OverlayBar),
                  matching: find.byType(AnimatedOpacity),
                )
                .first,
          )
          .opacity,
      1,
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('escape closes the sheet before it touches full screen', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(find.byType(PlayerSheet), findsNothing);
    expect(fake.calls, isNot(contains('fullscreen')));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a help topic read in the sheet leaves the sheet open', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();
    await tester.tap(find.text(PlayerPanel.video.title));
    await tester.pumpAndSettle();

    await tester.tap(find.byTooltip(Strings.whatIsThis).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerQualityHelp));
    await tester.pumpAndSettle();
    Navigator.of(tester.element(find.text(Strings.playerQualityHelp))).pop();
    await tester.pumpAndSettle();

    expect(find.byType(PlayerSheet), findsOneWidget);
    expect(find.text(Strings.playerQuality), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('Back closes the sheet before it leaves the player', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.byType(PlayerSheet), findsNothing);
    expect(find.byType(WatchBody), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the time display counts down when remaining is on', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(find.text('0:00 / 1:40'), findsOneWidget);

    await tester.tap(find.text('0:00 / 1:40'));
    await tester.pumpAndSettle();

    expect(find.text('1:40 / 1:40'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('picture in picture is offered from the settings group', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerPictureInPicture));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('pip'));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('changing quality renegotiates with target_height', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    // The source is 1080p, so Original names it and 1080p is not offered as a
    // rung of its own.
    expect(find.text(Strings.playerOriginalAt(1080)).last, findsOneWidget);
    expect(find.text('1080p'), findsNothing);
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();

    final http.Request update = seen.firstWhere(
      (http.Request r) => r.url.path == '/api/v1/sessions/s1/update',
    );
    expect(
      (jsonDecode(update.body) as Map<String, dynamic>)['target_height'],
      720,
    );
    expect(
      fake.calls.where((String c) => c.startsWith('attach')).length,
      greaterThan(1),
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a control change after a local seek sends the new position', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.at(positionMs: 60000);
    await tester.pump();
    fake.at(positionMs: 5000);
    await tester.pump();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();

    final http.Request update = seen.firstWhere(
      (http.Request r) => r.url.path == '/api/v1/sessions/s1/update',
    );
    expect(
      (jsonDecode(update.body) as Map<String, dynamic>)['position_ms'],
      5000,
    );
    expect(fake.attachedPositionMs, 5000);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'a control change before the resume plays keeps the resume point',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(_app(fake, seen: seen, intercept: _resumeAt40s));
      await tester.pumpAndSettle();

      fake.opening();
      await tester.pump();

      await tester.tap(find.byIcon(Icons.video_settings));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('720p').last);
      await tester.pumpAndSettle();

      final http.Request update = seen.firstWhere(
        (http.Request r) => r.url.path == '/api/v1/sessions/s1/update',
      );
      expect(
        (jsonDecode(update.body) as Map<String, dynamic>)['position_ms'],
        40000,
      );
      expect(fake.attachedPositionMs, 40000);

      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('a step before the resume plays steps from the resume point', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, intercept: _resumeAt40s));
    await tester.pumpAndSettle();

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    expect(fake.seekedMs, 40000 + kSeekStepMs);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pump();
    expect(fake.seekedMs, 40000 + 2 * kSeekStepMs);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'a control change after a seek made before playing sends the seek',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(_app(fake, seen: seen, intercept: _resumeAt40s));
      await tester.pumpAndSettle();

      fake.pausedAt(positionMs: 40000);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.video_settings));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('720p').last);
      await tester.pumpAndSettle();

      final http.Request update = seen.firstWhere(
        (http.Request r) => r.url.path == '/api/v1/sessions/s1/update',
      );
      expect(
        (jsonDecode(update.body) as Map<String, dynamic>)['position_ms'],
        40000 + kSeekStepMs,
      );

      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'a renegotiated stream reports where it was sent until it plays',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(_app(fake, seen: seen));
      await tester.pumpAndSettle();
      await _settled(tester);

      fake.at(positionMs: 60000);
      await tester.pump();

      await tester.tap(find.byIcon(Icons.video_settings));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
      await tester.pumpAndSettle();
      await tester.tap(find.text('720p').last);
      await tester.pumpAndSettle();
      expect(fake.attachedPositionMs, 60000);

      fake.opening();
      await tester.pump();
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();

      expect(_reported(seen).last, 60000);
    },
  );

  testWidgets('a restarted stream reports where it was sent until it plays', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        sequential: true,
        originMs: 0,
        seekOriginMs: 88000,
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pumpAndSettle();
    expect(fake.attachedPositionMs, 2000);

    fake.opening();
    await tester.pump();
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();

    expect(_reported(seen).last, 90000);
  });

  testWidgets('a pending renegotiation is visible and cannot be retriggered', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        onUpdate: (int call) => call == 0
            ? slow.future
            : Future<http.Response>.value(
                _renegotiated('remux', '/stream/second/master.m3u8'),
              ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pump();

    int updates() =>
        seen.where((http.Request r) => r.url.path.endsWith('/update')).length;

    expect(
      find.text(Strings.applyingPlaybackChanges),
      findsOneWidget,
      reason:
          'a multi-minute POST with no pending state is what M18 exists to prevent',
    );
    expect(updates(), 1);
    expect(
      tester.widget<PlayerSettingsPanel>(find.byType(PlayerSettingsPanel)).busy,
      isTrue,
    );

    await tester.tap(find.text('720p').last, warnIfMissed: false);
    await tester.pump();
    expect(
      updates(),
      1,
      reason:
          'the control that started a renegotiation stays blocked until it settles',
    );

    slow.complete(_renegotiated('transcode', '/stream/first/master.m3u8'));
    await tester.pumpAndSettle();

    expect(fake.attached, '/stream/first/master.m3u8');
    expect(find.text(Strings.applyingPlaybackChanges), findsNothing);
    expect(
      tester.widget<PlayerSettingsPanel>(find.byType(PlayerSettingsPanel)).busy,
      isFalse,
    );

    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('480p').last);
    await tester.pumpAndSettle();
    expect(
      updates(),
      2,
      reason:
          'a gate that never releases would lock the settings for the session',
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a pending server seek holds the settings until it lands', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    bool isSeek(http.Request r) =>
        r.method == 'POST' && r.url.path.endsWith('/seek');
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        sequential: true,
        intercept: (http.Request r) =>
            isSeek(r) ? _seekRestart(88000, null) : null,
        lag: (http.Request r) => const Duration(seconds: 5),
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pump();

    expect(
      seen.where(isSeek).length,
      1,
      reason: 'the stream it would seek in is about to be replaced',
    );

    await tester.tap(find.byIcon(Icons.video_settings));
    await _settleAroundSpinner(tester);
    expect(
      tester.widget<PlayerSettingsPanel>(find.byType(PlayerSettingsPanel)).busy,
      isTrue,
    );
    await tester.tap(
      find.text(Strings.playerOriginalAt(1080)).last,
      warnIfMissed: false,
    );
    await _settleAroundSpinner(tester);
    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/update')),
      isEmpty,
    );

    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();

    expect(fake.attached, '/stream/tok3/master.m3u8');
    expect(fake.attachedPositionMs, 2000);
    expect(
      tester.widget<PlayerSettingsPanel>(find.byType(PlayerSettingsPanel)).busy,
      isFalse,
    );

    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();

    final http.Request update = seen.lastWhere(
      (http.Request r) => r.url.path.endsWith('/update'),
    );
    expect(jsonDecode(update.body)['position_ms'], 90000);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a seek during a pending renegotiation is ignored', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        sequential: true,
        onUpdate: (int _) => slow.future,
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pump();
    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pump();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isEmpty,
    );

    slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await tester.pumpAndSettle();

    expect(fake.attached, '/stream/second/master.m3u8');
    expect(find.text(Strings.applyingPlaybackChanges), findsNothing);
    expect(
      tester.widget<PlayerSettingsPanel>(find.byType(PlayerSettingsPanel)).busy,
      isFalse,
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a renegotiation answered in the background is dropped', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(
      _app(fake, touch: true, onUpdate: (int _) => slow.future),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pump();

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await tester.pump();
    await tester.pump();

    expect(
      fake.attached,
      isNot('/stream/second/master.m3u8'),
      reason: 'that session was ended when the app went to the background',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'a renegotiation answered after coming back does not keep the player busy',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      final Completer<http.Response> slow = Completer<http.Response>();
      await tester.pumpWidget(
        _app(fake, seen: seen, touch: true, onUpdate: (int _) => slow.future),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.tune));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerSubtitles));
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerNoSubtitles));
      await tester.pumpAndSettle();
      await tester.tap(find.textContaining('English').last);
      await tester.pump();

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      await tester.pump();
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      await tester.pump();
      slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
      await tester.pump();
      await tester.pump();

      expect(
        seen.where(
          (http.Request r) =>
              r.method == 'POST' && r.url.path == '/api/v1/sessions',
        ),
        hasLength(2),
      );
      expect(fake.attached, '/stream/tok/file');
      expect(find.text(Strings.applyingPlaybackChanges), findsNothing);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('a server seek answered after the page closed is dropped', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(
      _app(
        fake,
        sequential: true,
        intercept: (http.Request r) =>
            r.method == 'POST' && r.url.path.endsWith('/seek')
            ? _seekRestart(88000, null)
            : null,
        lag: (http.Request r) => const Duration(seconds: 5),
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pump();
    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();

    expect(fake.attached, '/stream/tok/master.m3u8');
  });

  testWidgets('a renegotiation stops the video and shows the spinner', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(_app(fake, onUpdate: (int _) => slow.future));
    await tester.pumpAndSettle();
    fake.emit(playing: true);
    await tester.pump();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    fake.calls.clear();
    await tester.tap(find.text('720p').last);
    await tester.pump();

    expect(
      fake.calls,
      contains('pause'),
      reason: 'the settings they picked are not what the old stream is playing',
    );
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text(Strings.applyingPlaybackChanges), findsOneWidget);

    slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await tester.pumpAndSettle();

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(fake.attachedAutoplay, isFalse);
    expect(
      fake.calls,
      containsAllInOrder(<String>['pause', 'attach:remux', 'play']),
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a renegotiation that never answers offers a way out', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    final List<String> pushed = <String>[];
    final Completer<http.Response> answer = Completer<http.Response>();
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        episodic: true,
        pushed: pushed,
        onUpdate: (int _) => answer.future,
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles).last);
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pump();

    expect(find.text(Strings.applyingPlaybackChanges), findsOneWidget);
    await tester.pump(Duration(seconds: _stallDelay - 1));
    expect(
      find.text(Strings.playerGoBack),
      findsNothing,
      reason: 'a change that is merely slow keeps the plain spinner',
    );

    await tester.pump(const Duration(seconds: 2));
    expect(find.text(Strings.playerGoBack), findsOneWidget);
    expect(find.text(Strings.playerKeepWaiting), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    await tester.tap(find.text(Strings.playerGoBack));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await tester.pump();

    expect(pushed.single, episodeRoute('e1', series: 's1'));
    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
      ),
      isTrue,
    );

    fake.calls.clear();
    answer.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await tester.pump(const Duration(seconds: 1));

    expect(
      fake.calls.where((String c) => c.startsWith('attach')),
      isEmpty,
      reason: 'an answer that arrives after leaving attaches nothing',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a failed renegotiation resumes the stream it paused', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(
      _app(
        fake,
        onUpdate: (int _) => Future<http.Response>.error(Exception('down')),
        toasts: true,
      ),
    );
    await tester.pumpAndSettle();
    fake.emit(playing: true);
    await tester.pump();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    fake.calls.clear();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.renegotiationFailed), findsOneWidget);
    expect(fake.calls, <String>[
      'pause',
      'play',
    ], reason: 'a failure must not leave the player stopped on a live stream');

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('a paused video is not resumed by a failed renegotiation', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(
      _app(
        fake,
        onUpdate: (int _) => Future<http.Response>.error(Exception('down')),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.pause));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    fake.calls.clear();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();

    expect(fake.calls, <String>['pause']);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('flushes a final progress on teardown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    int progressPosts() => seen
        .where(
          (http.Request r) =>
              r.method == 'POST' &&
              r.url.path == '/api/v1/sessions/s1/progress',
        )
        .length;

    expect(progressPosts(), 1, reason: 'playing, once the stream loads');

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();

    expect(progressPosts(), 2);
  });

  testWidgets('a dialog open across an app switch does not keep playback from '
      'coming back', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, touch: true));
    await tester.pumpAndSettle();
    final BuildContext page = tester.element(find.byType(WatchBody));
    unawaited(
      showDialog<void>(
        context: page,
        builder: (BuildContext _) => const SizedBox(),
      ),
    );
    await tester.pumpAndSettle();

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    Navigator.of(page).pop();
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await _shown(tester);

    expect(
      seen.where(
        (http.Request r) =>
            r.method == 'POST' && r.url.path == '/api/v1/sessions',
      ),
      hasLength(2),
    );
    expect(find.byType(PlayerFrame), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('an app switch under a page route starts nothing until the '
      'route is popped', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, touch: true));
    await tester.pumpAndSettle();
    final BuildContext page = tester.element(find.byType(WatchBody));
    unawaited(
      Navigator.of(page).push(
        MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
      ),
    );
    await tester.pumpAndSettle();
    Iterable<http.Request> starts() => seen.where(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/sessions',
    );

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(starts(), hasLength(1));

    Navigator.of(page).pop();
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    await _shown(tester);
    expect(starts(), hasLength(2));

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('releases playback when another route covers the page', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    bool sessionEnded() => seen.any(
      (http.Request r) =>
          r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
    );

    expect(sessionEnded(), isFalse);

    Navigator.of(tester.element(find.byType(WatchBody))).push(
      MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
    );
    await tester.pumpAndSettle();

    expect(fake.calls, contains('pause'));
    expect(sessionEnded(), isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  group('a start that outlives the player', () {
    bool startRequest(http.Request r) =>
        r.method == 'POST' && r.url.path == '/api/v1/sessions';

    Widget slowStart(FakePlayerController fake, List<http.Request> seen) =>
        _app(
          fake,
          seen: seen,
          intercept: (http.Request r) => startRequest(r) ? _route(r) : null,
          lag: (http.Request r) => const Duration(seconds: 5),
        );

    int ends(List<http.Request> seen) => seen
        .where(
          (http.Request r) =>
              r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
        )
        .length;

    int beats(List<http.Request> seen) => seen
        .where(
          (http.Request r) =>
              r.method == 'POST' &&
              r.url.path == '/api/v1/sessions/s1/progress',
        )
        .length;

    testWidgets('a session that answers after leaving is ended unplayed', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(slowStart(fake, seen));
      await tester.pump(const Duration(seconds: 1));

      expect(seen.where(startRequest), hasLength(1));
      expect(ends(seen), 0);

      await tester.pumpWidget(const SizedBox());
      await tester.pump(const Duration(seconds: 5));

      expect(ends(seen), 1);
      expect(fake.calls.where((String c) => c.startsWith('attach')), isEmpty);

      await tester.pump(const Duration(seconds: 60));

      expect(beats(seen), 0);
    });

    testWidgets('leaving while the stream attaches never starts a heartbeat', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController()
        ..attachGate = Completer<void>();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(_app(fake, seen: seen, intercept: _resumeAt40s));
      await tester.pump(const Duration(seconds: 1));

      expect(fake.calls, contains('attach:direct'));

      await tester.pumpWidget(const SizedBox());
      await tester.pump();

      expect(ends(seen), 1);
      final int flushed = beats(seen);

      fake.attachGate!.complete();
      await tester.pump(const Duration(seconds: 60));

      expect(beats(seen), flushed);
      expect(ends(seen), 1);
      expect(_reported(seen), everyElement(40000));
    });

    testWidgets('a route covering a starting player ends the late session', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(slowStart(fake, seen));
      await tester.pump(const Duration(seconds: 1));

      final NavigatorState navigator = Navigator.of(
        tester.element(find.byType(WatchBody)),
      );
      navigator.push(
        MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
      );
      await tester.pump(const Duration(seconds: 5));

      expect(ends(seen), 1);
      expect(fake.calls.where((String c) => c.startsWith('attach')), isEmpty);

      navigator.pop();
      await tester.pump(const Duration(seconds: 6));

      expect(seen.where(startRequest), hasLength(2));
      expect(fake.calls.where((String c) => c.startsWith('attach')), [
        'attach:direct',
      ]);

      await tester.pumpWidget(const SizedBox());
      await tester.pump(const Duration(seconds: 6));
    });
  });

  testWidgets('a retry after a failed attach keeps the resume point', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..failedAttaches = 1;
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, intercept: _resumeAt40s));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();

    expect(_reported(seen), isNotEmpty);
    expect(_reported(seen), everyElement(40000));
  });

  testWidgets('progress keeps the resume point until the player plays', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, intercept: _resumeAt40s));
    await tester.pumpAndSettle();

    fake.opening();
    await tester.pump();

    expect(_reported(seen), isNotEmpty);
    expect(_reported(seen), everyElement(40000));

    fake.at(positionMs: 41000);
    await tester.pump();
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();

    expect(_reported(seen).last, 41000);
  });

  testWidgets('a retry ends the failed session before starting another', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..failedAttaches = 1;
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerFailed), findsOneWidget);

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    final List<String> lifecycle = seen
        .where(
          (http.Request r) =>
              (r.method == 'POST' && r.url.path == '/api/v1/sessions') ||
              r.method == 'DELETE',
        )
        .map((http.Request r) => '${r.method} ${r.url.path}')
        .toList();
    expect(lifecycle, <String>[
      'POST /api/v1/sessions',
      'DELETE /api/v1/sessions/s1',
      'POST /api/v1/sessions',
    ]);
    expect(fake.attached, '/stream/tok/file');

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('starts a fresh session when the page is uncovered', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    int sessionStarts() => seen
        .where(
          (http.Request r) =>
              r.method == 'POST' && r.url.path == '/api/v1/sessions',
        )
        .length;

    expect(sessionStarts(), 1);

    final NavigatorState navigator = Navigator.of(
      tester.element(find.byType(WatchBody)),
    );
    navigator.push(
      MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
    );
    await tester.pumpAndSettle();
    navigator.pop();
    await tester.pumpAndSettle();

    expect(sessionStarts(), 2);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('backgrounding a touch player ends the session', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, touch: true));
    await tester.pumpAndSettle();

    bool sessionEnded() => seen.any(
      (http.Request r) =>
          r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
    );

    expect(sessionEnded(), isFalse);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();

    // Pausing does not stop the player reading ahead, so it has to be detached
    // as well or it walks the rest of the playlist against a dead session.
    expect(fake.calls, containsAllInOrder(<String>['pause', 'detach']));
    expect(sessionEnded(), isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('returning from the background restarts the session paused', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, touch: true));
    await tester.pumpAndSettle();

    int sessionStarts() => seen
        .where(
          (http.Request r) =>
              r.method == 'POST' && r.url.path == '/api/v1/sessions',
        )
        .length;

    expect(sessionStarts(), 1);
    expect(fake.calls.last, 'play');

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    fake.calls.clear();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(sessionStarts(), 2);
    expect(fake.calls, <String>['attach:direct']);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('being uncovered by a route still resumes playing', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();

    final NavigatorState navigator = Navigator.of(
      tester.element(find.byType(WatchBody)),
    );
    navigator.push(
      MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
    );
    await tester.pumpAndSettle();
    fake.calls.clear();
    navigator.pop();
    await tester.pumpAndSettle();

    expect(fake.calls, <String>['attach:direct', 'play']);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  // An incoming call or a pulled-down notification shade must not cost the
  // session, so only a full background counts.
  testWidgets('going inactive leaves the session alone', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen, touch: true));
    await tester.pumpAndSettle();

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pumpAndSettle();

    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
      ),
      isFalse,
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  // Minimising a desktop window or hiding a browser tab keeps playing.
  testWidgets('backgrounding leaves a non-touch player alone', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();

    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
      ),
      isFalse,
    );
    expect(fake.calls, isNot(contains('pause')));

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  // The escape hatch is only useful if it survives the round trip, so assert
  // it on the wire rather than on the widget that set it.
  testWidgets('a delivery preference is sent when the session starts', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        controls: const PlaybackControls(delivery: DeliveryPreference.always),
      ),
    );
    await tester.pumpAndSettle();

    final http.Request start = seen.firstWhere(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/sessions',
    );
    expect(
      (jsonDecode(start.body) as Map<String, dynamic>)['delivery'],
      'always',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  group('the saved converting default', () {
    Future<String?> startedWith(
      WidgetTester tester, {
      PlaybackControls controls = const PlaybackControls(),
    }) async {
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(
        _app(FakePlayerController(), seen: seen, controls: controls),
      );
      await tester.pumpAndSettle();
      final http.Request start = seen.firstWhere(
        (http.Request r) =>
            r.method == 'POST' && r.url.path == '/api/v1/sessions',
      );
      return (jsonDecode(start.body) as Map<String, dynamic>)['delivery']
          as String?;
    }

    testWidgets('starts the session when the video chose nothing', (
      WidgetTester tester,
    ) async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.player.delivery': 'always',
      });

      expect(await startedWith(tester), 'always');

      await tester.tap(find.byIcon(Icons.settings).first);
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: find.byType(AppDropdown<DeliveryPreference?>),
          matching: find.text(Strings.playerDeliveryAlways),
        ),
        findsOneWidget,
      );

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('gives way to a choice made for this video', (
      WidgetTester tester,
    ) async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.player.delivery': 'always',
      });

      expect(
        await startedWith(
          tester,
          controls: const PlaybackControls(delivery: DeliveryPreference.auto),
        ),
        isNull,
      );

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('is not overridden by the last video', (
      WidgetTester tester,
    ) async {
      final List<String> pushed = <String>[];
      await tester.pumpWidget(
        _app(
          FakePlayerController(),
          episodic: true,
          pushed: pushed,
          controls: const PlaybackControls(delivery: DeliveryPreference.never),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.skip_next));
      await tester.pumpAndSettle();

      expect(
        Uri.parse(pushed.single).queryParameters.containsKey('delivery'),
        isFalse,
      );
    });
  });

  group('the player draws only the subtitles the server sent', () {
    http.Response started(
      String mode,
      String manifest,
      String? delivery, {
      bool sequential = false,
    }) => http.Response(
      jsonEncode(<String, dynamic>{
        'session_id': 's1',
        'mode': mode,
        'manifest_url': manifest,
        'sequential': sequential,
        'heartbeat_interval_s': 10,
        'selected': <String, dynamic>{
          'audio_track': 1,
          if (delivery != null)
            'subtitle_track': <String, dynamic>{'type': 'embedded', 'index': 2},
          'subtitle_delivery': delivery,
        },
        'trickplay': <dynamic>[],
      }),
      201,
    );

    bool isStart(http.Request r) =>
        r.method == 'POST' && r.url.path == '/api/v1/sessions';

    testWidgets("a direct file with nothing chosen hides the file's own", (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      expect(fake.attachedSubtitles, <bool>[false]);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('a subtitle sent as WebVTT is drawn', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(
        _app(
          fake,
          intercept: (http.Request r) => isStart(r)
              ? started('remux', '/stream/tok/master.m3u8', 'hls_vtt')
              : null,
        ),
      );
      await tester.pumpAndSettle();

      expect(fake.attachedSubtitles, <bool>[true]);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('a burned subtitle is not drawn a second time', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(
        _app(
          fake,
          intercept: (http.Request r) => isStart(r)
              ? started('transcode', '/stream/tok/master.m3u8', 'burned')
              : null,
        ),
      );
      await tester.pumpAndSettle();

      expect(fake.attachedSubtitles, <bool>[false]);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('turning subtitles off re-attaches without them', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(
        _app(
          fake,
          intercept: (http.Request r) => isStart(r)
              ? started('remux', '/stream/tok/master.m3u8', 'hls_vtt')
              : null,
          onUpdate: (int _) => Future<http.Response>.value(
            _renegotiated('direct', '/stream/tok2/file'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.subtitles).first);
      await tester.pumpAndSettle();
      await tester.tap(find.textContaining('English').last);
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerNoSubtitles).last);
      await tester.pumpAndSettle();

      expect(fake.attachedSubtitles, <bool>[true, false]);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });

    testWidgets('a seek restart keeps the subtitle it was drawing', (
      WidgetTester tester,
    ) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(
        _app(
          fake,
          sequential: true,
          seekOriginMs: 88000,
          intercept: (http.Request r) {
            if (isStart(r)) {
              return started(
                'remux',
                '/stream/tok/master.m3u8',
                'hls_vtt',
                sequential: true,
              );
            }
            if (r.url.path.endsWith('/seek')) {
              return http.Response(
                jsonEncode(<String, dynamic>{
                  'manifest_url': '/stream/tok3/master.m3u8',
                  'mode': 'remux',
                  'origin_ms': 88000,
                  'sequential': true,
                  'selected': <String, dynamic>{
                    'audio_track': 1,
                    'subtitle_track': <String, dynamic>{
                      'type': 'embedded',
                      'index': 2,
                    },
                    'subtitle_delivery': 'hls_vtt',
                  },
                }),
                200,
              );
            }
            return null;
          },
        ),
      );
      await tester.pumpAndSettle();

      fake.at(positionMs: 1000);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
      await tester.pumpAndSettle();

      expect(fake.attached, '/stream/tok3/master.m3u8');
      expect(fake.attachedSubtitles, <bool>[true, true]);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    });
  });

  testWidgets('space toggles playback from the keyboard', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(fake.calls, <String>['pause']);

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(fake.calls, <String>['pause', 'play']);
  });

  testWidgets('the shortcuts survive a trip into the settings panel', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();

    await tester.tapAt(const Offset(40, 40));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();

    expect(
      fake.calls,
      contains('pause'),
      reason: 'tabbing into the panel must not cost the player its keyboard',
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the playback log shows every line the player noted', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..logLines = <String>[
        '09:49:50 waiting at 228.0/596.0 s, buffered 0.0-532.0',
        '09:49:52 ended at 231.4/596.0 s, buffered 0.0-532.0, hidden',
      ];
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.byType(OverlayBar),
        matching: find.byIcon(PlayerPanel.settings.icon),
      ),
    );
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.text(Strings.playerLogShow));
    await tester.tap(find.text(Strings.playerLogShow));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerLog), findsOneWidget);
    expect(find.text(fake.logLines.join('\n')), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('an empty playback log says so', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.byType(OverlayBar),
        matching: find.byIcon(PlayerPanel.settings.icon),
      ),
    );
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.text(Strings.playerLogShow));
    await tester.tap(find.text(Strings.playerLogShow));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerLogEmpty), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the playback log sends a report only once the viewer has seen '
      'it', (WidgetTester tester) async {
    await TokenStore().save(
      const AuthTokens(accessToken: 'a1', refreshToken: 'r1', playerId: 'p-9'),
    );
    final List<http.Request> seen = <http.Request>[];
    final FakePlayerController fake = FakePlayerController()
      ..logLines = <String>['09:49:50 waiting at 228.0/596.0 s, buffered none'];
    bool isReport(http.Request r) =>
        r.url.path == '/api/v1/diagnostics/reports';
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        toasts: true,
        intercept: (http.Request r) => isReport(r)
            ? http.Response(jsonEncode(<String, String>{'id': 'r1'}), 201)
            : null,
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.byType(OverlayBar),
        matching: find.byIcon(PlayerPanel.settings.icon),
      ),
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text(Strings.playerLogShow));
    await tester.tap(find.text(Strings.playerLogShow));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.sendReport));
    await tester.pumpAndSettle();

    expect(find.text(Strings.reportTitle), findsOneWidget);
    expect(find.text(Strings.reportHistory), findsOneWidget);
    expect(find.textContaining('09:49:50 waiting'), findsOneWidget);
    expect(seen.where(isReport), isEmpty, reason: 'nothing goes before Send');

    await tester.enterText(
      find.descendant(
        of: find.byType(ReportDialog),
        matching: find.byType(TextField),
      ),
      ' stutters ',
    );
    await tester.tap(find.text(Strings.reportSend));
    await tester.pumpAndSettle();

    final Map<String, dynamic> body =
        jsonDecode(seen.singleWhere(isReport).body) as Map<String, dynamic>;
    expect(body['player_id'], 'p-9');
    expect(body['app'], 'flutter');
    expect(body['app_version'], '0.0.7');
    expect(body['note'], 'stutters');
    final Map<String, dynamic> payload =
        body['payload'] as Map<String, dynamic>;
    expect(payload['log'], fake.logLines);
    expect((payload['history'] as List<dynamic>).first['event'], 'opened');
    expect(payload['panel'], isNotEmpty);
    expect(find.text(Strings.toastReportSent), findsOneWidget);
    expect(find.text(Strings.reportTitle), findsNothing);

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('escape closes the panel before it touches full screen', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerQuality), findsOneWidget);
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerQuality), findsNothing);
    expect(
      fake.calls,
      isNot(contains('fullscreen')),
      reason: 'the panel is the nearer thing to back out of',
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('escape leaves full screen, and does nothing outside it', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    expect(
      fake.calls,
      isNot(contains('fullscreen')),
      reason: 'a windowed player has nothing to leave',
    );

    await tester.tap(find.byIcon(Icons.fullscreen));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(fake.calls, contains('fullscreen'));

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('the question mark opens the legend', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    // The panel used to be the only way in, which no keyboard user could find.
    // "?" has no physical key to simulate, so this sends the slash it sits on.
    await tester.sendKeyEvent(LogicalKeyboardKey.slash);
    await tester.pumpAndSettle();

    expect(find.byType(ShortcutsDialog), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('the shortcuts keep working after focus goes elsewhere', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    tester.binding.focusManager.primaryFocus?.unfocus();
    await tester.pumpAndSettle();

    fake.calls.clear();
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(
      fake.calls,
      _noPlayOrPause,
      reason: 'out of the focus chain, the handler hears nothing',
    );

    await tester.tap(find.byType(PlayerFrame));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(
      fake.calls,
      anyOf(contains('play'), contains('pause')),
      reason: 'touching the player has to put the keys back in reach',
    );
  });

  testWidgets('focus brings the hidden chrome back', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    final Finder chrome = find
        .ancestor(
          of: find.byIcon(Icons.settings).first,
          matching: find.byType(AnimatedOpacity),
        )
        .first;

    // The chrome only auto-hides while something is actually playing.
    fake.emit(playing: true);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 3200));
    expect(tester.widget<AnimatedOpacity>(chrome).opacity, 0);

    // Tabbing onto a control the user cannot see is what this prevents.
    Focus.of(tester.element(find.byIcon(Icons.settings).first)).requestFocus();
    await tester.pumpAndSettle();

    expect(tester.widget<AnimatedOpacity>(chrome).opacity, 1);
  });

  testWidgets('the arrow keys seek by ten seconds', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.emit(playing: true);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();

    expect(fake.seekedMs, 10000);
  });

  testWidgets('a digit jumps to its tenth of the runtime', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.emit(playing: true);
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.digit5);
    await tester.pumpAndSettle();

    expect(fake.seekedMs, 50000);
  });

  testWidgets('the settings panel opens the legend, listing every shortcut', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.keyboard));
    await tester.pumpAndSettle();

    expect(
      find.byType(ShortcutsDialog),
      findsOneWidget,
      reason: 'the panel is the only route in, so it has to work',
    );
    for (final ShortcutRow row in kPlayerShortcuts) {
      expect(find.text(row.label), findsOneWidget);
    }

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('typing in a player field is not read as a shortcut', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles).last);
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pumpAndSettle();

    await tester.tap(find.byType(TextField));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.keyM);
    await tester.pumpAndSettle();

    expect(
      fake.calls.where((String c) => c.startsWith('muted:')),
      isEmpty,
      reason:
          'the subtitle offset field takes letters and digits, so shortcuts '
          'have to stand down while a text field holds focus',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('raising the volume out of a mute is remembered as unmuted', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.volume_up));
    await tester.pumpAndSettle();
    expect((await SharedPreferences.getInstance()).getBool(_mutedKey), isTrue);

    await tester.drag(find.byType(Slider), const Offset(40, 0));
    await tester.pumpAndSettle();

    expect(
      (await SharedPreferences.getInstance()).getBool(_mutedKey),
      isFalse,
      reason:
          'the slider unmutes through setVolume, so the stored flag has to '
          'follow or the next episode starts silent again',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('finishing an episode offers the next one', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();
    await _settled(tester);

    expect(find.byType(UpNextCard), findsNothing);

    fake.emit(ended: true);
    await _shown(tester);

    expect(find.byType(UpNextCard), findsOneWidget);
    expect(find.text('S01E02 · Second'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('autoplay set to immediately opens the next episode at once', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.autoplay': kAutoplayImmediately,
    });
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.emit(ended: true);
    await tester.pump();

    expect(find.byType(UpNextCard), findsNothing);

    await tester.pumpAndSettle();

    expect(pushed.single, startsWith(watchRoute('v2')));
  });

  testWidgets('autoplay with the playback log open replaces the watch page', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.autoplay': kAutoplayImmediately,
    });
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();
    await tester.tap(
      find.descendant(
        of: find.byType(OverlayBar),
        matching: find.byIcon(PlayerPanel.settings.icon),
      ),
    );
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text(Strings.playerLogShow));
    await tester.tap(find.text(Strings.playerLogShow));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playerLog), findsOneWidget);
    await _settled(tester);

    fake.emit(ended: true);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerLog), findsNothing);
    expect(pushed.single, startsWith(watchRoute('v2')));
    expect(find.byType(WatchBody, skipOffstage: false), findsNothing);
  });

  testWidgets(
    'autoplay leaves a page opened while the next one was looked up',
    (WidgetTester tester) async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.player.autoplay': kAutoplayImmediately,
      });
      final FakePlayerController fake = FakePlayerController();
      final List<String> pushed = <String>[];
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(
        _app(
          fake,
          seen: seen,
          episodic: true,
          pushed: pushed,
          intercept: (http.Request r) =>
              r.url.path.endsWith('/versions') ? _episodic(r.url.path) : null,
          lag: (http.Request r) => const Duration(seconds: 1),
        ),
      );
      await tester.pumpAndSettle();
      await _settled(tester);

      fake.emit(ended: true);
      await tester.pump();
      unawaited(
        Navigator.of(tester.element(find.byType(WatchBody))).pushNamed('/away'),
      );
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();

      expect(pushed, <String>['/away']);
      expect(find.byType(WatchBody), findsNothing);
      expect(
        seen.where(
          (http.Request r) =>
              r.method == 'POST' && r.url.path == '/api/v1/sessions',
        ),
        hasLength(1),
      );
    },
  );

  testWidgets(
    'choosing immediately during the countdown opens the next episode',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<String> pushed = <String>[];
      await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
      await tester.pumpAndSettle();
      await _settled(tester);

      fake.emit(ended: true);
      await _shown(tester);
      expect(find.byType(UpNextCard), findsOneWidget);

      await tester.tap(find.byIcon(Icons.settings).first);
      await tester.pumpAndSettle();
      await tester.tap(
        find.text(autoplayLabel(const LocalSettings().autoplayNextS)).first,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(Strings.playerAutoplayImmediately).last);
      await tester.pumpAndSettle();

      expect(
        pushed.single,
        startsWith(watchRoute('v2')),
        reason: 'Immediately is a choice to move on now, not to stop autoplay',
      );
    },
  );

  testWidgets('turning autoplay off and on at the end counts down the new '
      'delay', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();
    await _settled(tester);
    fake.emit(ended: true);
    await _shown(tester);
    expect(find.byType(UpNextCard), findsOneWidget);

    Finder inPanel(String text) => find.descendant(
      of: find.byType(PlayerSettingsPanel),
      matching: find.text(text),
    );
    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    await tester.tap(
      inPanel(autoplayLabel(const LocalSettings().autoplayNextS)).first,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerAutoplayOff).last);
    await tester.pumpAndSettle();
    expect(find.byType(UpNextCard), findsNothing);

    await tester.tap(inPanel(Strings.playerAutoplayOff).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(autoplayLabel(30)).last);
    await tester.pump();
    await _shown(tester);

    expect(
      tester.widget<UpNextCard>(find.byType(UpNextCard)).remainingSeconds,
      30,
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('cancelling stays cancelled while the video rests on its end', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.emit(ended: true);
    await _shown(tester);
    await tester.tap(find.text(Strings.cancel));
    await tester.pump();

    expect(find.byType(UpNextCard), findsNothing);

    fake.emit(ended: true);
    await _shown(tester);

    expect(
      find.byType(UpNextCard),
      findsNothing,
      reason:
          'ended stays true on the last frame, so arming has to latch or every '
          'snapshot reopens the card the viewer just dismissed',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('playing the current episode again calls off the countdown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.emit(ended: true);
    await _shown(tester);
    expect(find.byType(UpNextCard), findsOneWidget);

    fake.emit(playing: true);
    await _shown(tester);

    expect(
      find.byType(UpNextCard),
      findsNothing,
      reason: 'choosing to keep watching is a decision not to move on',
    );

    await tester.pump(const Duration(seconds: 12));

    expect(find.byType(UpNextCard), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a failure that comes after the countdown armed calls it off', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.emit(ended: true);
    await _shown(tester);
    expect(find.byType(UpNextCard), findsOneWidget);

    fake.fail('boom');
    await tester.pump(const Duration(seconds: 1));
    await _shown(tester);
    expect(find.byType(UpNextCard), findsNothing);

    await tester.pump(const Duration(seconds: 12));
    expect(pushed, isEmpty);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('seeking back from the end calls off the countdown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.emit(ended: true);
    await _shown(tester);
    expect(find.byType(UpNextCard), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await _shown(tester);
    expect(find.byType(UpNextCard), findsNothing);

    await tester.pump(const Duration(seconds: 12));
    expect(pushed, isEmpty);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('space at the end replays from the start, as the centre button '
      'does', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    await _settled(tester);
    fake.emit(ended: true);
    await _shown(tester);
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pump();

    expect(fake.calls, <String>['seek', 'play']);
    expect(fake.seekedMs, 0);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a player that is already fullscreen makes the page immersive', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController(fullscreen: true);
    final ScopedValue<bool> immersive = ScopedValue<bool>(false);
    addTearDown(immersive.dispose);

    await tester.pumpWidget(
      ImmersiveScope(immersive: immersive, child: _app(fake, touch: true)),
    );
    await tester.pumpAndSettle();

    expect(immersive.value, isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a phone sent to the background while the page gathers starts '
      'only once it comes back', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        touch: true,
        intercept: (http.Request r) =>
            r.url.path == '/api/v1/users/u1/progress/v1' ? _route(r) : null,
        lag: (http.Request r) => const Duration(seconds: 1),
      ),
    );
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();

    bool started() => seen.any(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/sessions',
    );
    expect(started(), isFalse);
    expect(fake.calls, isNot(contains('play')));

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(started(), isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a player that is already fullscreen makes the page immersive '
      'while it gathers', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController(fullscreen: true);
    final ScopedValue<bool> immersive = ScopedValue<bool>(false);
    addTearDown(immersive.dispose);

    await tester.pumpWidget(
      ImmersiveScope(
        immersive: immersive,
        child: _gathering(fake, touch: true),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(immersive.value, isTrue);

    await tester.pump(const Duration(seconds: 2));
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a gather that fails leaves immersive mode for its error block', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController(fullscreen: true);
    final ScopedValue<bool> immersive = ScopedValue<bool>(false);
    addTearDown(immersive.dispose);

    await tester.pumpWidget(
      ImmersiveScope(
        immersive: immersive,
        child: _app(
          fake,
          touch: true,
          intercept: (http.Request r) => r.url.path == '/api/v1/versions/v1'
              ? http.Response('', 500)
              : null,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.couldNotStartPlayback), findsOneWidget);
    expect(immersive.value, isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a popup open while the page gathers does not stop the start', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_gathering(fake, seen: seen, touch: true));
    await tester.pump();
    final BuildContext page = tester.element(find.byType(WatchBody));
    unawaited(
      showDialog<void>(
        context: page,
        builder: (BuildContext _) => const SizedBox(),
      ),
    );
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();

    Navigator.of(page).pop();
    await tester.pumpAndSettle();

    expect(_started(seen), isTrue);
    expect(find.byType(PlayerFrame), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a page pushed over the player while it gathers holds the start '
      'until it is popped', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_gathering(fake, seen: seen));
    await tester.pump();
    final BuildContext page = tester.element(find.byType(WatchBody));
    unawaited(
      Navigator.of(page).push(
        MaterialPageRoute<void>(builder: (BuildContext _) => const SizedBox()),
      ),
    );
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(_started(seen), isFalse);

    Navigator.of(page).pop();
    await tester.pumpAndSettle();
    expect(_started(seen), isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('coming back before the page has gathered starts once it has', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_gathering(fake, seen: seen, touch: true));
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    expect(_started(seen), isFalse);

    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(_started(seen), isTrue);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('leaving while the page gathers starts nothing', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_gathering(fake, seen: seen));
    await tester.pump();

    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();

    expect(_started(seen), isFalse);
    expect(fake.calls, isNot(contains('attach:direct')));
  });

  testWidgets('a movie gets no episode transport controls', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.skip_next), findsNothing);
    expect(find.byIcon(Icons.skip_previous), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'a stream with nothing to show yet says so instead of looking stuck',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      expect(find.text(Strings.playerLoading), findsNothing);

      fake.stall(positionMs: 0, ready: false);
      await _shown(tester);

      expect(
        find.text(Strings.playerLoading),
        findsOneWidget,
        reason: 'a 4K transcode can take seconds to produce its first frames',
      );
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      fake.emit(playing: true);
      await tester.pumpAndSettle();

      expect(find.text(Strings.playerLoading), findsNothing);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('stalling part way through reads as buffering, not loading', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.stall(positionMs: 40000);
    await _shown(tester);

    expect(find.text(Strings.playerBuffering), findsOneWidget);
    expect(find.text(Strings.playerLoading), findsNothing);

    fake.emit(playing: true);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerBuffering), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'a stream that never arrives says so instead of spinning forever',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      fake.stall(positionMs: 0, ready: false);
      await _shown(tester);
      expect(find.text(Strings.playerLoading), findsOneWidget);

      await tester.pump(Duration(seconds: _stallDelay - 1));
      expect(
        find.text(Strings.playerTooSlow),
        findsNothing,
        reason: 'a slow but working start must not be called a failure',
      );

      await tester.pump(const Duration(seconds: 2));

      expect(find.text(Strings.playerTooSlow), findsOneWidget);
      expect(find.text(Strings.playerTooSlowHint), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(find.text(Strings.playerGoBack), findsOneWidget);

      await tester.tap(find.text(Strings.playerKeepWaiting));
      await tester.pump();

      expect(
        find.text(Strings.playerTooSlow),
        findsNothing,
        reason: 'dismissing returns to the plain wait rather than giving up',
      );
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      await tester.pump(Duration(seconds: _stallDelay + 1));
      expect(
        find.text(Strings.playerTooSlow),
        findsOneWidget,
        reason:
            'dismissing restarts the clock, so a viewer who waits is asked '
            'again',
      );

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('a stream that is merely slow clears the warning once it moves', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    fake.stall(positionMs: 0, ready: false);
    await tester.pump(Duration(seconds: _stallDelay + 1));
    expect(find.text(Strings.playerTooSlow), findsOneWidget);

    fake.stall(positionMs: 0, ready: false, bufferedAheadMs: 500);
    await tester.pump(const Duration(seconds: 2));

    expect(
      find.text(Strings.playerTooSlow),
      findsNothing,
      reason: 'any forward movement shows the pipeline is alive',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('going back from a stalled player lands on the title page', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();

    fake.stall(positionMs: 0, ready: false);
    await tester.pump(Duration(seconds: _stallDelay + 1));
    expect(find.text(Strings.playerTooSlow), findsOneWidget);

    await tester.tap(find.text(Strings.playerGoBack));
    await tester.pumpAndSettle();

    expect(
      pushed.single,
      episodeRoute('e1', series: 's1'),
      reason:
          'the viewer came from the title page and that is where the way '
          'out belongs',
    );
  });

  testWidgets('an error that playback moves past is never shown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pump();
    fake.fail('Could not open codec.');
    await tester.pump();
    fake.at(positionMs: 3000);
    await tester.pump(const Duration(seconds: 2));
    fake.stall(positionMs: 3000);
    await tester.pump(Duration(seconds: _stallDelay + 1));

    expect(find.text(Strings.playerTooSlow), findsOneWidget);
    expect(find.text(Strings.playerFailed), findsNothing);
    expect(find.text('Could not open codec.'), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('an error then nothing moving for the stall time is a failure', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    fake.stall(positionMs: 0, ready: false);
    await tester.pump();
    fake.fail('Failed to open the stream.');
    await tester.pump();

    expect(find.text(Strings.playerLoading), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text('Failed to open the stream.'), findsNothing);

    await tester.pump(Duration(seconds: _stallDelay - 1));
    expect(find.text(Strings.playerFailed), findsNothing);

    await tester.pump(const Duration(seconds: 2));

    expect(find.text(Strings.playerFailed), findsOneWidget);
    expect(find.text('Failed to open the stream.'), findsOneWidget);
    expect(find.text(Strings.playerTooSlow), findsNothing);
    expect(find.text(Strings.dismiss), findsOneWidget);
    expect(find.text(Strings.playerKeepWaiting), findsNothing);
    expect(find.text(Strings.playerGoBack), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('dismissing a failure waits again, and a new error counts', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.stall(positionMs: 0, ready: false);
    await tester.pump();
    fake.fail('first');
    await tester.pump(Duration(seconds: _stallDelay + 1));
    expect(find.text(Strings.playerFailed), findsOneWidget);

    await tester.tap(find.text(Strings.dismiss));
    await tester.pump();

    expect(find.text(Strings.playerFailed), findsNothing);
    expect(find.byType(CircularProgressIndicator), findsOneWidget);

    await tester.pump(Duration(seconds: _stallDelay + 1));

    expect(find.text(Strings.playerTooSlow), findsOneWidget);
    expect(find.text('first'), findsNothing);

    fake.fail('second');
    await tester.pump();

    expect(find.text(Strings.playerFailed), findsOneWidget);
    expect(find.text('second'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a failure clears once playback moves again', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.stall(positionMs: 0, ready: false);
    await tester.pump();
    fake.fail('gone');
    await tester.pump(Duration(seconds: _stallDelay + 1));
    expect(find.text(Strings.playerFailed), findsOneWidget);

    fake.at(positionMs: 2000);
    await tester.pump(const Duration(seconds: 2));

    expect(find.text(Strings.playerFailed), findsNothing);
    expect(find.text('gone'), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a new stream forgets the errors of the one before', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.at(positionMs: 1000);
    await tester.pump();
    fake.fail('old stream');
    await tester.pump();

    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('720p').last);
    await tester.pumpAndSettle();
    fake.stall(positionMs: 1000);
    await tester.pump(Duration(seconds: _stallDelay + 1));

    expect(find.text(Strings.playerTooSlow), findsOneWidget);
    expect(find.text(Strings.playerFailed), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('an error with the picture frozen and no spinner is a failure', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.at(positionMs: 5000);
    await tester.pump(const Duration(seconds: 2));
    fake.fail('decode error');
    await tester.pump();

    await tester.pump(Duration(seconds: _stallDelay - 1));
    expect(find.text(Strings.playerFailed), findsNothing);

    await tester.pump(const Duration(seconds: 2));

    expect(find.text(Strings.playerFailed), findsOneWidget);
    expect(find.text('decode error'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a paused player holds an error until it plays again', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.pausedAt(positionMs: 5000);
    await tester.pump(const Duration(seconds: 2));
    fake.fail('decode error');
    await tester.pump();

    await tester.pump(Duration(seconds: _stallDelay + 5));
    expect(find.text(Strings.playerFailed), findsNothing);

    fake.at(positionMs: 5000);
    await tester.pump(Duration(seconds: _stallDelay - 1));
    expect(find.text(Strings.playerFailed), findsNothing);

    await tester.pump(const Duration(seconds: 3));

    expect(find.text(Strings.playerFailed), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'an end straight after an error is a failure, not the next episode',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      final List<String> pushed = <String>[];
      await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
      await tester.pumpAndSettle();
      await _settled(tester);
      fake.at(positionMs: 40000);
      await tester.pump();
      fake.fail('stream died');
      await tester.pump();

      fake.endedAt(positionMs: 40000);
      await tester.pump(const Duration(seconds: 1));

      expect(find.text(Strings.playerFailed), findsOneWidget);
      expect(find.text('stream died'), findsOneWidget);
      expect(find.byType(UpNextCard), findsNothing);

      await tester.tap(find.text(Strings.dismiss));
      await tester.pump();
      fake.endedAt(positionMs: 40000);
      await tester.pump(const Duration(seconds: 15));

      expect(find.text(Strings.playerFailed), findsNothing);
      expect(find.byType(UpNextCard), findsNothing);
      expect(pushed, isEmpty);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('an end that playback reached after an error is a plain end', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();
    await _settled(tester);
    fake.at(positionMs: 40000);
    await tester.pump();
    fake.fail('Could not open codec.');
    await tester.pump();

    fake.emit(ended: true);
    await tester.pump(const Duration(seconds: 1));

    expect(find.text(Strings.playerFailed), findsNothing);
    expect(find.byType(UpNextCard), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'the middle of the player plays, and offers a replay at the end',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();
      await _settled(tester);
      fake.pause();
      await _shown(tester);

      expect(find.byIcon(Icons.replay), findsNothing);
      fake.calls.clear();
      await tester.tap(find.byIcon(Icons.play_arrow).last);
      await tester.pump();
      expect(fake.calls, <String>['play']);

      fake.emit(playing: true);
      await _shown(tester);
      expect(
        find.byIcon(Icons.play_arrow),
        findsNothing,
        reason: 'nothing should sit over a video that is playing',
      );

      fake.emit(ended: true);
      await _shown(tester);
      expect(find.byIcon(Icons.replay), findsOneWidget);

      fake.calls.clear();
      await tester.tap(find.byIcon(Icons.replay));
      await tester.pump();
      expect(fake.seekedMs, 0, reason: 'replay starts from the beginning');
      expect(fake.calls, <String>['seek', 'play']);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('double clicking the video goes full screen and back', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.calls.clear();

    final Offset spot =
        tester.getCenter(find.byType(PlayerFrame)) + const Offset(0, 80);
    await tester.tapAt(spot);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(spot);
    await tester.pumpAndSettle();

    expect(fake.calls, contains('fullscreen'));
    expect(
      fake.calls,
      _noPlayOrPause,
      reason: 'a double click is one gesture, not a play and a full screen',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('on touch a tap never pauses, and a double tap seeks', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();
    fake.calls.clear();

    final Offset spot =
        tester.getCenter(find.byType(PlayerFrame)) + const Offset(0, 80);
    await tester.tapAt(spot);
    await tester.pumpAndSettle();

    expect(
      fake.calls,
      _noPlayOrPause,
      reason: 'a thumb landing on the picture asks to see the controls',
    );

    await tester.tapAt(spot);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(spot);
    await tester.pumpAndSettle();

    expect(fake.calls, isNot(contains('fullscreen')));
    expect(fake.calls.where((String c) => c.startsWith('seek')), isNotEmpty);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a touch player drops the controls a phone cannot use', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, touch: true));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.fullscreen), findsNothing);
    expect(find.byIcon(Icons.width_wide), findsNothing);

    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();
    await tester.tap(find.text(PlayerPanel.settings.title));
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.shortcutsHeading),
      findsNothing,
      reason: 'there is no keyboard to list shortcuts for',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets(
    'a touch player offers a way out that iOS does not otherwise have',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake, touch: true));
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.arrow_back), findsOneWidget);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'a pointer player leaves the way out to the browser and the shell',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.arrow_back), findsNothing);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('the network wait is selectable, applied and remembered', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{_timeoutKey: 90});
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(
      fake.networkTimeout,
      90,
      reason: 'the stored preference has to reach the player on start',
    );
    expect(
      fake.calls.indexOf('timeout:90'),
      allOf(isNonNegative, lessThan(fake.calls.indexOf('attach:direct'))),
      reason:
          'mpv reads the timeout when it opens the stream, so setting it after '
          'attach leaves the first stream on the wrong value',
    );

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.playerNetworkTimeout),
      findsOneWidget,
      reason:
          'the control is offered when the engine can honour it, which is what '
          'makes its absence on a native-HLS path mean something',
    );

    await tester.tap(find.text(Strings.playerAutoplayDelay(90)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerAutoplayDelay(60)).last);
    await tester.pumpAndSettle();

    expect(fake.networkTimeout, 60, reason: 'and again when it is changed');
    expect(
      (await SharedPreferences.getInstance()).getInt(_timeoutKey),
      60,
      reason: 'the choice has to outlive the session that made it',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('the buffer target reaches the player before the stream opens', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{_bufferKey: 120});
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(fake.bufferSeconds, 120);
    expect(
      fake.calls.indexOf('buffer:120'),
      allOf(isNonNegative, lessThan(fake.calls.indexOf('attach:direct'))),
      reason:
          'hls.js takes its buffer config when it is constructed, so a target '
          'set after attach would not apply until the next stream',
    );

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerBufferDuration(120)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerBufferDuration(300)).last);
    await tester.pumpAndSettle();

    expect(fake.bufferSeconds, 300);
    expect(
      (await SharedPreferences.getInstance()).getInt(_bufferKey),
      300,
      reason: 'a weak connection outlives the session that noticed it',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('waiting for the buffer holds playback until the target is met', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      _waitKey: true,
      _bufferKey: 30,
    });
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await _settleAroundSpinner(tester);

    expect(fake.attachedAutoplay, isFalse);
    expect(find.text(Strings.playerBuffering), findsOneWidget);
    expect(find.text(Strings.playerBufferingTo(0, 30)), findsOneWidget);

    fake.at(positionMs: 0, bufferedAheadMs: 12000);
    await tester.pump(const Duration(seconds: 1));

    expect(
      fake.calls.contains('play'),
      isFalse,
      reason: 'twelve seconds is short of the thirty that was asked for',
    );

    fake.at(positionMs: 0, bufferedAheadMs: 30000);
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(fake.calls.contains('play'), isTrue);
    expect(find.text(Strings.playerBuffering), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a player that cannot buffer ahead never holds playback', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      _waitKey: true,
      _bufferKey: 600,
    });
    final FakePlayerController fake = FakePlayerController()
      ..hlsBuffers = false;
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(fake.calls.contains('play'), isTrue);
    expect(find.text(Strings.playerBuffering), findsNothing);
    expect(find.text(Strings.playerPlayNow), findsNothing);

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.playerBufferingSection),
      findsNothing,
      reason:
          'a control that cannot reach the engine reads as broken, so the '
          'whole section goes rather than sitting there doing nothing',
    );
    expect(find.text(Strings.playerBufferTarget), findsNothing);
    expect(find.text(Strings.playerWaitForBuffer), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a network wait that cannot reach the engine is not offered', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..hlsBuffers = false;
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.playerNetworkTimeout),
      findsNothing,
      reason:
          'the browser owns the timeout on a native-HLS path, so offering a '
          'choice that changes nothing is worse than offering none',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a clip shorter than the target waits only for what exists', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      _waitKey: true,
      _bufferKey: 300,
    });
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await _settleAroundSpinner(tester);

    expect(
      find.text(Strings.playerBufferingTo(0, 100)),
      findsOneWidget,
      reason:
          'the fake runs a 100 second video, so the reachable target is the '
          'whole clip and not the five minutes that were asked for',
    );

    fake.at(positionMs: 0, bufferedAheadMs: 99600);
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    expect(
      fake.calls.contains('play'),
      isTrue,
      reason:
          'buffered to within a second of the whole clip is buffered, and the '
          'overlay already says so',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a buffer that stops growing gives up and starts', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      _waitKey: true,
      _bufferKey: 600,
    });
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await _settleAroundSpinner(tester);

    fake.at(positionMs: 0, bufferedAheadMs: 11000);
    for (int i = 0; i <= kPrebufferGiveUpSeconds; i++) {
      await tester.pump(const Duration(seconds: 1));
    }

    expect(
      fake.calls.contains('play'),
      isTrue,
      reason:
          'eleven seconds that never move is exactly what a paused progressive '
          'video looks like, and waiting longer would not help',
    );
    expect(
      kPrebufferGiveUpSeconds,
      10,
      reason:
          'this is the dead wait a viewer sits through every time the memory '
          'limit binds, so it is bounded by patience rather than by how long '
          'a slow transcode takes to deliver its next segment',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a viewer who is tired of waiting can start anyway', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      _waitKey: true,
      _bufferKey: 600,
    });
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await _settleAroundSpinner(tester);

    expect(find.text(Strings.playerPlayNow), findsOneWidget);

    await tester.tap(find.text(Strings.playerPlayNow));
    await _settleAroundSpinner(tester);

    expect(fake.calls.contains('play'), isTrue);
    expect(find.text(Strings.playerPlayNow), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a finished video is not treated as waiting for frames', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();

    fake.emit(ended: true);
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a carried language is asked for when the session starts', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        controls: const PlaybackControls(
          audioLanguage: 'fr',
          subtitleLanguage: 'fr',
        ),
      ),
    );
    await tester.pumpAndSettle();

    final http.Request start = seen.firstWhere(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/sessions',
    );
    final Map<String, dynamic> body =
        jsonDecode(start.body) as Map<String, dynamic>;
    expect(body['audio_language'], 'fr');
    expect(body['subtitle_language'], 'fr');
    expect(
      body.containsKey('audio_track'),
      isFalse,
      reason: 'an index from the previous episode means nothing in this file',
    );
    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/update')),
      isEmpty,
      reason:
          'the start request already carries the request, so nothing has '
          'to be renegotiated after it',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a chosen subtitle follows the viewer into the next episode', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.subtitles).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles).last);
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.skip_next));
    await tester.pumpAndSettle();

    final Uri next = Uri.parse(pushed.single);
    expect(next.path, '/watch');
    expect(next.queryParameters['version'], 'v2');
    expect(next.queryParameters['slang'], 'en');
    expect(
      next.queryParameters.containsKey('sub'),
      isFalse,
      reason: 'stream 2 of this episode is not stream 2 of the next one',
    );
  });

  testWidgets('a viewer who chose nothing carries nothing forward', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<String> pushed = <String>[];
    await tester.pumpWidget(_app(fake, episodic: true, pushed: pushed));
    await tester.pumpAndSettle();

    await tester.tap(find.byIcon(Icons.skip_next));
    await tester.pumpAndSettle();

    expect(
      Uri.parse(pushed.single).queryParameters,
      <String, String>{'version': 'v2'},
      reason:
          'the account preference has to keep answering for someone who '
          'never overrode it',
    );
  });

  testWidgets('an episode is titled series, code and title, dot-separated', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();

    expect(find.text('The Show · S01E01 · Pilot'), findsWidgets);
    expect(find.textContaining('S01E01:'), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('an episode gets both steps, with no way back from the first', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.skip_previous), findsOneWidget);
    expect(find.byIcon(Icons.skip_next), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(
            find.ancestor(
              of: find.byIcon(Icons.skip_previous),
              matching: find.byType(IconButton),
            ),
          )
          .onPressed,
      isNull,
      reason: 'the first episode of the first season has no predecessor',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a session resumed part way reports the position on the '
      'whole timeline, not within its own manifest', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, sequential: true, originMs: 40000),
    );
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.at(positionMs: 5000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();

    final http.Request progress = seen.lastWhere(
      (http.Request r) => r.url.path.endsWith('/progress'),
    );
    expect(
      jsonDecode(progress.body)['position_ms'],
      45000,
      reason:
          'the manifest starts at 40s, so the player reporting 5s means the '
          'viewer is 45s into the film; reporting 5s would rewind the bookmark',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('seeking inside what is already produced stays local', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, sequential: true, originMs: 10000),
    );
    await tester.pumpAndSettle();
    await _settled(tester);

    fake.at(positionMs: 5000, bufferedAheadMs: 20000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isEmpty,
      reason: 'a short hop is already on disk, so restarting would be waste',
    );
    expect(
      fake.seekedMs,
      5000 + kSeekStepMs,
      reason: 'the player is asked in its own coordinates, not the timeline',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('seeking past what is produced restarts the session', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        sequential: true,
        originMs: 0,
        seekOriginMs: 88000,
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pumpAndSettle();

    final http.Request seek = seen.firstWhere(
      (http.Request r) => r.url.path.endsWith('/seek'),
    );
    expect(jsonDecode(seek.body)['position_ms'], 90000);
    expect(
      fake.attached,
      '/stream/tok3/master.m3u8',
      reason:
          'a restarted producer writes a new init segment, so the player '
          'has to re-attach for it to be fetched',
    );
    expect(
      fake.attachedPositionMs,
      2000,
      reason:
          'the new manifest begins at 88s, so 90s on the timeline is 2s into it',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a player that cannot seek in stream restarts for every seek', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..streamSeeks = false;
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, sequential: true, originMs: 0, seekOriginMs: 8000),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000, bufferedAheadMs: 60000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isNotEmpty,
      reason:
          'libavformat cannot seek inside an fmp4 HLS playlist, so mpv must be '
          'handed a fresh manifest that already starts where the viewer aimed',
    );
    expect(
      fake.attachedPositionMs,
      0,
      reason:
          'the new manifest already begins at the target, so asking the player '
          'to seek again would hit the same broken path',
    );
  });

  testWidgets('a player that can seek in stream still seeks locally', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..streamSeeks = true;
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, sequential: true, originMs: 0),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000, bufferedAheadMs: 60000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isEmpty,
    );
    expect(fake.seekedMs, 1000 + kSeekStepMs);
  });

  testWidgets(
    'a native player seeks inside mpegts segments without a restart',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController()
        ..unseekable.add('fmp4');
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(
        _app(fake, seen: seen, sequential: true, container: 'mpegts'),
      );
      await tester.pumpAndSettle();

      fake.at(positionMs: 1000, bufferedAheadMs: 60000);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();

      expect(
        seen.where((http.Request r) => r.url.path.endsWith('/seek')),
        isEmpty,
      );
      expect(fake.seekedMs, 1000 + kSeekStepMs);

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    'a native player restarted far into mpegts segments lands on the target',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController()
        ..unseekable.add('fmp4');
      final List<http.Request> seen = <http.Request>[];
      await tester.pumpWidget(
        _app(
          fake,
          seen: seen,
          sequential: true,
          seekOriginMs: 88000,
          container: 'mpegts',
        ),
      );
      await tester.pumpAndSettle();

      fake.at(positionMs: 1000);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
      await tester.pumpAndSettle();

      expect(
        seen.where((http.Request r) => r.url.path.endsWith('/seek')),
        isNotEmpty,
      );
      expect(
        fake.attachedPositionMs,
        2000,
        reason: 'a restart begins on a segment boundary, 2s before 90s',
      );

      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
    },
  );

  testWidgets('a native player restarts fmp4 segments for every seek', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..unseekable.add('fmp4');
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        sequential: true,
        seekOriginMs: 8000,
        container: 'fmp4',
      ),
    );
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000, bufferedAheadMs: 60000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    await tester.pumpAndSettle();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isNotEmpty,
    );
    expect(fake.attachedPositionMs, 0);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a session that is not produced sequentially never restarts', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    fake.at(positionMs: 1000);
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pumpAndSettle();

    expect(
      seen.where((http.Request r) => r.url.path.endsWith('/seek')),
      isEmpty,
      reason:
          'direct play and just in time segments are reachable anywhere, so '
          'the whole timeline is seekable without the server doing anything',
    );
    expect(fake.seekedMs, 90000);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  Future<void> pickQuality(WidgetTester tester, String rung) async {
    await tester.tap(find.byIcon(Icons.video_settings));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(rung).last);
  }

  testWidgets('during a change the player settings stay usable while play, '
      'pause and the timeline wait', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(_app(fake, onUpdate: (int _) => slow.future));
    await tester.pumpAndSettle();
    fake.pause();
    await _shown(tester);

    await pickQuality(tester, '720p');
    await _settleAroundSpinner(tester);
    await tester.tap(find.text(Strings.playerNormalSpeed));
    await _settleAroundSpinner(tester);
    await tester.tap(find.text('1.5x').last);
    await _settleAroundSpinner(tester);

    expect(fake.rate, 1.5);
    expect(
      tester
          .widget<IconButton>(
            find.ancestor(
              of: find.descendant(
                of: find.byType(OverlayBar),
                matching: find.byIcon(Icons.play_arrow),
              ),
              matching: find.byType(IconButton),
            ),
          )
          .onPressed,
      isNull,
    );
    expect(tester.widget<Timeline>(find.byType(Timeline)).onSeek, isNull);

    slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await _settleAroundSpinner(tester);

    expect(tester.widget<Timeline>(find.byType(Timeline)).onSeek, isNotNull);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a failed change puts the setting back and says so each time', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(
      _app(
        fake,
        onUpdate: (int _) => Future<http.Response>.error(Exception('down')),
        toasts: true,
      ),
    );
    await tester.pumpAndSettle();
    fake.pause();
    await _shown(tester);

    await pickQuality(tester, '720p');
    await tester.pumpAndSettle();

    expect(find.text(Strings.renegotiationFailed), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(PlayerSettingsPanel),
        matching: find.text(Strings.playerOriginalAt(1080)),
      ),
      findsOneWidget,
    );

    await tester.tap(find.text(Strings.playerOriginalAt(1080)).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('480p').last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.renegotiationFailed), findsNWidgets(2));

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a failed far seek goes back to where it was and says so', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(
      _app(
        fake,
        sequential: true,
        toasts: true,
        intercept: (http.Request r) =>
            r.method == 'POST' && r.url.path.endsWith('/seek')
            ? http.Response('', 500)
            : null,
      ),
    );
    await tester.pumpAndSettle();
    fake.at(positionMs: 1000);
    await _shown(tester);
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await tester.pumpAndSettle();

    expect(find.text(Strings.seekFailed), findsOneWidget);
    expect(find.text('0:01 / 1:40'), findsOneWidget);
    expect(fake.calls, <String>['pause', 'play']);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a stream that fails to load while watching offers Retry from '
      'where it was', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();
    await _settled(tester);
    fake
      ..pausedAt(positionMs: 30000)
      ..failedAttaches = 1;
    await _shown(tester);

    await pickQuality(tester, '720p');
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerFailed), findsOneWidget);

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    final List<http.Request> starts = seen
        .where(
          (http.Request r) =>
              r.method == 'POST' && r.url.path == '/api/v1/sessions',
        )
        .toList();
    expect(starts, hasLength(2));
    expect(
      (jsonDecode(starts.last.body)
          as Map<String, dynamic>)['start_position_ms'],
      30000,
    );
    expect(find.byType(PlayerFrame), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a change with no answer ends the session at the request bound '
      'and offers Retry', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    final Completer<http.Response> never = Completer<http.Response>();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(
      _app(fake, seen: seen, onUpdate: (int _) => never.future, toasts: true),
    );
    await tester.pumpAndSettle();
    fake.pause();
    await _shown(tester);

    await pickQuality(tester, '720p');
    await tester.pump(const Duration(seconds: kRequestTimeoutS - 1));
    expect(find.text(Strings.applyingPlaybackChanges), findsOneWidget);

    await tester.pump(const Duration(seconds: 1));
    await _shown(tester);

    expect(find.text(Strings.applyingPlaybackChanges), findsNothing);
    expect(find.text(Strings.renegotiationFailed), findsNothing);
    expect(find.text(Strings.playerFailed), findsOneWidget);
    expect(find.text(Strings.retry), findsOneWidget);
    expect(
      seen.where(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
      ),
      hasLength(1),
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  Future<void> watchWithFailingChanges(
    WidgetTester tester,
    FakePlayerController fake,
  ) async {
    await tester.pumpWidget(
      _app(
        fake,
        touch: true,
        episodic: true,
        pushed: <String>[],
        toasts: true,
        onUpdate: (int _) async => http.Response('', 500),
      ),
    );
    await tester.pumpAndSettle();
    await _settled(tester);
  }

  Future<void> pickEnglishSubtitles(WidgetTester tester) async {
    await tester.tap(find.byIcon(Icons.tune));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerNoSubtitles));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('English').last);
    await tester.pump();
    await _shown(tester);
  }

  testWidgets('a change that fails at the end brings the countdown back, '
      'from the start', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    await watchWithFailingChanges(tester, fake);
    fake.emit(ended: true);
    await _shown(tester);
    int left() =>
        tester.widget<UpNextCard>(find.byType(UpNextCard)).remainingSeconds;
    expect(left(), const LocalSettings().autoplayNextS);
    await tester.pump(const Duration(seconds: 3));
    expect(left(), lessThan(const LocalSettings().autoplayNextS));

    await pickEnglishSubtitles(tester);

    expect(find.text(Strings.renegotiationFailed), findsOneWidget);
    expect(left(), const LocalSettings().autoplayNextS);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a countdown the viewer cancelled stays off after a failed '
      'change at the end', (WidgetTester tester) async {
    final FakePlayerController fake = FakePlayerController();
    await watchWithFailingChanges(tester, fake);
    fake.emit(ended: true);
    await _shown(tester);
    await tester.tap(find.text(Strings.cancel));
    await _shown(tester);
    expect(find.byType(UpNextCard), findsNothing);

    await pickEnglishSubtitles(tester);

    expect(find.text(Strings.renegotiationFailed), findsOneWidget);
    expect(find.byType(UpNextCard), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('a stream that never loads fails after the network wait', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController()
      ..attachGate = Completer<void>();
    await tester.pumpWidget(_app(fake));
    await _settleAroundSpinner(tester);
    expect(find.text(Strings.playerLoading), findsOneWidget);

    await tester.pump(
      Duration(
        seconds: const LocalSettings().networkTimeoutS + kLoadTimeoutMarginS,
      ),
    );
    await _shown(tester);

    expect(find.text(Strings.playerFailed), findsOneWidget);
    expect(find.text(Strings.retry), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('the back arrow leaves during a change and ends the session', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    final List<String> pushed = <String>[];
    await tester.pumpWidget(
      _app(
        fake,
        seen: seen,
        touch: true,
        episodic: true,
        pushed: pushed,
        sequential: true,
        intercept: (http.Request r) =>
            r.method == 'POST' && r.url.path.endsWith('/seek')
            ? _seekRestart(88000, null)
            : null,
        lag: (http.Request r) => const Duration(seconds: 5),
      ),
    );
    await tester.pumpAndSettle();
    fake.at(positionMs: 1000);
    await _shown(tester);
    await tester.sendKeyEvent(LogicalKeyboardKey.digit9);
    await _settleAroundSpinner(tester);

    await tester.tap(find.byIcon(Icons.arrow_back));
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));

    expect(pushed.single, episodeRoute('e1', series: 's1'));
    expect(
      seen.any(
        (http.Request r) =>
            r.method == 'DELETE' && r.url.path == '/api/v1/sessions/s1',
      ),
      isTrue,
    );
    fake.calls.clear();
    await tester.pump(const Duration(seconds: 5));

    expect(fake.calls.where((String c) => c.startsWith('attach')), isEmpty);
  });

  testWidgets('an old stream that ends during a change starts no countdown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final Completer<http.Response> slow = Completer<http.Response>();
    await tester.pumpWidget(
      _app(fake, episodic: true, onUpdate: (int _) => slow.future),
    );
    await tester.pumpAndSettle();
    await _settled(tester);
    fake.pause();
    await _shown(tester);

    await tester.tap(find.byIcon(Icons.audiotrack).first);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerStereoDownmix));
    await _settleAroundSpinner(tester);
    fake.emit(ended: true);
    await _shown(tester);

    expect(find.byType(UpNextCard), findsNothing);

    slow.complete(_renegotiated('remux', '/stream/second/master.m3u8'));
    await _settleAroundSpinner(tester);

    expect(find.byType(UpNextCard), findsNothing);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });
}
