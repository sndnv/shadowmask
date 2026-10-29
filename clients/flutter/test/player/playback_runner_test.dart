import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/player/playback_runner.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_notice.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/player/watches/buffer_watch.dart';
import 'package:shadowmask/player/watches/stall_watch.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/profile_version.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

import '../support/fake_player_controller.dart';
import '../support/playback_run.dart';

class FakePlaybackApi extends Fake implements PlaybackApi {
  final List<String> calls = <String>[];
  final List<Completer<PlaybackSession>> starts =
      <Completer<PlaybackSession>>[];
  final List<Completer<Negotiation>> answers = <Completer<Negotiation>>[];
  final List<Completer<void>> progresses = <Completer<void>>[];
  final List<Completer<void>> ends = <Completer<void>>[];
  bool holdProgress = false;
  bool holdEnds = false;
  PlaybackControls? controls;
  ClientDecoding? decoding;
  List<String>? subtitleDeliveries;
  bool? selectsAudioTracks;
  bool? mixesSound;

  @override
  Future<PlaybackSession> startSession({
    required String versionId,
    int startPositionMs = 0,
    int profileVersion = kExpectedProfileVersion,
    String platform = 'generic',
    ClientDecoding? decoding,
    List<String> subtitleDeliveries = const <String>[],
    bool selectsAudioTracks = false,
    bool mixesSound = false,
    PlaybackControls controls = const PlaybackControls(),
  }) {
    calls.add('start:$versionId@$startPositionMs:$profileVersion:$platform');
    this.controls = controls;
    this.decoding = decoding;
    this.subtitleDeliveries = subtitleDeliveries;
    this.selectsAudioTracks = selectsAudioTracks;
    this.mixesSound = mixesSound;
    final Completer<PlaybackSession> answer = Completer<PlaybackSession>();
    starts.add(answer);
    return answer.future;
  }

  @override
  Future<Negotiation> update(
    String sessionId,
    PlaybackControls controls,
    int positionMs,
  ) {
    calls.add('update:$sessionId@$positionMs');
    this.controls = controls;
    return _held();
  }

  @override
  Future<Negotiation> seek(String sessionId, int positionMs) {
    calls.add('seek:$sessionId@$positionMs');
    return _held();
  }

  Future<Negotiation> _held() {
    final Completer<Negotiation> answer = Completer<Negotiation>();
    answers.add(answer);
    return answer.future;
  }

  @override
  Future<void> progress(String sessionId, int positionMs, String state) {
    calls.add('progress:$sessionId@$positionMs:$state');
    return _sent(holdProgress, progresses);
  }

  @override
  Future<void> endSession(String sessionId) {
    calls.add('end:$sessionId');
    return _sent(holdEnds, ends);
  }

  Future<void> _sent(bool hold, List<Completer<void>> held) {
    if (!hold) {
      return Future<void>.value();
    }
    final Completer<void> answer = Completer<void>();
    held.add(answer);
    return answer.future;
  }

  List<String> named(String kind) =>
      calls.where((String c) => c.startsWith('$kind:')).toList();
}

class Rig {
  Rig({
    LocalSettings local = const LocalSettings(),
    PlaybackControls controls = const PlaybackControls(),
    int resumeMs = 0,
    bool playing = true,
    bool gathered = true,
    FakePlayerController? player,
  }) : player = player ?? FakePlayerController() {
    runner = PlaybackRunner(
      api: api,
      player: this.player,
      saveLocal: (LocalSettings l) async => saved.add(l),
      platform: 'linux',
      versionId: kVersion.id,
      controls: controls,
      playing: playing,
    );
    if (gathered) {
      gather(local: local, resumeMs: resumeMs);
    }
  }

  void gather({
    LocalSettings local = const LocalSettings(),
    int resumeMs = 0,
  }) => runner.opened(
    title: const PlaybackTitle(version: kVersion),
    local: local,
    resumeMs: resumeMs,
    profileVersion: 7,
  );

  final FakePlaybackApi api = FakePlaybackApi();
  final FakePlayerController player;
  final List<LocalSettings> saved = <LocalSettings>[];
  late final PlaybackRunner runner;

  PlaybackState get state => runner.state.value;

  Future<void> answerStart(
    WidgetTester tester, [
    PlaybackSession session = kDirect,
  ]) async {
    await tester.pump();
    api.starts.last.complete(session);
    await tester.pump();
  }

  Future<void> watch(
    WidgetTester tester, {
    PlaybackSession session = kDirect,
    int atMs = 0,
  }) async {
    await answerStart(tester, session);
    await tester.pump(const Duration(milliseconds: kSeekSettleWaitMs));
    player.at(positionMs: atMs - session.originMs);
    await tester.pump();
    player.calls.clear();
    api.calls.clear();
  }

  Future<void> leave(WidgetTester tester) async {
    unawaited(runner.leave());
    await tester.pump();
  }
}

const PlaybackControls k720 = PlaybackControls(height: 720);

void main() {
  testWidgets('opening applies the saved settings and asks for a session '
      'with them', (WidgetTester tester) async {
    const ClientDecoding decoding = ClientDecoding(maxHeight: 1080);
    final Rig rig = Rig(
      local: const LocalSettings(
        speed: 1.5,
        volume: 0.8,
        muted: true,
        subtitleSize: SubtitleSize.large,
        networkTimeoutS: 30,
        bufferS: 120,
        defaultDelivery: DeliveryPreference.never,
        decoding: decoding,
      ),
      controls: k720,
      resumeMs: 40000,
    );
    expect(rig.player.calls, isEmpty, reason: 'the gather is a posted write');
    expect(rig.api.calls, isEmpty);

    await tester.pump();
    expect(rig.player.rate, 1.5);
    expect(rig.player.subtitleSize, SubtitleSize.large);
    expect(rig.player.calls, <String>[
      'volume:0.8',
      'muted:true',
      'timeout:30',
      'buffer:120',
    ]);
    expect(rig.api.calls, <String>['start:v1@40000:7:linux']);
    expect(
      rig.api.controls,
      k720.copyWith(
        delivery: DeliveryPreference.never,
        downmix: false,
        normalize: true,
        dialogue: DialogueLift.none,
        night: NightMode.off,
      ),
    );
    expect(rig.api.decoding, decoding);
    await rig.leave(tester);
  });

  testWidgets('settings at their defaults are not sent to the player', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await tester.pump();

    expect(rig.player.calls, isEmpty);
    expect(rig.player.rate, isNull);
    expect(rig.player.subtitleSize, isNull);
    await rig.leave(tester);
  });

  testWidgets('a start answer loads the stream at its offset, paused, then '
      'plays', (WidgetTester tester) async {
    final Rig rig = Rig(resumeMs: 40000);

    await rig.answerStart(tester);

    expect(rig.player.attached, '/direct.m3u8');
    expect(rig.player.attachedPositionMs, 40000);
    expect(rig.player.attachedAutoplay, isFalse);
    expect(rig.player.attachedSubtitles, <bool>[false]);
    expect(rig.player.calls, <String>['attach:direct', 'play']);
    expect(rig.state.status.sent, isNull);
    await rig.leave(tester);
  });

  testWidgets('a subtitle the player draws is attached with the stream', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();

    await rig.answerStart(
      tester,
      kTranscode.copyWith(
        selected: const SelectedTracks(subtitleDelivery: 'hls_vtt'),
      ),
    );

    expect(rig.player.attachedSubtitles, <bool>[true]);
    await rig.leave(tester);
  });

  testWidgets('a start declares the tracks the player handles itself', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();

    await tester.pump();

    expect(rig.api.subtitleDeliveries, rig.player.subtitleDeliveries);
    expect(rig.api.selectsAudioTracks, isTrue);
    expect(rig.api.mixesSound, isTrue);
    expect(rig.runner.state.value.mixesSound, isTrue);
    await rig.leave(tester);
  });

  testWidgets('a loaded stream tunes the player to the session\'s sound', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();

    await rig.answerStart(tester, kDirect);

    expect(rig.player.tunings, const <(bool, DialogueLift)>[
      (true, DialogueLift.none),
    ]);
    await rig.leave(tester);
  });

  testWidgets('a subtitle file and an audio track the player selects are '
      'attached with the stream', (WidgetTester tester) async {
    final Rig rig = Rig();

    await rig.answerStart(
      tester,
      kDirect.copyWith(
        selected: const SelectedTracks(
          audioTrack: 2,
          audioDelivery: 'embedded',
          subtitleTrack: SubtitleSelection.file('f1'),
          subtitleDelivery: 'external',
          subtitleUrl: '/stream/t1/subtitles.vtt',
        ),
      ),
    );

    expect(rig.player.attachedTracks, const <PlayerTracks>[
      PlayerTracks(subtitleUrl: '/stream/t1/subtitles.vtt', audioIndex: 2),
    ]);
    await rig.leave(tester);
  });

  testWidgets('readings far from where a new stream starts are held until '
      'it settles', (WidgetTester tester) async {
    final Rig rig = Rig(resumeMs: 40000);

    await rig.answerStart(tester);
    expect(
      rig.state.timeline.positionMs,
      40000,
      reason: 'the reading from before the load is not a position',
    );
    rig.player.at(positionMs: 3000);
    await tester.pump(const Duration(milliseconds: kSeekSettleWaitMs - 1));
    expect(rig.state.timeline.positionMs, 40000);
    await tester.pump(const Duration(milliseconds: 1));
    rig.player.at(positionMs: 3250);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 3250);
    await rig.leave(tester);
  });

  testWidgets('a reading close to where a new stream starts settles it', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(resumeMs: 40000);

    await rig.answerStart(tester);
    rig.player.at(positionMs: 40500);
    await tester.pump();

    expect(rig.state.timeline.positionMs, 40500);
    expect(rig.state.timeline.seekingToMs, isNull);
    await rig.leave(tester);
  });

  testWidgets('a refused start shows its error, and a retry starts again', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(resumeMs: 5000);
    await tester.pump();

    rig.api.starts.single.completeError(Exception('no'));
    await tester.pump();
    expect(rig.state.status.error, Strings.couldNotStartPlayback);
    expect(rig.state.status.sent, isNull);

    rig.runner.retry();
    await tester.pump();
    expect(rig.api.named('start'), <String>[
      'start:v1@5000:7:linux',
      'start:v1@5000:7:linux',
    ]);
    await rig.leave(tester);
  });

  testWidgets('a start answer after leaving only ends its session', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await tester.pump();

    await rig.leave(tester);
    rig.api.starts.single.complete(kDirect);
    await tester.pump();

    expect(rig.api.calls, <String>['start:v1@0:7:linux', 'end:s1']);
    expect(rig.player.calls, isEmpty);
  });

  testWidgets('leaving sends the last progress and the end together, and lets '
      'go of the player at once', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.api
      ..holdProgress = true
      ..holdEnds = true;
    bool left = false;

    unawaited(rig.runner.leave().then((_) => left = true));
    await tester.pump();

    expect(rig.api.calls, <String>['progress:s1@5000:playing', 'end:s1']);
    expect(left, isTrue);
    expect(rig.state.status.life, PlaybackLife.closed);
    rig.api.progresses.single.complete();
    rig.api.ends.single.complete();
    await tester.pump();
  });

  testWidgets('going to the background ends the session, and coming back '
      'starts only once the end is answered', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.api
      ..holdProgress = true
      ..holdEnds = true;

    rig.runner.background();
    await tester.pump();
    expect(rig.player.calls, <String>['pause', 'detach']);
    expect(rig.api.calls, <String>['progress:s1@5000:playing', 'end:s1']);
    expect(rig.state.stream, isNull);
    await tester.pump(const Duration(seconds: 30));
    expect(rig.api.named('progress'), hasLength(1), reason: 'no heartbeat');

    rig.runner.foreground(playing: false);
    await tester.pump();
    expect(rig.api.named('start'), isEmpty);
    rig.api.ends.single.complete();
    await tester.pump();
    expect(rig.api.named('start'), isEmpty);
    rig.api.progresses.single.complete();
    await tester.pump();
    expect(rig.api.named('start'), <String>['start:v1@5000:7:linux']);
    await rig.leave(tester);
  });

  testWidgets('a stream setting pauses, asks, loads the answer and plays', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);

    rig.runner.changeRemote(k720);
    await tester.pump();
    expect(rig.player.calls, <String>['pause']);
    expect(rig.api.calls, <String>['update:s1@5000']);
    expect(rig.api.controls, k720);

    rig.api.answers.single.complete(kRenegotiated);
    await tester.pump();
    expect(rig.player.attached, '/next.m3u8');
    expect(rig.player.attachedPositionMs, 5000);
    expect(rig.player.calls, <String>['pause', 'attach:transcode', 'play']);
    expect(rig.state.status.sent, isNull);
    await rig.leave(tester);
  });

  testWidgets('the history holds what was chosen, sent and answered, in '
      'order', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.runner.changeRemote(k720);
    await tester.pump();
    rig.api.answers.single.complete(kRenegotiated);
    await tester.pump();
    rig.runner.changeRemote(const PlaybackControls(height: 480));
    await tester.pump();
    rig.api.answers.last.completeError(
      ApiException('boom', status: 502, code: 'upstream_error'),
    );
    await tester.pump();
    await rig.leave(tester);

    final List<Map<String, Object?>> entries = rig.runner.history.entries;
    expect(entries.map((Map<String, Object?> e) => e['event']), <String>[
      'opened',
      'sent_start',
      'answered',
      'load',
      'loaded',
      'chose',
      'sent_update',
      'answered',
      'load',
      'loaded',
      'chose',
      'sent_update',
      'failed',
      'ended',
    ]);
    expect(entries[1]['capabilities'], <String, Object?>{
      'platform': 'linux',
      'profile_version': 7,
      'decoding': null,
      'subtitle_deliveries': rig.player.subtitleDeliveries,
      'audio_tracks': rig.player.selectsAudioTracks,
      'audio_downmix': rig.player.mixesSound,
    });
    expect(entries[5]['controls'], k720.toUpdateBody());
    expect(entries[6]['position_ms'], 5000);
    expect(entries[7]['mode'], 'transcode');
    expect(entries[12], containsPair('status', 502));
    expect(entries[12], containsPair('code', 'upstream_error'));
    expect(entries[13]['session'], 's1');
    expect(
      entries.expand((Map<String, Object?> e) => e.keys),
      isNot(contains('version_id')),
      reason: 'the version is never named',
    );
  });

  testWidgets('the history names no title, track title, token or version', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(gathered: false);
    rig.runner.opened(
      title: PlaybackTitle(
        name: 'A Secret Film',
        version: kVersion.copyWith(
          audio: const <AudioTrack>[
            AudioTrack(index: 1, codec: 'aac', title: 'Commentary.Release'),
          ],
          subtitles: const <SubtitleTrack>[
            SubtitleTrack(
              index: 2,
              format: SubtitleFormat.srt,
              title: 'Forced.Release',
            ),
          ],
          subtitleFiles: const <SubtitleFile>[
            SubtitleFile(
              id: 'f1',
              format: SubtitleFormat.srt,
              source: SubtitleSource.external,
              label: 'Sidecar.Release.srt',
            ),
          ],
        ),
      ),
      local: const LocalSettings(),
      resumeMs: 0,
      profileVersion: 7,
    );
    await rig.answerStart(
      tester,
      kDirect.copyWith(
        manifestUrl: '/stream/secret-token/file',
        selected: const SelectedTracks(
          audioTrack: 1,
          subtitleTrack: SubtitleSelection.file('f1'),
          subtitleUrl: '/stream/secret-token/subtitles.vtt',
        ),
      ),
    );
    rig.runner.changeRemote(const PlaybackControls(audioTrack: 1));
    await tester.pump();
    rig.api.answers.single.complete(
      kRenegotiated.copyWith(manifestUrl: '/stream/secret-token/master.m3u8'),
    );
    await tester.pump();
    await rig.leave(tester);

    final String told = jsonEncode(rig.runner.history.entries);
    for (final String secret in <String>[
      'A Secret Film',
      'Release',
      'secret-token',
      kVersion.id,
    ]) {
      expect(told, isNot(contains(secret)), reason: secret);
    }
    expect(told, contains('"audio_track":1'), reason: 'the tracks are told');
  });

  testWidgets('a failed stream setting is put back and playback goes on', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    final PlaybackControls before = rig.state.remote;

    rig.runner.changeRemote(k720);
    await tester.pump();
    rig.api.answers.single.completeError(Exception('no'));
    await tester.pump();

    expect(rig.state.remote, before);
    expect(rig.state.status.notice, PlaybackNotice.changeFailed);
    expect(rig.player.calls, <String>['pause', 'play']);
    await rig.leave(tester);
  });

  testWidgets('a seek the stream cannot reach asks the server and loads its '
      'answer', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester, session: kTranscode);

    rig.runner.seek(600000);
    await tester.pump();
    expect(rig.api.calls, <String>['seek:s1@600000']);
    rig.api.answers.single.complete(
      kRenegotiated.copyWith(originMs: 600000, sequential: true),
    );
    await tester.pump();

    expect(rig.player.attached, '/next.m3u8');
    expect(rig.player.attachedPositionMs, 0);
    expect(rig.player.calls, <String>['pause', 'attach:transcode', 'play']);
    expect(rig.state.timeline.positionMs, 600000);
    await rig.leave(tester);
  });

  testWidgets('a local seek moves the player and holds its target for the '
      'settle wait', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.runner.seek(30000);
    await tester.pump();
    expect(rig.player.seekedMs, 30000);
    rig.player.at(positionMs: 5000);
    await tester.pump(const Duration(milliseconds: kSeekSettleWaitMs - 1));
    expect(rig.state.timeline.positionMs, 30000);

    await tester.pump(const Duration(milliseconds: 1));
    rig.player.at(positionMs: 5250);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 5250);
    await rig.leave(tester);
  });

  testWidgets('a second seek holds its own target for the whole wait', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.runner.seek(30000);
    await tester.pump(const Duration(milliseconds: 1500));
    rig.runner.seek(50000);
    await tester.pump(const Duration(milliseconds: 1500));
    rig.player.at(positionMs: 7000);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 50000);

    await tester.pump(const Duration(milliseconds: 500));
    rig.player.at(positionMs: 7250);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 7250);
    await rig.leave(tester);
  });

  testWidgets('a change with no answer is slow, then ends the session at the '
      'request bound', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    final PlaybackControls before = rig.state.remote;

    rig.runner.changeRemote(k720);
    await tester.pump(const Duration(seconds: kStallSeconds - 1));
    expect(rig.state.status.slow, isFalse);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.slow, isTrue);
    await tester.pump(
      const Duration(seconds: kRequestTimeoutS - kStallSeconds - 1),
    );
    expect(rig.state.status.sent, isA<SentControls>());
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.sent, isNull);
    expect(rig.state.status.slow, isFalse);
    expect(rig.state.remote, before);
    expect(rig.state.stream, isNull);
    expect(rig.state.status.error, Strings.playerFailed);
    expect(rig.api.named('end'), <String>['end:s1']);
    expect(rig.player.calls.last, 'detach');

    rig.api.answers.single.complete(kRenegotiated);
    await tester.pump();
    expect(rig.player.attached, '/direct.m3u8', reason: 'a late answer');
    await rig.leave(tester);
  });

  testWidgets('a start with no answer fails at the request bound', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();

    await tester.pump(const Duration(seconds: kRequestTimeoutS - 1));
    expect(rig.state.status.sent, isA<SentStart>());
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.error, Strings.couldNotStartPlayback);

    rig.api.starts.single.complete(kDirect);
    await tester.pump();
    expect(rig.api.calls, <String>['start:v1@0:7:linux', 'end:s1']);
    await rig.leave(tester);
  });

  testWidgets('a load is bounded by the network wait and the margin, '
      'counted from the answer', (WidgetTester tester) async {
    final Rig rig = Rig(local: const LocalSettings(networkTimeoutS: 30));
    await tester.pump(const Duration(seconds: 100));
    rig.player.attachGate = Completer<void>();

    rig.api.starts.single.complete(kDirect);
    await tester.pump(const Duration(seconds: 30 + kLoadTimeoutMarginS - 1));
    expect(rig.state.status.sent, isA<SentLoad>());
    await tester.pump(const Duration(seconds: 1));

    expect(rig.state.status.sent, isNull);
    expect(rig.state.status.error, Strings.playerFailed);
    expect(rig.api.named('end'), <String>['end:s1']);
    expect(rig.player.calls, <String>['timeout:30', 'attach:direct', 'detach']);
    await rig.leave(tester);
  });

  testWidgets('a stream that fails to load ends its session', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    rig.player.failedAttaches = 1;

    await rig.answerStart(tester);

    expect(rig.state.status.error, Strings.playerFailed);
    expect(rig.state.stream, isNull);
    expect(rig.api.named('end'), <String>['end:s1']);
    expect(rig.player.calls, <String>['attach:direct', 'detach']);
    await rig.leave(tester);
  });

  testWidgets('the heartbeat reports at the session interval and only with a '
      'session', (WidgetTester tester) async {
    final Rig rig = Rig();
    await tester.pump(const Duration(seconds: 30));
    expect(rig.api.named('progress'), isEmpty);

    await rig.watch(
      tester,
      session: kDirect.copyWith(heartbeatIntervalS: 15),
      atMs: 5000,
    );
    await tester.pump(
      const Duration(seconds: 15, milliseconds: -kSeekSettleWaitMs - 1),
    );
    expect(rig.api.named('progress'), isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(rig.api.named('progress'), <String>['progress:s1@5000:playing']);

    await rig.leave(tester);
    await tester.pump(const Duration(seconds: 60));
    expect(rig.api.named('progress'), hasLength(2));
  });

  testWidgets('the old stream is not read during a change, and is read again '
      'once it ends', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.runner.changeRemote(k720);
    await tester.pump();
    rig.player.at(positionMs: 7000);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 0);
    expect(rig.state.timeline.playing, isTrue);

    rig.player.pausedAt(positionMs: 7000);
    await tester.pump();
    rig.api.answers.single.completeError(Exception('no'));
    await tester.pump();
    expect(rig.state.timeline.positionMs, 7000);
    expect(rig.state.timeline.playing, isTrue);
    expect(rig.player.calls.last, 'play');
    await rig.leave(tester);
  });

  testWidgets('a change that ends by playing tells the server it plays, and '
      'waits for the player to say so', (WidgetTester tester) async {
    final Rig rig = Rig(resumeMs: 40000);
    rig.player.playsAtOnce = false;

    await rig.answerStart(tester);

    expect(rig.player.calls, <String>['attach:direct', 'play']);
    expect(rig.state.timeline.playing, isTrue);
    expect(
      rig.api.named('progress'),
      <String>['progress:s1@40000:playing'],
      reason:
          'the write that plays tells the server, not a reading taken '
          'right after play(), which still says paused',
    );
    await rig.leave(tester);
  });

  testWidgets('a change that ends paused takes the reading it ignored', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(resumeMs: 40000, playing: false);
    rig.player.attachGate = Completer<void>();
    await rig.answerStart(tester);

    rig.player.pausedAt(positionMs: 40500);
    await tester.pump();
    expect(rig.state.timeline.positionMs, 40000);

    rig.player.attachGate!.complete();
    await tester.pump();
    expect(rig.state.timeline.positionMs, 40500);
    await rig.leave(tester);
  });

  testWidgets('a stall shows after the stall time and clears when playback '
      'moves', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.player.stall(positionMs: 5000);
    await tester.pump(const Duration(seconds: kStallSeconds));
    expect(rig.state.status.stalled, isFalse);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.stalled, isTrue);

    rig.player.at(positionMs: 6000);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.stalled, isFalse);
    await rig.leave(tester);
  });

  testWidgets('keep waiting clears a stall and its error, and counts again', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    rig.player.stall(positionMs: 5000);
    await tester.pump();
    rig.player.fail('boom');
    await tester.pump(const Duration(seconds: kStallSeconds + 1));
    expect(rig.state.status.stalled, isTrue);
    expect(rig.state.status.error, 'boom');

    rig.runner.keepWaiting();
    await tester.pump();
    expect(rig.state.status.stalled, isFalse);
    expect(rig.state.status.error, isNull);
    await tester.pump(const Duration(seconds: kStallSeconds));
    expect(rig.state.status.stalled, isFalse);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.stalled, isTrue);
    await rig.leave(tester);
  });

  testWidgets('an error is kept until playback moves past where it came', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    rig.player.at(positionMs: 5000, bufferedAheadMs: 3000);
    await tester.pump();
    rig.player.fail('boom');
    await tester.pump();

    rig.player.at(positionMs: 6000, bufferedAheadMs: 2000);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.error, 'boom');
    expect(rig.state.status.stalled, isFalse);

    rig.player.at(positionMs: 7000, bufferedAheadMs: 3000);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.error, isNull);
    await rig.leave(tester);
  });

  testWidgets('an error that ends the stream where it came is a stall at '
      'the next tick', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    rig.player.at(positionMs: 90000);
    await tester.pump();

    rig.player.fail('boom');
    await tester.pump();
    rig.player.endedAt(positionMs: 90000);
    await tester.pump(const Duration(seconds: 1));

    expect(rig.state.status.stalled, isTrue);
    expect(rig.state.status.error, 'boom');
    await rig.leave(tester);
  });

  testWidgets('a lost stream ends its session and shows the failure', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.player.calls.clear();

    rig.player.lose();
    await tester.pump();

    expect(rig.state.status.error, Strings.playerFailed);
    expect(rig.state.stream, isNull);
    expect(rig.api.named('end'), <String>['end:s1']);
    expect(rig.player.calls, <String>['detach']);
    await rig.leave(tester);
  });

  testWidgets('waiting for the buffer holds the first play until it is '
      'ready', (WidgetTester tester) async {
    final Rig rig = Rig(local: const LocalSettings(waitForBuffer: true));

    await rig.answerStart(tester);
    expect(rig.player.calls, <String>['attach:direct']);
    expect(rig.state.status.holding, isTrue);

    rig.player.at(positionMs: 0, bufferedAheadMs: 59000);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.player.calls, <String>['attach:direct', 'play']);
    expect(rig.state.status.holding, isFalse);
    await rig.leave(tester);
  });

  testWidgets('a buffer that stops growing gives up and plays', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(local: const LocalSettings(waitForBuffer: true));

    await rig.answerStart(tester);
    await tester.pump(const Duration(seconds: kPrebufferGiveUpSeconds - 2));
    expect(rig.player.calls, <String>['attach:direct']);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.player.calls, <String>['attach:direct', 'play']);
    await rig.leave(tester);
  });

  testWidgets('a player that cannot buffer ahead plays as soon as it loads', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(local: const LocalSettings(waitForBuffer: true));
    rig.player.hlsBuffers = false;

    await rig.answerStart(tester);

    expect(rig.player.calls, <String>['attach:direct', 'play']);
    expect(rig.state.status.holding, isFalse);
    await rig.leave(tester);
  });

  testWidgets('a player setting applies only what changed and is saved', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 2));
    await tester.pump();
    expect(rig.player.rate, 2);
    expect(rig.player.calls, isEmpty);
    expect(rig.saved.last, const LocalSettings(speed: 2));

    rig.runner.changeLocal(
      (LocalSettings l) => l.copyWith(
        volume: 0.5,
        muted: true,
        subtitleSize: SubtitleSize.small,
        networkTimeoutS: 60,
        bufferBytes: kDefaultBufferBytes * 2,
      ),
    );
    await tester.pump();
    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 1));
    await tester.pump();
    expect(rig.player.rate, 1);
    expect(rig.player.calls, <String>[
      'volume:0.5',
      'muted:true',
      'timeout:60',
      'buffer:$kDefaultBufferSeconds',
    ]);
    expect(rig.player.subtitleSize, SubtitleSize.small);
    expect(rig.player.bufferBytes, kDefaultBufferBytes * 2);
    await rig.leave(tester);
  });

  testWidgets('what the player changes itself is kept and not undone', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.player.toggleFullscreen();
    rig.player.setVolume(0.4);
    await tester.pump();
    expect(rig.state.local.fullscreen, isTrue);
    expect(rig.state.local.volume, 0.4);
    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 2));
    await tester.pump();
    expect(rig.player.calls, <String>['fullscreen', 'volume:0.4']);

    rig.player.togglePictureInPicture();
    await tester.pump();
    expect(rig.state.local.pictureInPicture, isTrue);
    rig.player.toggleFullscreen();
    await tester.pump();
    expect(rig.state.local.fullscreen, isFalse);
    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 1));
    await tester.pump();
    expect(rig.player.calls, <String>[
      'fullscreen',
      'volume:0.4',
      'pip',
      'fullscreen',
    ]);
    await rig.leave(tester);
  });

  testWidgets('a mute the browser forced keeps the saved choice until the '
      'player lifts it', (WidgetTester tester) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.player.mutePolicy();
    await tester.pump();
    expect(rig.state.local.mutedByPolicy, isTrue);
    expect(rig.state.local.muted, isFalse);
    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 2));
    await tester.pump();
    expect(rig.player.calls, isEmpty);
    expect(rig.saved.last.muted, isFalse);

    rig.player.unmute();
    await tester.pump();
    expect(rig.state.local.mutedByPolicy, isFalse);
    expect(rig.state.local.muted, isFalse);
    rig.runner.changeLocal((LocalSettings l) => l.copyWith(speed: 1));
    await tester.pump();
    expect(rig.player.calls, <String>['muted:false', 'unmute']);
    await rig.leave(tester);
  });

  testWidgets('the title and a fresh version reach the state', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    const EpisodeLink next = EpisodeLink(
      id: 'e2',
      seasonId: 'se1',
      label: 'E2',
    );
    final VersionDetail fresh = kVersion.copyWith(container: 'mp4');

    rig.runner.describe(name: 'Pilot', previous: null, next: next);
    rig.runner.refreshVersion(fresh);
    await tester.pump();

    expect(
      rig.state.title,
      PlaybackTitle(version: fresh, name: 'Pilot', next: next),
    );
    await rig.leave(tester);
  });

  testWidgets('a forced mute on a player the viewer had muted is still seen', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(local: const LocalSettings(muted: true));
    await rig.watch(tester);

    rig.player.mutePolicy();
    await tester.pump();

    expect(rig.state.local.mutedByPolicy, isTrue);
    expect(rig.state.local.muted, isTrue);
    await rig.leave(tester);
  });

  testWidgets('a stall cannot show while a change is in flight', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);

    rig.runner.changeRemote(k720);
    await tester.pump();
    rig.player.stall(positionMs: 5000);
    await tester.pump(const Duration(seconds: kStallSeconds + 5));

    expect(rig.state.status.sent, isA<SentControls>());
    expect(rig.state.status.stalled, isFalse);
    await rig.leave(tester);
  });

  testWidgets('after leaving the player is neither read nor called', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    expect(rig.player.listened.values, everyElement(isTrue));
    await rig.leave(tester);
    expect(
      rig.player.listened.values,
      everyElement(isFalse),
      reason: '${rig.player.listened}',
    );
    final PlaybackState closed = rig.state;
    rig.api.calls.clear();
    rig.player.calls.clear();

    rig.player.at(positionMs: 9000);
    rig.player.fail('boom');
    rig.player.lose();
    rig.player.toggleFullscreen();
    rig.runner.seek(20000);
    rig.runner.play();
    await tester.pump(const Duration(seconds: 60));

    expect(rig.state, closed);
    expect(rig.api.calls, isEmpty);
    expect(rig.player.calls, <String>['fullscreen']);
  });

  testWidgets('what the player already shows at opening is in the state', (
    WidgetTester tester,
  ) async {
    final FakePlayerController player = FakePlayerController(fullscreen: true)
      ..mutePolicy();
    unawaited(player.togglePictureInPicture());

    final Rig rig = Rig(player: player);
    expect(rig.state.local.fullscreen, isTrue, reason: 'before the gather');

    await tester.pump();
    expect(rig.state.status.gathered, isTrue);
    expect(rig.state.local.fullscreen, isTrue);
    expect(rig.state.local.pictureInPicture, isTrue);
    expect(rig.state.local.mutedByPolicy, isTrue);
    expect(rig.state.local.muted, isFalse, reason: 'the saved choice');
    await rig.leave(tester);
  });

  testWidgets('an error during a load belongs to the new stream and fails at '
      'the stall point', (WidgetTester tester) async {
    final Rig rig = Rig();
    rig.player.attachGate = Completer<void>();
    await rig.answerStart(tester);

    rig.player
      ..fail('boom')
      ..stall(positionMs: 0);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.sent, isA<SentLoad>());
    expect(rig.state.status.error, 'boom');

    rig.player.attachGate!.complete();
    await tester.pump(const Duration(seconds: kStallSeconds));
    expect(rig.state.status.stalled, isFalse);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.state.status.stalled, isTrue);
    expect(rig.state.status.error, 'boom');
    await rig.leave(tester);
  });

  testWidgets('a new stream forgets the errors of the one before', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester);
    rig.player.fail('old');
    await tester.pump();
    expect(rig.state.status.error, 'old');

    rig.runner.changeRemote(k720);
    await tester.pump();
    rig.api.answers.single.complete(kRenegotiated);
    await tester.pump();

    expect(rig.state.status.error, isNull);
    await rig.leave(tester);
  });

  testWidgets('a replay whose server seek fails is back at the end, paused', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig(resumeMs: 700000);
    await rig.watch(
      tester,
      session: kTranscode.copyWith(originMs: 600000),
      atMs: 700000,
    );
    rig.player.endedAt(positionMs: 100000);
    await tester.pump();

    rig.runner.replay();
    await tester.pump();
    expect(rig.state.status.sent, isA<SentSeek>());
    rig.api.answers.single.completeError(Exception('no'));
    await tester.pump();
    await tester.pump();

    expect(rig.state.status.sent, isNull);
    expect(rig.state.timeline.ended, isTrue);
    expect(rig.state.timeline.playing, isFalse);
    expect(rig.state.timeline.positionMs, 700000);
    expect(rig.player.calls, isNot(contains('play')));
    await rig.leave(tester);
  });

  testWidgets('replay at the end plays from the start of the title', (
    WidgetTester tester,
  ) async {
    final Rig local = Rig();
    await local.watch(tester, atMs: 5000);
    local.player.endedAt(positionMs: 100000);
    await tester.pump();
    local.player.calls.clear();

    local.runner.replay();
    await tester.pump();
    expect(local.player.calls, <String>['seek', 'play']);
    expect(local.player.seekedMs, 0);
    expect(local.state.timeline.ended, isFalse);
    expect(local.state.timeline.playing, isTrue);
    await local.leave(tester);

    final Rig resumed = Rig(resumeMs: 700000);
    await resumed.watch(
      tester,
      session: kTranscode.copyWith(originMs: 600000),
      atMs: 700000,
    );
    resumed.player.endedAt(positionMs: 100000);
    await tester.pump();
    resumed.api.calls.clear();

    resumed.runner.replay();
    await tester.pump();
    expect(resumed.api.calls, <String>['seek:s1@0']);
    expect(resumed.state.timeline.playing, isTrue);
    await resumed.leave(tester);
  });

  testWidgets('a start waits for the end before it at most the network wait', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.api
      ..holdProgress = true
      ..holdEnds = true;
    rig.runner.background();
    await tester.pump();

    rig.runner.foreground(playing: true);
    await tester.pump(
      const Duration(seconds: kDefaultNetworkTimeoutSeconds - 1),
    );
    expect(rig.api.named('start'), isEmpty);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.api.named('start'), <String>['start:v1@5000:7:linux']);

    rig.api.progresses.single.complete();
    rig.api.ends.single.complete();
    await rig.leave(tester);
  });

  testWidgets('an end waits for the end before it at most the network wait', (
    WidgetTester tester,
  ) async {
    final Rig rig = Rig();
    await rig.watch(tester, atMs: 5000);
    rig.api
      ..holdProgress = true
      ..holdEnds = true;
    rig.runner.background();
    await tester.pump();
    rig.runner.foreground(playing: true);
    await tester.pump(const Duration(seconds: kDefaultNetworkTimeoutSeconds));
    rig.api.starts.last.complete(kDirect.copyWith(sessionId: 's2'));
    await tester.pump();
    rig.api.calls.clear();

    unawaited(rig.runner.leave());
    await tester.pump(
      const Duration(seconds: kDefaultNetworkTimeoutSeconds - 1),
    );
    expect(rig.api.named('end'), isEmpty);
    await tester.pump(const Duration(seconds: 1));
    expect(rig.api.named('end'), <String>['end:s2']);

    for (final Completer<void> held in <Completer<void>>[
      ...rig.api.progresses,
      ...rig.api.ends,
    ]) {
      held.complete();
    }
    await tester.pump();
  });

  testWidgets('a gather that lands in the background starts only when the '
      'page comes back', (WidgetTester tester) async {
    final Rig rig = Rig(gathered: false);
    rig.runner.background();
    rig.gather(resumeMs: 5000);
    await tester.pump(const Duration(seconds: 30));
    expect(rig.api.calls, isEmpty);

    rig.runner.foreground(playing: true);
    await tester.pump();
    expect(rig.api.calls, <String>['start:v1@5000:7:linux']);
    await rig.leave(tester);
  });

  testWidgets('nothing is asked before the page has gathered, and leaving '
      'first drops the gather', (WidgetTester tester) async {
    final Rig rig = Rig(gathered: false);
    await tester.pump(const Duration(seconds: 30));
    expect(rig.api.calls, isEmpty);
    expect(rig.player.calls, isEmpty);

    await rig.leave(tester);
    rig.gather(resumeMs: 5000);
    await tester.pump(const Duration(seconds: 30));
    expect(rig.api.calls, isEmpty);
    expect(rig.player.calls, isEmpty);
  });
}
