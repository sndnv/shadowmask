import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/util/scoped_value.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/settings_panel.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/view/player_shortcuts.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/pages/player/watch_body.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

class FakePlayerController implements PlayerController {
  final ValueNotifier<PlayerSnapshot> _snapshot = ValueNotifier<PlayerSnapshot>(
    const PlayerSnapshot(durationMs: 100000, ready: true),
  );
  final ValueNotifier<bool> _fullscreen = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _mutedByPolicy = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _muted = ValueNotifier<bool>(false);
  final ValueNotifier<double> _volume = ValueNotifier<double>(1);
  final ValueNotifier<bool> _pip = ValueNotifier<bool>(false);
  final List<String> calls = <String>[];
  int? seekedMs;
  double? rate;
  int? networkTimeout;
  int? bufferSeconds;
  int? bufferBytes;
  String? attached;
  int? attachedPositionMs;
  bool? attachedAutoplay;
  bool streamSeeks = true;
  bool hlsBuffers = true;

  @override
  ValueListenable<PlayerSnapshot> get snapshot => _snapshot;

  @override
  ValueListenable<bool> get fullscreen => _fullscreen;

  @override
  ValueListenable<bool> get mutedByPolicy => _mutedByPolicy;

  @override
  ValueListenable<bool> get muted => _muted;

  @override
  ValueListenable<double> get volume => _volume;

  @override
  ValueListenable<bool> get pictureInPicture => _pip;

  @override
  bool get supportsPictureInPicture => true;

  @override
  bool get seeksWithinStream => streamSeeks;

  @override
  void unmute() {
    if (!_mutedByPolicy.value) {
      return;
    }
    setMuted(false);
    calls.add('unmute');
  }

  @override
  void setMuted(bool muted) {
    _muted.value = muted;
    if (!muted) {
      _mutedByPolicy.value = false;
    }
    calls.add('muted:$muted');
  }

  @override
  void setVolume(double volume) {
    _volume.value = volume;
    calls.add('volume:$volume');
    if (volume > 0 && _muted.value) {
      setMuted(false);
    }
  }

  @override
  Future<void> togglePictureInPicture() async {
    _pip.value = !_pip.value;
    calls.add('pip');
  }

  void mutePolicy() {
    _muted.value = true;
    _mutedByPolicy.value = true;
  }

  void emit({bool ended = false, bool playing = false}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: ended ? 100000 : 0,
      ready: true,
      ended: ended,
      playing: playing,
    );
  }

  void at({required int positionMs, int bufferedAheadMs = 0}) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      bufferedAheadMs: bufferedAheadMs,
      ready: true,
      playing: true,
    );
  }

  void stall({
    required int positionMs,
    bool ready = true,
    int bufferedAheadMs = 0,
  }) {
    _snapshot.value = PlayerSnapshot(
      durationMs: 100000,
      positionMs: positionMs,
      bufferedAheadMs: bufferedAheadMs,
      ready: ready,
      buffering: true,
      playing: true,
    );
  }

  @override
  Widget get view => const SizedBox.shrink();

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    int positionMs = 0,
    bool autoplay = true,
  }) async {
    calls.add('attach:${mode.name}');
    attached = manifestUrl;
    attachedPositionMs = positionMs;
    attachedAutoplay = autoplay;
  }

  @override
  Future<void> detach() async => calls.add('detach');

  @override
  void play() => calls.add('play');

  @override
  void pause() => calls.add('pause');

  @override
  void togglePlay() => calls.add('toggle');

  @override
  void seekTo(int positionMs) {
    seekedMs = positionMs;
    calls.add('seek');
  }

  @override
  void setRate(double value) => rate = value;

  @override
  void setNetworkTimeout(int seconds) {
    networkTimeout = seconds;
    calls.add('timeout:$seconds');
  }

  @override
  bool get buffersAhead => hlsBuffers;

  @override
  void setBuffer({required int seconds, required int bytes}) {
    bufferSeconds = seconds;
    bufferBytes = bytes;
    calls.add('buffer:$seconds');
  }

  @override
  void toggleFullscreen() {
    _fullscreen.value = !_fullscreen.value;
    calls.add('fullscreen');
  }

  @override
  PlayerDiagnostics diagnostics() => const PlayerDiagnostics(<String>[]);

  @override
  Future<void> dispose() async {}
}

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
            'language': 'eng',
          },
        ],
        'subtitles': <dynamic>[
          <String, dynamic>{'index': 2, 'language': 'eng', 'format': 'srt'},
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

http.Response _sequentialSession(int originMs) => http.Response(
  jsonEncode(<String, dynamic>{
    'session_id': 's1',
    'mode': 'remux',
    'manifest_url': '/stream/tok/master.m3u8',
    'origin_ms': originMs,
    'sequential': true,
    'heartbeat_interval_s': 10,
    'selected': <String, dynamic>{'audio_track': 1, 'subtitle_track': null},
    'trickplay': <dynamic>[],
  }),
  201,
);

http.Response _seekRestart(int originMs) => http.Response(
  jsonEncode(<String, dynamic>{
    'manifest_url': '/stream/tok3/master.m3u8',
    'mode': 'remux',
    'origin_ms': originMs,
    'sequential': true,
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
        'language': 'eng',
      },
    ],
    'subtitles': <dynamic>[
      <String, dynamic>{'index': 2, 'language': 'eng', 'format': 'srt'},
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

// The buffering card carries a CircularProgressIndicator, so pumpAndSettle
// never returns while it is on screen.
Future<void> _settleAroundSpinner(WidgetTester tester) async {
  for (int i = 0; i < 12; i++) {
    await tester.pump(const Duration(milliseconds: 20));
  }
}

const SelfUser _user = SelfUser(id: 'u1', username: 'u', role: UserRole.user);

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
  bool touch = false,
  Future<http.Response> Function(int call)? onUpdate,
}) {
  int updates = 0;
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) {
      seen?.add(r);
      if (onUpdate != null && r.url.path.endsWith('/update')) {
        return onUpdate(updates++);
      }
      if (sequential) {
        if (r.method == 'POST' && r.url.path == '/api/v1/sessions') {
          return Future<http.Response>.value(_sequentialSession(originMs));
        }
        if (r.method == 'POST' && r.url.path.endsWith('/seek')) {
          return Future<http.Response>.value(_seekRestart(seekOriginMs));
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
        user: _user,
        versionId: 'v1',
        initialControls: controls,
        controllerFactory: (_) => fake,
        wide: wide,
        touch: touch,
        onToggleWide: onToggleWide ?? () {},
      ),
    ),
  );
}

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

    expect(fake.calls, contains('attach:direct'));
    // One in the transport bar and one in the middle of a paused video.
    expect(find.byIcon(Icons.play_arrow), findsNWidgets(2));
    expect(find.text('0:00 / 1:40'), findsOneWidget);

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

    await tester.tap(find.byIcon(Icons.play_arrow).first);
    expect(fake.calls, contains('toggle'));

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

    // Off centre on purpose: the middle of a paused video is the play button.
    await tester.tapAt(
      tester.getCenter(find.byType(PlayerFrame)) + const Offset(0, 80),
    );
    // A single click only resolves once the double click window closes, which
    // is the price of double click toggling full screen.
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('unmute'));
    expect(fake.calls, isNot(contains('toggle')));

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

  testWidgets('flushes a final progress on teardown', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    final List<http.Request> seen = <http.Request>[];
    await tester.pumpWidget(_app(fake, seen: seen));
    await tester.pumpAndSettle();

    bool progressPosted() => seen.any(
      (http.Request r) =>
          r.method == 'POST' && r.url.path == '/api/v1/sessions/s1/progress',
    );

    expect(progressPosted(), isFalse);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();

    expect(progressPosted(), isTrue);
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

  // A fresh session, but waiting for a tap: coming back to the app is not the
  // same as asking to watch.
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
    expect(fake.attachedAutoplay, isTrue);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();

    expect(sessionStarts(), 2);
    expect(fake.attachedAutoplay, isFalse);

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
    navigator.pop();
    await tester.pumpAndSettle();

    expect(fake.attachedAutoplay, isTrue);

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

  testWidgets('space toggles playback from the keyboard', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();

    expect(fake.calls, contains('toggle'));
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
      contains('toggle'),
      reason: 'tabbing into the panel must not cost the player its keyboard',
    );

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
      isNot(contains('toggle')),
      reason: 'out of the focus chain, the handler hears nothing',
    );

    await tester.tap(find.byType(PlayerFrame));
    await tester.pumpAndSettle();
    fake.calls.clear();

    await tester.sendKeyEvent(LogicalKeyboardKey.space);
    await tester.pumpAndSettle();
    expect(
      fake.calls,
      contains('toggle'),
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

    expect(find.byType(UpNextCard), findsNothing);

    fake.emit(ended: true);
    await tester.pump();

    expect(find.byType(UpNextCard), findsOneWidget);
    expect(find.text('S01E02 · Second'), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('cancelling stays cancelled while the video rests on its end', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake, episodic: true));
    await tester.pumpAndSettle();

    fake.emit(ended: true);
    await tester.pump();
    await tester.tap(find.text(Strings.cancel));
    await tester.pump();

    expect(find.byType(UpNextCard), findsNothing);

    fake.emit(ended: true);
    await tester.pump();

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

    fake.emit(ended: true);
    await tester.pump();
    expect(find.byType(UpNextCard), findsOneWidget);

    fake.emit(playing: true);
    await tester.pump();

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
      await tester.pump();

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

    // Seeking into a part of the stream the server has not produced yet is the
    // same wait as the opening one, but the viewer has already seen frames.
    fake.stall(positionMs: 40000);
    await tester.pump();

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
      await tester.pump();
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

      // Dismissing restarts the clock rather than silencing it, so someone who
      // chose to wait is asked again instead of being left with the spinner.
      await tester.pump(Duration(seconds: _stallDelay + 1));
      expect(find.text(Strings.playerTooSlow), findsOneWidget);

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

    // Any forward movement at all is proof the pipeline is alive.
    fake.stall(positionMs: 0, ready: false, bufferedAheadMs: 500);
    await tester.pump(const Duration(seconds: 2));

    expect(find.text(Strings.playerTooSlow), findsNothing);

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

  testWidgets(
    'the middle of the player plays, and offers a replay at the end',
    (WidgetTester tester) async {
      final FakePlayerController fake = FakePlayerController();
      await tester.pumpWidget(_app(fake));
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.replay), findsNothing);
      fake.calls.clear();
      await tester.tap(find.byIcon(Icons.play_arrow).last);
      expect(fake.calls, contains('toggle'));

      fake.emit(playing: true);
      await tester.pump();
      expect(
        find.byIcon(Icons.play_arrow),
        findsNothing,
        reason: 'nothing should sit over a video that is playing',
      );

      fake.emit(ended: true);
      await tester.pump();
      expect(find.byIcon(Icons.replay), findsOneWidget);

      fake.calls.clear();
      await tester.tap(find.byIcon(Icons.replay));
      await tester.pump();
      expect(fake.seekedMs, 0, reason: 'replay starts from the beginning');
      expect(fake.calls, contains('toggle'));

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

    // Off centre, since the middle of a paused video is the play button.
    final Offset spot =
        tester.getCenter(find.byType(PlayerFrame)) + const Offset(0, 80);
    await tester.tapAt(spot);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(spot);
    await tester.pumpAndSettle();

    expect(fake.calls, contains('fullscreen'));
    expect(
      fake.calls,
      isNot(contains('toggle')),
      reason: 'a double click is one gesture, not a play and a full screen',
    );

    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
  });

  testWidgets('on touch a tap only reveals, and a double tap seeks', (
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
      isNot(contains('toggle')),
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
      // Android has a system back gesture and iOS has none, so the immersive
      // player would otherwise be a room with no door there.
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
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(
      fake.networkTimeout,
      kDefaultPlayerPrefs.networkTimeoutSeconds,
      reason: 'the stored preference has to reach the player on start',
    );
    expect(
      fake.calls.indexOf(
        'timeout:${kDefaultPlayerPrefs.networkTimeoutSeconds}',
      ),
      lessThan(fake.calls.indexOf('attach:direct')),
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

    await tester.tap(
      find
          .text(
            Strings.playerAutoplayDelay(
              kDefaultPlayerPrefs.networkTimeoutSeconds,
            ),
          )
          .last,
    );
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
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(fake.bufferSeconds, kDefaultPlayerPrefs.bufferSeconds);
    expect(
      fake.calls.indexOf('buffer:${kDefaultPlayerPrefs.bufferSeconds}'),
      lessThan(fake.calls.indexOf('attach:direct')),
      reason:
          'hls.js takes its buffer config when it is constructed, so a target '
          'set after attach would not apply until the next stream',
    );

    await tester.tap(find.byIcon(Icons.settings).first);
    await tester.pumpAndSettle();
    await tester.tap(
      find
          .text(Strings.playerBufferDuration(kDefaultPlayerPrefs.bufferSeconds))
          .last,
    );
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

  // The whole point of the toggle is that playback does not begin, so the
  // attach has to be told not to autoplay rather than being paused after.
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

  // Direct play on web never builds an hls.js instance, so nothing honours the
  // target. Holding playback for a buffer that cannot grow is a deadlock: a
  // paused progressive video stops filling once the browser has its own fill.
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

  // The same rule as the buffering section above, for the same reason: without
  // an hls.js instance the wait belongs to the browser, and no setting here can
  // move it. Shipping the control anyway is what made it look broken on web.
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

  // A clip shorter than the target can never reach it, so an absolute target
  // would sit through the whole give-up timer on every short video.
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

    // A fully buffered clip stops a fraction short of its stated duration, and
    // the overlay rounds that to the target while an exact comparison does not,
    // so it read "45s of 45s" and still sat through the give-up timer.
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

  // Even where the target is honoured, a connection that cannot reach it would
  // otherwise wait for ever without the viewer touching anything.
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

  // A target set too high on a connection that cannot reach it would otherwise
  // trap the viewer in the waiting state with no way out.
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
          audioLanguage: 'fra',
          subtitleLanguage: 'fra',
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
    expect(body['audio_language'], 'fra');
    expect(body['subtitle_language'], 'fra');
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
    expect(next.queryParameters['slang'], 'eng');
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

    fake.at(positionMs: 5000);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.pause).first);
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
}
