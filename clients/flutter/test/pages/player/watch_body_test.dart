import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/player/player_frame.dart';
import 'package:shadowmask/components/player/shortcuts_dialog.dart';
import 'package:shadowmask/components/player/up_next_card.dart';
import 'package:shadowmask/view/player_shortcuts.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/pages/player/watch_body.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

class FakePlayerController implements PlayerController {
  final ValueNotifier<PlayerSnapshot> _snapshot = ValueNotifier<PlayerSnapshot>(
    const PlayerSnapshot(durationMs: 100000),
  );
  final ValueNotifier<bool> _fullscreen = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _mutedByPolicy = ValueNotifier<bool>(false);
  final ValueNotifier<bool> _muted = ValueNotifier<bool>(false);
  final ValueNotifier<double> _volume = ValueNotifier<double>(1);
  final ValueNotifier<bool> _pip = ValueNotifier<bool>(false);
  final List<String> calls = <String>[];
  int? seekedMs;
  double? rate;

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
      ended: ended,
      playing: playing,
    );
  }

  @override
  Widget get view => const SizedBox.shrink();

  @override
  Future<void> attach(
    String manifestUrl, {
    required PlaybackMode mode,
    int positionMs = 0,
  }) async {
    calls.add('attach:${mode.name}');
  }

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
    };

http.Response _episodeVersion() => http.Response(
  jsonEncode(<String, dynamic>{
    'id': 'v1',
    'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
    'library_id': 'lib',
    'quality': 'hd',
    'container': 'mp4',
    'duration_ms': 100000,
  }),
  200,
);

http.Response? _episodic(String path) {
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
}) {
  final ApiClient api = ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request r) {
      seen?.add(r);
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
    home: Scaffold(
      body: WatchBody(
        api: api,
        user: _user,
        versionId: 'v1',
        initialControls: const PlaybackControls(),
        controllerFactory: (_) => fake,
        wide: wide,
        onToggleWide: onToggleWide ?? () {},
      ),
    ),
  );
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('boots a session and renders the transport and clock', (
    WidgetTester tester,
  ) async {
    final FakePlayerController fake = FakePlayerController();
    await tester.pumpWidget(_app(fake));
    await tester.pumpAndSettle();

    expect(fake.calls, contains('attach:direct'));
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
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

    await tester.tap(find.byIcon(Icons.play_arrow));
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

    await tester.tap(find.byType(GestureDetector).first);
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
    await tester.tap(find.text(Strings.playerOriginal).last);
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
}
