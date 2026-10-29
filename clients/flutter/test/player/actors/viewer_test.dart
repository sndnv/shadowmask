import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/player/actors/lifecycle.dart';
import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/actors/viewer.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

import '../../support/playback_run.dart';

const PlaybackControls k720 = PlaybackControls(
  height: 720,
  delivery: DeliveryPreference.auto,
  downmix: false,
  normalize: true,
  dialogue: DialogueLift.none,
  night: NightMode.off,
);

const VersionDetail kSurround = VersionDetail(
  id: 'v1',
  title: TitleRef(type: TitleKind.movie, id: 't1'),
  libraryId: 'lib',
  quality: Quality.hd,
  container: 'mkv',
  durationMs: kTitleMs,
  audio: <AudioTrack>[AudioTrack(index: 1, codec: 'eac3', channels: 6)],
);

const VersionDetail kStereo = VersionDetail(
  id: 'v1',
  title: TitleRef(type: TitleKind.movie, id: 't1'),
  libraryId: 'lib',
  quality: Quality.hd,
  container: 'mkv',
  durationMs: kTitleMs,
  audio: <AudioTrack>[AudioTrack(index: 1, codec: 'aac', channels: 2)],
);

PlaybackSession _sending(int channels) => kDirect.copyWith(
  selected: const SelectedTracks(audioTrack: 1),
  audioChannels: channels,
);

Map<String, PlaybackRun Function()> _inFlight() =>
    <String, PlaybackRun Function()>{
      'start': PlaybackRun.opened,
      'load': () {
        final PlaybackRun run = PlaybackRun.opened();
        run.apply(
          (PlaybackState s) => serverStarted(
            s,
            id: run.sentId,
            session: kDirect,
            positionMs: 0,
            seeksInStream: true,
          ),
        );
        return run;
      },
      'update': () =>
          PlaybackRun.watching()
            ..apply((PlaybackState s) => viewerChangesRemote(s, k720)),
      'seek': () =>
          PlaybackRun.watching(session: kTranscode)
            ..apply((PlaybackState s) => viewerSeeks(s, 600000)),
    };

void main() {
  test('a local change applies at once and is saved, even during a change', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    const LocalSettings bigger = LocalSettings(
      subtitleSize: SubtitleSize.large,
    );

    expect(
      run.apply((PlaybackState s) => viewerChangesLocal(s, bigger)),
      const <PlaybackCommand>[ApplyLocal(bigger), SaveLocal(bigger)],
    );
    expect(run.state.local, bigger);
    expect(run.state.status.sent, isA<SentControls>());
  });

  test('a seek the stream can reach moves the player and shows the target', () {
    final PlaybackRun run = PlaybackRun.watching();

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 335000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 335000)],
    );
    expect(run.state.timeline.positionMs, 335000);
    expect(run.state.timeline.seekingToMs, 335000);
    expect(run.state.status.sent, isNull);
  });

  test('steps add up before the player catches up', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 60000)
      ..apply((PlaybackState s) => viewerSeeksBy(s, 10000));

    expect(
      run.apply((PlaybackState s) => viewerSeeksBy(s, 10000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 80000)],
    );
    expect(run.state.timeline.positionMs, 80000);
  });

  test('steps on a stream still being produced reach from what the player '
      'has read, not from the targets held', () {
    final PlaybackRun run = PlaybackRun.watching(session: kTranscode);

    for (final int target in <int>[10000, 20000, kSeekReachMs]) {
      expect(
        run.apply((PlaybackState s) => viewerSeeksBy(s, 10000)),
        <PlaybackCommand>[SeekPlayer(offsetMs: target)],
      );
    }
    expect(
      run.apply((PlaybackState s) => viewerSeeksBy(s, 10000)).last,
      SendSeek(
        id: run.sentId,
        sessionId: 's1',
        positionMs: kSeekReachMs + 10000,
      ),
    );
  });

  test('a reading that settles moves the reach with it', () {
    final PlaybackRun run = PlaybackRun.watching(session: kTranscode)
      ..apply((PlaybackState s) => viewerSeeks(s, 20000))
      ..apply(
        (PlaybackState s) => playerReads(
          s,
          const PlayerSnapshot(
            positionMs: 20000,
            bufferedAheadMs: 5000,
            playing: true,
          ),
        ),
      );

    expect(run.state.timeline.reachMs, 25000);
    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 25000 + kSeekReachMs)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 25000 + kSeekReachMs)],
    );
  });

  test('a server change clears a stall', () {
    final PlaybackRun update = PlaybackRun.watching()
      ..apply((PlaybackState s) => stallChanged(s, true))
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    expect(update.state.status.stalled, isFalse);

    final PlaybackRun seek = PlaybackRun.watching(session: kTranscode)
      ..apply((PlaybackState s) => stallChanged(s, true))
      ..apply((PlaybackState s) => viewerSeeks(s, 600000));
    expect(seek.state.status.stalled, isFalse);
  });

  test('toggle decides from the state it is applied to: pause, play, or '
      'replay at the end', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 5000);

    expect(run.apply(viewerToggles).first, const PausePlayer());
    expect(run.state.timeline.playing, isFalse);
    expect(run.apply(viewerToggles).first, const PlayPlayer());
    expect(run.state.timeline.playing, isTrue);

    run.apply(
      (PlaybackState s) => playerReads(
        s,
        const PlayerSnapshot(positionMs: kTitleMs, ended: true),
      ),
    );
    expect(run.apply(viewerToggles), const <PlaybackCommand>[
      SeekPlayer(offsetMs: 0),
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: true),
    ]);
  });

  test('toggle waits for a change in flight', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final PlaybackState before = run.state;

    expect(run.apply(viewerToggles), isEmpty);
    expect(run.state, before);
  });

  test('a seek to a share of the title needs its length', () {
    final PlaybackRun run = PlaybackRun.watching();
    expect(
      run.apply((PlaybackState s) => viewerSeeksToFraction(s, 0.5)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: kTitleMs ~/ 2)],
    );

    final PlaybackRun unknown = PlaybackRun.watching(
      version: kVersion.copyWith(durationMs: 0),
    );
    expect(
      unknown.apply((PlaybackState s) => viewerSeeksToFraction(s, 0.5)),
      isEmpty,
    );
  });

  test('a step on a title with no known length does nothing', () {
    final PlaybackRun run = PlaybackRun.watching(
      version: kVersion.copyWith(durationMs: 0),
    );

    expect(run.apply((PlaybackState s) => viewerSeeksBy(s, 10000)), isEmpty);
  });

  test('a replay whose server seek fails or times out is paused again', () {
    PlaybackRun ended() =>
        PlaybackRun.watching(
          session: kTranscode.copyWith(originMs: 600000),
          atMs: 700000,
        )..apply(
          (PlaybackState s) => playerReads(
            s,
            const PlayerSnapshot(positionMs: 100000, ended: true),
          ),
        );

    final PlaybackRun failed = ended()..apply(viewerReplays);
    expect(
      failed.apply((PlaybackState s) => serverFailed(s, failed.sentId)),
      isEmpty,
    );
    expect(failed.state.timeline.playing, isFalse);
    expect(failed.state.timeline.positionMs, 700000);

    final PlaybackRun timedOut = ended()..apply(viewerReplays);
    timedOut.apply(
      (PlaybackState s) =>
          changeTimedOut(s, s.status.sent!, message: 'timeout'),
    );
    expect(timedOut.state.timeline.playing, isFalse);
  });

  test('replay plays from the start of the title, through the server when '
      'the stream starts later', () {
    final PlaybackRun run =
        PlaybackRun.watching(
          session: kTranscode.copyWith(originMs: 600000),
          atMs: 700000,
        )..apply(
          (PlaybackState s) => playerReads(
            s,
            const PlayerSnapshot(positionMs: 100000, ended: true),
          ),
        );

    expect(run.apply(viewerReplays), <PlaybackCommand>[
      const PausePlayer(),
      SendSeek(id: run.sentId, sessionId: 's1', positionMs: 0),
    ]);
    expect(run.state.timeline.playing, isTrue);
    expect(run.state.timeline.ended, isFalse);

    final PlaybackRun local = PlaybackRun.watching(atMs: 90000)
      ..apply(
        (PlaybackState s) => playerReads(
          s,
          const PlayerSnapshot(positionMs: kTitleMs, ended: true),
        ),
      );
    expect(local.apply(viewerReplays), const <PlaybackCommand>[
      SeekPlayer(offsetMs: 0),
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: true),
    ]);
  });

  test('seeks are kept inside the title', () {
    final PlaybackRun run = PlaybackRun.watching();

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, -5000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 0)],
    );
    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, kTitleMs + 5000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: kTitleMs)],
    );
  });

  test('a seek past what the stream has produced is a server change', () {
    final PlaybackRun run = PlaybackRun.watching(session: kTranscode);

    final List<PlaybackCommand> sent = run.apply(
      (PlaybackState s) => viewerSeeks(s, 600000),
    );

    expect(sent, <PlaybackCommand>[
      const PausePlayer(),
      SendSeek(id: run.sentId, sessionId: 's1', positionMs: 600000),
    ]);
    expect(
      run.state.status.sent,
      SentChange.seek(
        id: run.sentId,
        positionMs: 600000,
        previousMs: 0,
        previousPlaying: true,
      ),
    );
    expect(run.state.timeline.positionMs, 600000);
  });

  test(
    'within reach of what is produced a sequential stream seeks locally',
    () {
      final PlaybackRun run = PlaybackRun.watching(session: kTranscode);

      expect(
        run.apply((PlaybackState s) => viewerSeeks(s, 20000)),
        const <PlaybackCommand>[SeekPlayer(offsetMs: 20000)],
      );
    },
  );

  test(
    'a sequential stream the player cannot seek inside goes to the server',
    () {
      final PlaybackRun run = PlaybackRun.watching(
        session: kTranscode,
        seeksInStream: false,
      );

      expect(
        run.apply((PlaybackState s) => viewerSeeks(s, 5000)).last,
        isA<SendSeek>(),
      );
    },
  );

  test('a seek before where the stream starts goes to the server', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode.copyWith(originMs: 600000),
      atMs: 600000,
    );

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 300000)).last,
      isA<SendSeek>(),
    );
  });

  test('a local seek on a stream that starts later moves inside it', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode.copyWith(originMs: 600000),
      atMs: 600000,
    );

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 610000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 10000)],
    );
    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 600000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 0)],
      reason: 'where the stream starts is inside it',
    );
  });

  test('a seek stays local up to the reach past what is produced', () {
    expect(
      PlaybackRun.watching(
        session: kTranscode,
      ).apply((PlaybackState s) => viewerSeeks(s, kSeekReachMs)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: kSeekReachMs)],
    );
    expect(
      PlaybackRun.watching(
        session: kTranscode,
      ).apply((PlaybackState s) => viewerSeeks(s, kSeekReachMs + 1)).last,
      isA<SendSeek>(),
    );
  });

  test('what is buffered ahead extends how far a seek stays local', () {
    final PlaybackRun run = PlaybackRun.watching(session: kTranscode)
      ..apply(
        (PlaybackState s) => playerReads(
          s,
          const PlayerSnapshot(bufferedAheadMs: 60000, playing: true),
        ),
      );

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 60000 + kSeekReachMs)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 60000 + kSeekReachMs)],
    );
  });

  test('a seek on a title with no known length is not cut short', () {
    final PlaybackRun run = PlaybackRun.watching(
      version: kVersion.copyWith(durationMs: 0),
    );

    expect(
      run.apply((PlaybackState s) => viewerSeeks(s, 500000)),
      const <PlaybackCommand>[SeekPlayer(offsetMs: 500000)],
    );
  });

  test('seeking back from the end is no longer at the end', () {
    const PlayerSnapshot end = PlayerSnapshot(
      positionMs: kTitleMs,
      ended: true,
    );
    final PlaybackRun local = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerReads(s, end))
      ..apply((PlaybackState s) => viewerSeeks(s, 0));
    expect(local.state.timeline.ended, isFalse);

    final PlaybackRun restarted = PlaybackRun.watching(session: kTranscode)
      ..apply(
        (PlaybackState s) => playerReads(
          s,
          const PlayerSnapshot(positionMs: 3000000, ended: true),
        ),
      )
      ..apply((PlaybackState s) => viewerSeeks(s, 6000000));
    expect(restarted.commands.last, isA<SendSeek>());
    expect(restarted.state.timeline.ended, isFalse);
  });

  test('a retry while waiting for the buffer starts over without the wait', () {
    final PlaybackRun run = PlaybackRun.watching(
      local: const LocalSettings(waitForBuffer: true),
    )..apply((PlaybackState s) => playerErrs(s, 'decoder'));
    expect(run.state.status.holding, isTrue);

    run.apply(viewerRetries);

    expect(run.state.status.holding, isFalse);
  });

  test('play and pause go to the player at once and end the buffer wait', () {
    final PlaybackRun run = PlaybackRun.watching(
      local: const LocalSettings(waitForBuffer: true),
    );
    expect(run.state.status.holding, isTrue);

    expect(run.apply(viewerPlays), const <PlaybackCommand>[
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: true),
    ], reason: 'play now');
    expect(run.state.status.holding, isFalse);
    expect(run.apply(viewerPauses), const <PlaybackCommand>[
      PausePlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: false),
    ]);
    expect(run.state.timeline.playing, isFalse);
  });

  test('a pause or a play that changes what is shown reports it at once', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 5000);

    expect(run.apply(viewerPauses), const <PlaybackCommand>[
      PausePlayer(),
      SendProgress(sessionId: 's1', positionMs: 5000, playing: false),
    ]);
    expect(run.apply(viewerPauses), const <PlaybackCommand>[PausePlayer()]);
    expect(run.apply(viewerPlays), const <PlaybackCommand>[
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 5000, playing: true),
    ]);
  });

  for (final MapEntry<String, PlaybackRun Function()> kind
      in _inFlight().entries) {
    test('seek, play, pause and stream settings wait for a ${kind.key}', () {
      final PlaybackRun run = kind.value();
      final PlaybackState before = run.state;

      for (final PlaybackWrite write in <PlaybackWrite>[
        (PlaybackState s) => viewerSeeks(s, 20000),
        (PlaybackState s) => viewerSeeksBy(s, 10000),
        viewerPlays,
        viewerPauses,
        (PlaybackState s) =>
            viewerChangesRemote(s, const PlaybackControls(height: 480)),
        viewerRetries,
      ]) {
        expect(run.apply(write), isEmpty);
        expect(run.state, before);
      }
    });
  }

  test('a stream setting pauses, asks the server where the viewer is and '
      'blocks', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000);
    final PlaybackControls before = run.state.remote;

    final List<PlaybackCommand> sent = run.apply(
      (PlaybackState s) => viewerChangesRemote(s, k720),
    );

    expect(sent, <PlaybackCommand>[
      const PausePlayer(),
      SendUpdate(
        id: run.sentId,
        sessionId: 's1',
        controls: k720,
        positionMs: 90000,
      ),
    ]);
    expect(run.state.remote, k720);
    expect(
      run.state.status.sent,
      SentChange.controls(
        id: run.sentId,
        controls: k720,
        previous: before,
        positionMs: 90000,
      ),
    );
  });

  test(
    'a stream setting that sends the same update only changes the value',
    () {
      final PlaybackRun run = PlaybackRun.watching();
      final PlaybackControls offset = run.state.remote.copyWith(offsetMs: 1500);

      expect(
        run.apply((PlaybackState s) => viewerChangesRemote(s, offset)),
        isEmpty,
        reason: 'an offset with no subtitle on is not sent',
      );
      expect(run.state.remote, offset);
      expect(run.state.status.sent, isNull);
    },
  );

  group('sound', () {
    test('a player that mixes surround itself is tuned without the server', () {
      final PlaybackRun run = PlaybackRun.watching(
        session: _sending(6),
        version: kSurround,
        mixesSound: true,
      );
      final PlaybackControls tuned = run.state.remote.copyWith(
        normalize: false,
        dialogue: DialogueLift.db6,
      );

      expect(
        run.apply((PlaybackState s) => viewerChangesRemote(s, tuned)),
        const <PlaybackCommand>[
          TuneSound(normalize: false, dialogue: DialogueLift.db6),
        ],
      );
      expect(run.state.remote, tuned);
      expect(run.state.status.sent, isNull);
    });

    test('a lift on surround a player sends on unmixed goes to the server', () {
      final PlaybackRun run = PlaybackRun.watching(
        session: _sending(6),
        version: kSurround,
      );
      final PlaybackControls lifted = run.state.remote.copyWith(
        dialogue: DialogueLift.db3,
      );

      final List<PlaybackCommand> sent = run.apply(
        (PlaybackState s) => viewerChangesRemote(s, lifted),
      );

      expect(sent.first, const PausePlayer());
      expect(sent.whereType<SendUpdate>().single.controls, lifted);
    });

    test('a lift on what the server sends as stereo goes to the server', () {
      final PlaybackRun run = PlaybackRun.watching(
        session: _sending(2),
        version: kSurround,
        mixesSound: true,
      );
      final PlaybackControls lifted = run.state.remote.copyWith(
        dialogue: DialogueLift.db9,
      );

      expect(
        run
            .apply((PlaybackState s) => viewerChangesRemote(s, lifted))
            .whereType<SendUpdate>(),
        hasLength(1),
      );
    });

    test('normalize goes to the server when the server folds', () {
      final PlaybackRun run = PlaybackRun.watching(
        session: _sending(2),
        version: kSurround,
        mixesSound: true,
      );
      final PlaybackControls loud = run.state.remote.copyWith(normalize: false);

      expect(
        run
            .apply((PlaybackState s) => viewerChangesRemote(s, loud))
            .whereType<SendUpdate>(),
        hasLength(1),
      );
    });

    test('normalize goes to the server when it folds a lift on stereo', () {
      for (final bool mixesSound in <bool>[true, false]) {
        final PlaybackRun run = PlaybackRun.watching(
          session: _sending(2),
          version: kStereo,
          controls: k720.copyWith(dialogue: DialogueLift.db6),
          mixesSound: mixesSound,
        );
        final PlaybackControls loud = run.state.remote.copyWith(
          normalize: false,
        );

        expect(
          run
              .apply((PlaybackState s) => viewerChangesRemote(s, loud))
              .whereType<SendUpdate>(),
          hasLength(1),
          reason: 'mixes sound: $mixesSound',
        );
      }
    });

    test('normalize on stereo with no lift stays with the player', () {
      final PlaybackRun run = PlaybackRun.watching(
        session: _sending(2),
        version: kStereo,
        controls: k720,
        mixesSound: true,
      );
      final PlaybackControls loud = run.state.remote.copyWith(normalize: false);

      expect(
        run.apply((PlaybackState s) => viewerChangesRemote(s, loud)),
        const <PlaybackCommand>[
          TuneSound(normalize: false, dialogue: DialogueLift.none),
        ],
      );
    });

    test('night mode and a stereo downmix always go to the server', () {
      for (final PlaybackControls Function(PlaybackControls) change
          in <PlaybackControls Function(PlaybackControls)>[
            (PlaybackControls c) => c.copyWith(night: NightMode.light),
            (PlaybackControls c) => c.copyWith(downmix: true),
          ]) {
        final PlaybackRun run = PlaybackRun.watching(
          session: _sending(6),
          version: kSurround,
          mixesSound: true,
        );

        expect(
          run
              .apply(
                (PlaybackState s) =>
                    viewerChangesRemote(s, change(run.state.remote)),
              )
              .whereType<SendUpdate>(),
          hasLength(1),
        );
      }
    });
  });

  test('a stream setting with no stream is kept for the next start', () {
    final PlaybackRun failed = PlaybackRun.opened();
    failed
      ..apply((PlaybackState s) => serverRefusedStart(s, failed.sentId, 'no'))
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    expect(failed.commands, isEmpty);
    expect(
      failed.apply(viewerRetries).single,
      SendStart(id: failed.sentId, controls: k720, positionMs: 0),
    );

    final PlaybackRun away = PlaybackRun.watching()
      ..apply(lifecycleBackgrounds)
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    expect(away.commands, isEmpty);
    expect(
      away
          .apply((PlaybackState s) => lifecycleForegrounds(s, playing: true))
          .single,
      SendStart(id: away.sentId, controls: k720, positionMs: 0),
    );
  });

  test('a retry after a failed start starts again where it was going', () {
    final PlaybackRun run = PlaybackRun.opened(resumeMs: 42000);
    run.apply((PlaybackState s) => serverRefusedStart(s, run.sentId, 'no'));

    expect(run.apply(viewerRetries), <PlaybackCommand>[
      SendStart(id: 2, controls: run.state.remote, positionMs: 42000),
    ]);
    expect(run.state.status.error, isNull);
    expect(
      run.state.status.sent,
      const SentChange.start(id: 2, positionMs: 42000),
    );
  });

  test('a retry after a player error ends that session and starts again', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'));

    expect(run.apply(viewerRetries), <PlaybackCommand>[
      const EndSession(sessionId: 's1', positionMs: 0, playing: true),
      SendStart(id: run.sentId, controls: run.state.remote, positionMs: 0),
    ]);
    expect(run.state.stream, isNull);
  });

  test('a retry with nothing failed does nothing', () {
    expect(PlaybackRun.watching().apply(viewerRetries), isEmpty);
  });

  test('a retry waits while a load is in flight or the page is away, even '
      'with an error held', () {
    final PlaybackRun loading = PlaybackRun.opened();
    loading
      ..apply(
        (PlaybackState s) => serverStarted(
          s,
          id: loading.sentId,
          session: kDirect,
          positionMs: 0,
          seeksInStream: true,
        ),
      )
      ..apply((PlaybackState s) => playerErrs(s, 'tcp: refused'));
    expect(loading.state.status.error, 'tcp: refused');
    expect(loading.apply(viewerRetries), isEmpty);

    PlaybackRun refused() =>
        PlaybackRun.opened()
          ..apply((PlaybackState s) => serverRefusedStart(s, 1, 'no'));
    expect(
      (refused()..apply(lifecycleBackgrounds)).apply(viewerRetries),
      isEmpty,
    );
    expect((refused()..apply(lifecycleLeaves)).apply(viewerRetries), isEmpty);
  });

  test('keep waiting clears a stall and its error, and not a start error', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'))
      ..apply((PlaybackState s) => stallChanged(s, true));

    expect(run.apply(viewerKeepsWaiting), isEmpty);
    expect(run.state.status.stalled, isFalse);
    expect(run.state.status.error, isNull);

    final PlaybackRun refused = PlaybackRun.opened()
      ..apply((PlaybackState s) => serverRefusedStart(s, 1, 'no'));
    final PlaybackState before = refused.state;
    refused.apply(viewerKeepsWaiting);
    expect(refused.state, before);
  });

  test('keep waiting during a load keeps its error', () {
    final PlaybackRun run = PlaybackRun.opened();
    run
      ..apply(
        (PlaybackState s) => serverStarted(
          s,
          id: run.sentId,
          session: kDirect,
          positionMs: 0,
          seeksInStream: true,
        ),
      )
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'));
    expect(run.state.status.error, 'decoder');
    final PlaybackState before = run.state;

    expect(run.apply(viewerKeepsWaiting), isEmpty);
    expect(run.state, before);
  });

  test('nothing the viewer does after leaving is sent', () {
    final PlaybackRun run = PlaybackRun.watching()..apply(lifecycleLeaves);

    for (final PlaybackWrite write in <PlaybackWrite>[
      (PlaybackState s) => viewerSeeks(s, 20000),
      viewerPlays,
      (PlaybackState s) => viewerChangesRemote(s, k720),
      (PlaybackState s) => viewerChangesLocal(s, const LocalSettings()),
      viewerRetries,
    ]) {
      expect(run.apply(write), isEmpty);
    }
  });
}
