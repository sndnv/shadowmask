import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/actors/lifecycle.dart';
import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/actors/viewer.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/playback_controls.dart';

import '../../support/playback_run.dart';

const PlaybackControls k720 = PlaybackControls(
  height: 720,
  delivery: DeliveryPreference.auto,
);

void main() {
  test('opening applies the local settings, takes the delivery preference '
      'and starts at the resume point', () {
    const LocalSettings local = LocalSettings(
      defaultDelivery: DeliveryPreference.always,
    );
    const PlaybackControls asked = PlaybackControls(
      height: 720,
      delivery: DeliveryPreference.always,
    );

    final PlaybackRun run = PlaybackRun.created(
      controls: const PlaybackControls(height: 720),
    );
    expect(run.commands, isEmpty);
    expect(run.state.status.gathered, isFalse);

    expect(run.gather(local: local, resumeMs: 42000), const <PlaybackCommand>[
      ApplyLocal(local),
      SendStart(id: 1, controls: asked, positionMs: 42000),
    ]);
    final PlaybackState state = run.state;
    expect(state.remote, asked);
    expect(state.timeline.positionMs, 42000);
    expect(state.timeline.playing, isTrue);
    expect(state.title, const PlaybackTitle(version: kVersion));
    expect(state.status.gathered, isTrue);
    expect(state.status.sent, const SentChange.start(id: 1, positionMs: 42000));
  });

  test('what the player showed before the gather is kept over the '
      'preferences', () {
    final PlaybackRun run = PlaybackRun.created(
      local: const LocalSettings(
        fullscreen: true,
        pictureInPicture: true,
        mutedByPolicy: true,
      ),
    )..gather(local: const LocalSettings(volume: 0.4, muted: true));

    expect(run.state.local.fullscreen, isTrue);
    expect(run.state.local.pictureInPicture, isTrue);
    expect(run.state.local.mutedByPolicy, isTrue);
    expect(run.state.local.volume, 0.4);
    expect(run.state.local.muted, isTrue);
  });

  test('a gather after leaving or a second gather does nothing', () {
    final PlaybackRun left = PlaybackRun.created()..apply(lifecycleLeaves);
    expect(left.gather(), isEmpty);
    expect(left.state.status.gathered, isFalse);

    final PlaybackRun twice = PlaybackRun.opened(resumeMs: 42000);
    final PlaybackState once = twice.state;
    expect(twice.gather(resumeMs: 0), isEmpty);
    expect(twice.state, once);
  });

  test('a freshly opened session is not ended, waiting, slow, stalled or '
      'holding', () {
    final PlaybackState state = PlaybackRun.opened().state;

    expect(state.timeline.ended, isFalse);
    expect(state.timeline.waiting, isFalse);
    expect(state.status.slow, isFalse);
    expect(state.status.stalled, isFalse);
    expect(state.status.holding, isFalse);
    expect(state.status.error, isNull);
    expect(state.status.notice, isNull);
  });

  test('a stall reported during a start, a load or a change is ignored', () {
    final PlaybackRun start = PlaybackRun.opened()
      ..apply((PlaybackState s) => stallChanged(s, true));
    expect(start.state.status.stalled, isFalse);

    final PlaybackRun load = PlaybackRun.opened();
    load.apply(
      (PlaybackState s) => serverStarted(
        s,
        id: load.sentId,
        session: kDirect,
        positionMs: 0,
        seeksInStream: true,
      ),
    );
    expect(load.state.stream, isNotNull);
    load.apply((PlaybackState s) => stallChanged(s, true));
    expect(load.state.status.stalled, isFalse);

    final PlaybackRun change = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    expect(change.state.stream, isNotNull);
    change.apply((PlaybackState s) => stallChanged(s, true));
    expect(change.state.status.stalled, isFalse);
  });

  test('every way a change ends leaves nothing slow', () {
    final Map<String, PlaybackRun Function()> ends =
        <String, PlaybackRun Function()>{
          'a refused start': () {
            final PlaybackRun run = PlaybackRun.opened()
              ..apply((PlaybackState s) => changeSlow(s, 1));
            return run
              ..apply((PlaybackState s) => serverRefusedStart(s, 1, 'no'));
          },
          'a failed update': () {
            final PlaybackRun run = PlaybackRun.watching()
              ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
            final int id = run.sentId;
            return run
              ..apply((PlaybackState s) => changeSlow(s, id))
              ..apply((PlaybackState s) => serverFailed(s, id));
          },
          'a failed seek': () {
            final PlaybackRun run = PlaybackRun.watching(session: kTranscode)
              ..apply((PlaybackState s) => viewerSeeks(s, 600000));
            final int id = run.sentId;
            return run
              ..apply((PlaybackState s) => changeSlow(s, id))
              ..apply((PlaybackState s) => serverFailed(s, id));
          },
          'a load': () {
            final PlaybackRun run = PlaybackRun.opened()
              ..apply((PlaybackState s) => changeSlow(s, 1))
              ..apply(
                (PlaybackState s) => serverStarted(
                  s,
                  id: 1,
                  session: kDirect,
                  positionMs: 0,
                  seeksInStream: true,
                ),
              );
            return run..apply((PlaybackState s) => playerLoaded(s, 1));
          },
          'a failed load': () {
            final PlaybackRun run = PlaybackRun.opened()
              ..apply((PlaybackState s) => changeSlow(s, 1))
              ..apply(
                (PlaybackState s) => serverStarted(
                  s,
                  id: 1,
                  session: kDirect,
                  positionMs: 0,
                  seeksInStream: true,
                ),
              );
            return run
              ..apply((PlaybackState s) => playerFailedToLoad(s, 1, 'no'));
          },
          'going to the background': () => PlaybackRun.opened()
            ..apply((PlaybackState s) => changeSlow(s, 1))
            ..apply(lifecycleBackgrounds),
          'leaving': () => PlaybackRun.opened()
            ..apply((PlaybackState s) => changeSlow(s, 1))
            ..apply(lifecycleLeaves),
        };

    for (final MapEntry<String, PlaybackRun Function()> end in ends.entries) {
      expect(end.value().state.status.slow, isFalse, reason: end.key);
    }
  });

  test('going to the background or leaving clears a stall and the buffer '
      'wait', () {
    PlaybackRun stalled() =>
        PlaybackRun.watching(local: const LocalSettings(waitForBuffer: true))
          ..apply((PlaybackState s) => stallChanged(s, true));

    final PlaybackRun away = stalled()..apply(lifecycleBackgrounds);
    expect(away.state.status.stalled, isFalse);
    expect(away.state.status.holding, isFalse);

    final PlaybackRun left = stalled()..apply(lifecycleLeaves);
    expect(left.state.status.stalled, isFalse);
    expect(left.state.status.holding, isFalse);
  });

  test('a new stream after the end is not at its end', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply(
        (PlaybackState s) => playerReads(
          s,
          const PlayerSnapshot(positionMs: kTitleMs, ended: true),
        ),
      )
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final int id = run.sentId;

    run.apply(
      (PlaybackState s) => serverUpdated(
        s,
        id: id,
        negotiation: kRenegotiated,
        seeksInStream: true,
      ),
    );

    expect(run.state.timeline.ended, isFalse);
  });

  test('a delivery chosen for the video wins over the preference', () {
    final PlaybackState state = PlaybackRun.opened(
      local: const LocalSettings(defaultDelivery: DeliveryPreference.always),
      controls: const PlaybackControls(delivery: DeliveryPreference.never),
    ).state;

    expect(state.remote.delivery, DeliveryPreference.never);
  });

  test('going to the background pauses, ends the session with its last '
      'progress and unloads', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000);
    final PlaybackControls remote = run.state.remote;

    expect(run.apply(lifecycleBackgrounds), const <PlaybackCommand>[
      PausePlayer(),
      EndSession(sessionId: 's1', positionMs: 90000, playing: true),
      UnloadPlayer(),
    ]);
    expect(run.state.status.life, PlaybackLife.background);
    expect(run.state.stream, isNull);
    expect(run.state.timeline.positionMs, 90000);
    expect(run.state.remote, remote);
    expect(run.apply(lifecycleBackgrounds), isEmpty);
  });

  test('going to the background during a change drops its answer and keeps '
      'what was asked', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000)
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final int id = run.sentId;

    expect(
      run.apply(lifecycleBackgrounds)[1],
      const EndSession(sessionId: 's1', positionMs: 90000),
    );
    expect(
      run.apply(
        (PlaybackState s) => serverUpdated(
          s,
          id: id,
          negotiation: kRenegotiated,
          seeksInStream: true,
        ),
      ),
      isEmpty,
    );
    expect(
      run
          .apply((PlaybackState s) => lifecycleForegrounds(s, playing: true))
          .single,
      SendStart(id: run.sentId, controls: k720, positionMs: 90000),
    );
  });

  test('going to the background during the start ends the late session when '
      'it answers', () {
    final PlaybackRun run = PlaybackRun.opened();

    expect(run.apply(lifecycleBackgrounds), const <PlaybackCommand>[
      PausePlayer(),
      UnloadPlayer(),
    ]);
    expect(
      run.apply(
        (PlaybackState s) => serverStarted(
          s,
          id: 1,
          session: kDirect,
          positionMs: 0,
          seeksInStream: true,
        ),
      ),
      const <PlaybackCommand>[EndSession(sessionId: 's1')],
    );
  });

  test('coming back starts again where the viewer was', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000)
      ..apply(lifecycleBackgrounds);

    expect(
      run.apply((PlaybackState s) => lifecycleForegrounds(s, playing: false)),
      <PlaybackCommand>[
        SendStart(id: 2, controls: run.state.remote, positionMs: 90000),
      ],
    );
    expect(run.state.timeline.playing, isFalse);
    expect(run.state.status.life, PlaybackLife.open);
  });

  test('coming back while not in the background does nothing', () {
    expect(
      PlaybackRun.watching().apply(
        (PlaybackState s) => lifecycleForegrounds(s, playing: true),
      ),
      isEmpty,
    );
  });

  test('leaving ends the session once and nothing is sent after it', () {
    final PlaybackRun run = PlaybackRun.watching();

    expect(run.apply(lifecycleLeaves), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 0, playing: true),
    ]);
    for (final PlaybackWrite write in <PlaybackWrite>[
      lifecycleLeaves,
      lifecycleBackgrounds,
      heartbeatDue,
      (PlaybackState s) => changeSlow(s, 1),
      (PlaybackState s) => changeTimedOut(
        s,
        const SentChange.start(id: 1, positionMs: 0),
        message: 'late',
      ),
      (PlaybackState s) => serverFailed(s, 1),
      (PlaybackState s) => playerLoaded(s, 1),
      (PlaybackState s) =>
          playerReads(s, const PlayerSnapshot(positionMs: 5000)),
      playerBufferReady,
      (PlaybackState s) => stallChanged(s, true),
    ]) {
      expect(run.apply(write), isEmpty);
    }
  });

  test('leaving during a change drops its answer', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final int id = run.sentId;

    expect(run.apply(lifecycleLeaves), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 0),
    ]);
    expect(
      run.apply(
        (PlaybackState s) => serverUpdated(
          s,
          id: id,
          negotiation: kRenegotiated,
          seeksInStream: true,
        ),
      ),
      isEmpty,
    );
  });

  test('the heartbeat reports where the viewer is and whether it plays', () {
    expect(PlaybackRun.opened().apply(heartbeatDue), isEmpty);

    final PlaybackRun run = PlaybackRun.watching(atMs: 90000);
    expect(run.apply(heartbeatDue), const <PlaybackCommand>[
      SendProgress(sessionId: 's1', positionMs: 90000, playing: true),
    ]);

    run.apply((PlaybackState s) => viewerChangesRemote(s, k720));
    expect(run.apply(heartbeatDue), const <PlaybackCommand>[
      SendProgress(sessionId: 's1', positionMs: 90000, playing: false),
    ], reason: 'the player is paused while a change is in flight');
  });

  test('slow marks only the change it was armed for', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final int id = run.sentId;

    run.apply((PlaybackState s) => changeSlow(s, id - 1));
    expect(run.state.status.slow, isFalse);
    run.apply((PlaybackState s) => changeSlow(s, id));
    expect(run.state.status.slow, isTrue);
    run.apply((PlaybackState s) => serverFailed(s, id));
    expect(run.state.status.slow, isFalse);
  });

  PlaybackStep timesOut(PlaybackState s) =>
      changeTimedOut(s, s.status.sent!, message: 'timeout');

  test('a start or a load that takes too long fails with Retry', () {
    final PlaybackRun start = PlaybackRun.opened()..apply(timesOut);
    expect(start.state.status.error, 'timeout');
    expect(start.state.status.sent, isNull);

    final PlaybackRun load = PlaybackRun.opened()
      ..apply(
        (PlaybackState s) => serverStarted(
          s,
          id: 1,
          session: kDirect,
          positionMs: 0,
          seeksInStream: true,
        ),
      );
    expect(load.apply(timesOut), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 0),
      UnloadPlayer(),
    ]);
    expect(load.state.status.error, 'timeout');
  });

  test('a settings request that takes too long ends the session with Retry '
      'and puts the setting back', () {
    final PlaybackRun run = PlaybackRun.watching(atMs: 90000);
    final PlaybackControls before = run.state.remote;
    run.apply((PlaybackState s) => viewerChangesRemote(s, k720));

    expect(run.apply(timesOut), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 90000),
      UnloadPlayer(),
    ]);
    expect(run.state.remote, before);
    expect(run.state.stream, isNull);
    expect(run.state.status.sent, isNull);
    expect(run.state.status.error, 'timeout');
    expect(run.state.status.notice, isNull);
  });

  test('a far seek that takes too long ends the session with Retry where the '
      'viewer was', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode,
      atMs: 90000,
    )..apply((PlaybackState s) => viewerSeeks(s, 3000000));

    expect(run.apply(timesOut), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 90000),
      UnloadPlayer(),
    ]);
    expect(run.state.timeline.positionMs, 90000);
    expect(run.state.status.error, 'timeout');
  });

  test('a timeout for a change that has moved on does nothing', () {
    final PlaybackRun stale = PlaybackRun.watching();
    final PlaybackState watching = stale.state;
    expect(
      stale.apply(
        (PlaybackState s) => changeTimedOut(
          s,
          const SentChange.start(id: 1, positionMs: 0),
          message: 'timeout',
        ),
      ),
      isEmpty,
    );
    expect(stale.state, watching);

    final PlaybackRun answered = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerChangesRemote(s, k720));
    final SentChange request = answered.state.status.sent!;
    answered.apply(
      (PlaybackState s) => serverUpdated(
        s,
        id: request.id,
        negotiation: kRenegotiated,
        seeksInStream: true,
      ),
    );
    expect(
      answered.apply(
        (PlaybackState s) => changeTimedOut(s, request, message: 'timeout'),
      ),
      isEmpty,
      reason: 'the request timed out as its answer turned it into a load',
    );
    expect(answered.state.status.sent, isA<SentLoad>());
  });

  test('a gather that lands in the background starts nothing until the page '
      'comes back', () {
    final PlaybackRun run = PlaybackRun.created()..apply(lifecycleBackgrounds);

    expect(run.gather(resumeMs: 42000), const <PlaybackCommand>[
      ApplyLocal(LocalSettings()),
    ]);
    expect(run.state.status.life, PlaybackLife.background);
    expect(run.state.status.sent, isNull);
    expect(run.apply(heartbeatDue), isEmpty);
    expect(
      run.apply((PlaybackState s) => lifecycleForegrounds(s, playing: true)),
      <PlaybackCommand>[
        SendStart(id: 1, controls: run.state.remote, positionMs: 42000),
      ],
    );
  });

  test('coming back before the gather opens the page and waits for it', () {
    final PlaybackRun run = PlaybackRun.created()..apply(lifecycleBackgrounds);

    expect(
      run.apply((PlaybackState s) => lifecycleForegrounds(s, playing: false)),
      isEmpty,
    );
    expect(run.state.status.life, PlaybackLife.open);
    expect(run.state.timeline.playing, isFalse);
    expect(run.gather(resumeMs: 42000).last, isA<SendStart>());
  });

  test('coming back clears an error from before', () {
    final PlaybackRun run = PlaybackRun.opened(resumeMs: 42000)
      ..apply((PlaybackState s) => serverRefusedStart(s, 1, 'refused'))
      ..apply(lifecycleBackgrounds)
      ..apply((PlaybackState s) => lifecycleForegrounds(s, playing: true));

    expect(run.state.status.error, isNull);
    expect(
      run.state.status.sent,
      const SentChange.start(id: 2, positionMs: 42000),
    );
  });

  test('the heartbeat and the end report paused while the buffer holds', () {
    final PlaybackRun run = PlaybackRun.watching(
      atMs: 90000,
      local: const LocalSettings(waitForBuffer: true),
    );
    expect(run.state.status.holding, isTrue);

    expect(run.apply(heartbeatDue), const <PlaybackCommand>[
      SendProgress(sessionId: 's1', positionMs: 90000, playing: false),
    ]);
    expect(run.apply(lifecycleLeaves), const <PlaybackCommand>[
      EndSession(sessionId: 's1', positionMs: 90000),
    ]);
  });

  test('a seek settles only on its own target', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerSeeks(s, 300000))
      ..apply((PlaybackState s) => seekSettled(s, 200000));
    expect(run.state.timeline.seekingToMs, 300000);

    run.apply((PlaybackState s) => seekSettled(s, 300000));
    expect(run.state.timeline.seekingToMs, isNull);
    expect(run.state.timeline.positionMs, 300000);
  });

  test('a stall clears without dropping the error, which only playback '
      'moving past it clears', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'))
      ..apply((PlaybackState s) => stallChanged(s, true));
    expect(run.state.status.stalled, isTrue);
    expect(run.state.status.error, 'decoder');

    run.apply((PlaybackState s) => stallChanged(s, false));
    expect(run.state.status.stalled, isFalse);
    expect(run.state.status.error, 'decoder');
  });
}
