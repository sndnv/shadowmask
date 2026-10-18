import 'package:flutter_test/flutter_test.dart';
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
import 'package:shadowmask/view/playback_controls.dart';

import '../../support/playback_run.dart';

PlaybackWrite _reads(PlayerSnapshot reading) =>
    (PlaybackState s) => playerReads(s, reading);

PlaybackRun _loading({
  LocalSettings local = const LocalSettings(),
  bool playing = true,
  int resumeMs = 0,
}) {
  final PlaybackRun run = PlaybackRun.opened(
    local: local,
    playing: playing,
    resumeMs: resumeMs,
  );
  run.apply(
    (PlaybackState s) => serverStarted(
      s,
      id: 1,
      session: kDirect,
      positionMs: resumeMs,
      seeksInStream: true,
    ),
  );
  return run;
}

void main() {
  test('readings write the title position, the buffer and waiting', () {
    final PlaybackRun run = PlaybackRun.watching(
      session: kTranscode.copyWith(originMs: 120000),
      atMs: 120000,
    );

    expect(
      run.apply(
        _reads(
          const PlayerSnapshot(
            positionMs: 5000,
            bufferedAheadMs: 8000,
            playing: true,
            ready: true,
            buffering: true,
            bufferingPercent: 40,
          ),
        ),
      ),
      isEmpty,
    );
    expect(run.state.timeline.positionMs, 125000);
    expect(run.state.timeline.bufferedAheadMs, 8000);
    expect(run.state.timeline.waiting, isTrue);
    expect(run.state.timeline.bufferingPercent, 40);
    expect(run.state.timeline.playing, isTrue);
  });

  test('a change between playing and paused is reported at once', () {
    final PlaybackRun run = PlaybackRun.watching();
    const PlayerSnapshot paused = PlayerSnapshot(positionMs: 3000, ready: true);

    expect(run.apply(_reads(paused)), const <PlaybackCommand>[
      SendProgress(sessionId: 's1', positionMs: 3000, playing: false),
    ]);
    expect(run.apply(_reads(paused)), isEmpty);
  });

  test('the end of the title is read from the player', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply(_reads(const PlayerSnapshot(positionMs: kTitleMs, ended: true)));

    expect(run.state.timeline.ended, isTrue);
    expect(run.state.timeline.playing, isFalse);
  });

  test('readings are ignored during a change and after leaving', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply(
        (PlaybackState s) =>
            viewerChangesRemote(s, const PlaybackControls(height: 720)),
      );
    final PlaybackState changing = run.state;

    expect(
      run.apply(_reads(const PlayerSnapshot(positionMs: 999999))),
      isEmpty,
    );
    expect(run.state, changing);

    final PlaybackRun left = PlaybackRun.watching()..apply(lifecycleLeaves);
    final PlaybackState closed = left.state;
    expect(left.apply(_reads(const PlayerSnapshot(positionMs: 5000))), isEmpty);
    expect(left.state, closed);
  });

  test('after a local seek the position stays on the target until a reading '
      'reaches it', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerSeeks(s, 300000))
      ..apply(_reads(const PlayerSnapshot(positionMs: 10000, playing: true)));

    expect(run.state.timeline.positionMs, 300000);
    expect(run.state.timeline.seekingToMs, 300000);

    run.apply(_reads(const PlayerSnapshot(positionMs: 300400, playing: true)));

    expect(run.state.timeline.positionMs, 300400);
    expect(run.state.timeline.seekingToMs, isNull);
  });

  test('a reading at the edge of the seek window settles the seek', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => viewerSeeks(s, 300000))
      ..apply(
        _reads(
          const PlayerSnapshot(
            positionMs: 300000 + kSeekSettleMs,
            playing: true,
          ),
        ),
      );

    expect(run.state.timeline.positionMs, 300000 + kSeekSettleMs);
    expect(run.state.timeline.seekingToMs, isNull);
  });

  test('an end the player reaches short of a held seek target settles it as '
      'ended', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply(_reads(const PlayerSnapshot(positionMs: 6290000, playing: true)))
      ..apply((PlaybackState s) => viewerSeeks(s, kTitleMs))
      ..apply(_reads(const PlayerSnapshot(positionMs: 6305000, ended: true)));

    expect(run.state.timeline.ended, isTrue);
    expect(run.state.timeline.seekingToMs, isNull);
    expect(run.state.timeline.positionMs, 6305000);
  });

  test('an end read at the old position after a seek back from the end is '
      'ignored', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply(_reads(const PlayerSnapshot(positionMs: kTitleMs, ended: true)))
      ..apply((PlaybackState s) => viewerSeeks(s, 600000))
      ..apply(_reads(const PlayerSnapshot(positionMs: kTitleMs, ended: true)));

    expect(run.state.timeline.ended, isFalse);
    expect(run.state.timeline.seekingToMs, 600000);
    expect(run.state.timeline.positionMs, 600000);
  });

  test(
    'loaded completes the change, plays if playing and tells the server',
    () {
      final PlaybackRun run = _loading(resumeMs: 5000);

      expect(
        run.apply((PlaybackState s) => playerLoaded(s, 1)),
        const <PlaybackCommand>[
          PlayPlayer(),
          SendProgress(sessionId: 's1', positionMs: 5000, playing: true),
        ],
      );
      expect(run.state.status.sent, isNull);
    },
  );

  test('an error during a load is kept for the new stream, and a new stream '
      'forgets the errors before it', () {
    final PlaybackRun run = _loading()
      ..apply((PlaybackState s) => playerErrs(s, 'tcp: refused'))
      ..apply((PlaybackState s) => playerLoaded(s, 1));
    expect(run.state.status.error, 'tcp: refused');

    run
      ..apply(
        (PlaybackState s) =>
            viewerChangesRemote(s, const PlaybackControls(height: 720)),
      )
      ..apply(
        (PlaybackState s) => serverUpdated(
          s,
          id: s.status.sent!.id,
          negotiation: kRenegotiated,
          seeksInStream: true,
        ),
      );
    expect(run.state.status.error, isNull);
  });

  test('loaded leaves a paused or waiting video paused', () {
    expect(
      _loading(playing: false).apply((PlaybackState s) => playerLoaded(s, 1)),
      isEmpty,
    );
    expect(
      _loading(
        local: const LocalSettings(waitForBuffer: true),
      ).apply((PlaybackState s) => playerLoaded(s, 1)),
      isEmpty,
    );
  });

  test('loaded for another change is ignored', () {
    final PlaybackRun run = _loading();
    final PlaybackState before = run.state;

    expect(run.apply((PlaybackState s) => playerLoaded(s, 2)), isEmpty);
    expect(run.state, before);
  });

  test('a stream that fails to load ends its session, unloads and shows the '
      'error', () {
    final PlaybackRun run = _loading(resumeMs: 5000);

    expect(
      run.apply((PlaybackState s) => playerFailedToLoad(s, 1, 'cannot open')),
      const <PlaybackCommand>[
        EndSession(sessionId: 's1', positionMs: 5000),
        UnloadPlayer(),
      ],
    );
    expect(run.state.stream, isNull);
    expect(run.state.status.error, 'cannot open');
    expect(run.state.status.sent, isNull);
    expect(run.apply(viewerRetries).single, isA<SendStart>());
  });

  test('the buffer being ready ends the wait and plays', () {
    final PlaybackRun run = PlaybackRun.watching(
      local: const LocalSettings(waitForBuffer: true),
    );

    expect(run.apply(playerBufferReady), const <PlaybackCommand>[
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: true),
    ]);
    expect(run.state.status.holding, isFalse);
    expect(run.apply(playerBufferReady), isEmpty);
  });

  test('the buffer being ready after a pause keeps the video paused', () {
    final PlaybackRun run = PlaybackRun.watching(
      local: const LocalSettings(waitForBuffer: true),
    )..apply(viewerPauses);

    expect(run.apply(playerBufferReady), isEmpty);
  });

  test('the player fills in a duration the server did not give', () {
    final PlaybackRun missing = PlaybackRun.watching(
      version: kVersion.copyWith(durationMs: 0),
    )..apply(_reads(const PlayerSnapshot(durationMs: 7000000)));
    expect(missing.state.title.version?.durationMs, 7000000);

    final PlaybackRun given = PlaybackRun.watching()
      ..apply(_reads(const PlayerSnapshot(durationMs: 7000000)));
    expect(given.state.title.version?.durationMs, kTitleMs);

    final PlaybackRun later = PlaybackRun.watching(
      version: kVersion.copyWith(durationMs: 0),
      session: kTranscode.copyWith(originMs: 600000),
      atMs: 600000,
    )..apply(_reads(const PlayerSnapshot()));
    expect(
      later.state.title.version?.durationMs,
      0,
      reason: 'a player that has no duration yet says nothing about the title',
    );
  });

  test('during a change the new stream waits for its own buffer', () {
    final PlaybackRun run =
        PlaybackRun.watching(local: const LocalSettings(waitForBuffer: true))
          ..apply(
            (PlaybackState s) =>
                viewerChangesRemote(s, const PlaybackControls(height: 720)),
          );
    final int id = run.sentId;

    expect(run.apply(playerBufferReady), isEmpty);
    expect(run.state.status.holding, isTrue);
    run.apply(
      (PlaybackState s) => serverUpdated(
        s,
        id: id,
        negotiation: kRenegotiated,
        seeksInStream: true,
      ),
    );
    expect(run.apply((PlaybackState s) => playerLoaded(s, id)), isEmpty);
    expect(run.apply(playerBufferReady), const <PlaybackCommand>[
      PlayPlayer(),
      SendProgress(sessionId: 's1', positionMs: 0, playing: true),
    ]);
  });

  test('a player error is kept for the stall view, but not one from a stream '
      'being replaced', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'));
    expect(run.state.status.error, 'decoder');
    expect(run.state.stream, isNotNull);

    final PlaybackRun changing = PlaybackRun.watching()
      ..apply(
        (PlaybackState s) =>
            viewerChangesRemote(s, const PlaybackControls(height: 720)),
      )
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'));
    expect(changing.state.status.error, isNull);
  });

  test('playback moving past an error clears it, but not during a change', () {
    final PlaybackRun run = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'))
      ..apply(playerMovedPastError);
    expect(run.state.status.error, isNull);

    final PlaybackRun changing = PlaybackRun.watching()
      ..apply((PlaybackState s) => playerErrs(s, 'decoder'))
      ..apply(
        (PlaybackState s) =>
            viewerChangesRemote(s, const PlaybackControls(height: 720)),
      )
      ..apply(playerMovedPastError);
    expect(changing.state.status.error, 'decoder');
  });

  test('settings the player changes itself are taken without a command', () {
    final PlaybackRun run = PlaybackRun.watching();
    final LocalSettings reported = run.state.local.copyWith(
      volume: 0.3,
      fullscreen: false,
      mutedByPolicy: true,
    );

    expect(
      run.apply((PlaybackState s) => playerReportsLocal(s, reported)),
      isEmpty,
    );
    expect(run.state.local, reported);
  });
}
