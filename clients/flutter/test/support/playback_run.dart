import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_mode.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/player/actors/lifecycle.dart';
import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';

const int kTitleMs = 6313000;

const VersionDetail kVersion = VersionDetail(
  id: 'v1',
  title: TitleRef(type: TitleKind.movie, id: 't1'),
  libraryId: 'lib',
  quality: Quality.hd,
  container: 'mkv',
  durationMs: kTitleMs,
);

const PlaybackSession kDirect = PlaybackSession(
  sessionId: 's1',
  manifestUrl: '/direct.m3u8',
);

const PlaybackSession kTranscode = PlaybackSession(
  sessionId: 's1',
  mode: PlaybackMode.transcode,
  container: 'fmp4',
  manifestUrl: '/transcode.m3u8',
  sequential: true,
);

const Negotiation kRenegotiated = Negotiation(
  manifestUrl: '/next.m3u8',
  mode: PlaybackMode.transcode,
  container: 'fmp4',
);

class PlaybackRun {
  PlaybackRun(PlaybackStep opened) : state = opened.$1, commands = opened.$2;

  factory PlaybackRun.created({
    LocalSettings local = const LocalSettings(),
    PlaybackControls controls = const PlaybackControls(),
    bool playing = true,
    VersionDetail version = kVersion,
    bool mixesSound = false,
  }) => PlaybackRun(
    lifecycleOpens(
      versionId: version.id,
      local: local,
      controls: controls,
      playing: playing,
      mixesSound: mixesSound,
    ),
  );

  factory PlaybackRun.opened({
    LocalSettings local = const LocalSettings(),
    PlaybackControls controls = const PlaybackControls(),
    int resumeMs = 0,
    bool playing = true,
    VersionDetail version = kVersion,
    bool mixesSound = false,
  }) => PlaybackRun.created(
    controls: controls,
    playing: playing,
    version: version,
    mixesSound: mixesSound,
  )..gather(local: local, resumeMs: resumeMs, version: version);

  List<PlaybackCommand> gather({
    LocalSettings local = const LocalSettings(),
    int resumeMs = 0,
    VersionDetail version = kVersion,
  }) => apply(
    (PlaybackState s) => lifecycleGathered(
      s,
      title: PlaybackTitle(version: version),
      local: local,
      resumeMs: resumeMs,
    ),
  );

  factory PlaybackRun.watching({
    PlaybackSession session = kDirect,
    bool seeksInStream = true,
    int atMs = 0,
    bool playing = true,
    LocalSettings local = const LocalSettings(),
    PlaybackControls controls = const PlaybackControls(),
    VersionDetail version = kVersion,
    bool mixesSound = false,
  }) {
    final PlaybackRun run = PlaybackRun.opened(
      local: local,
      controls: controls,
      resumeMs: atMs,
      playing: playing,
      version: version,
      mixesSound: mixesSound,
    );
    final int id = run.state.status.sent!.id;
    run
      ..apply(
        (PlaybackState s) => serverStarted(
          s,
          id: id,
          session: session,
          positionMs: atMs,
          seeksInStream: seeksInStream,
        ),
      )
      ..apply((PlaybackState s) => playerLoaded(s, id))
      ..apply((PlaybackState s) => seekSettled(s, atMs));
    return run;
  }

  PlaybackState state;
  List<PlaybackCommand> commands;

  int get sentId => state.status.sent!.id;

  List<PlaybackCommand> apply(PlaybackWrite write) {
    final PlaybackState before = state;
    final (PlaybackState after, List<PlaybackCommand> out) = write(before);
    expectInvariants(before, after, out);
    state = after;
    commands = out;
    return out;
  }
}

void expectInvariants(
  PlaybackState before,
  PlaybackState after,
  List<PlaybackCommand> commands,
) {
  final int? sentId = after.status.sent?.id;
  for (final PlaybackCommand command in commands) {
    switch (command) {
      case SendStart(:final int id) ||
          SendUpdate(:final int id) ||
          SendSeek(:final int id) ||
          LoadStream(:final int id):
        expect(id, sentId, reason: 'a request carries the change in flight');
      case PlayPlayer():
        expect(
          after.status.sent,
          isNull,
          reason: 'nothing plays while a change is in flight',
        );
      default:
        break;
    }
    if (command is SendUpdate ||
        command is SendSeek ||
        command is SendProgress) {
      expect(after.stream, isNotNull, reason: 'only a session gets requests');
    }
    if (command is SendUpdate || command is SendSeek) {
      expect(
        commands,
        contains(const PausePlayer()),
        reason: 'the player is paused while the server changes the stream',
      );
    }
    if (command is SendStart) {
      expect(
        after.status.gathered,
        isTrue,
        reason: 'a start needs what the page gathered',
      );
    }
    if (command is TuneSound) {
      expect(
        (command.normalize, command.dialogue),
        (
          after.remote.normalize ?? true,
          after.remote.dialogue ?? DialogueLift.none,
        ),
        reason: 'the player is tuned to the settings in the state',
      );
    }
  }
  if (after.watching && after.reportsPlaying != before.reportsPlaying) {
    expect(
      commands.whereType<SendProgress>().map((SendProgress p) => p.playing),
      <bool>[after.reportsPlaying],
      reason: 'the server hears at once when playing starts or stops',
    );
  }
  expect(
    after.status.lastChangeId,
    greaterThanOrEqualTo(before.status.lastChangeId),
    reason: 'change ids never go back',
  );
  if (before.status.life == PlaybackLife.closed) {
    expect(
      commands.every((PlaybackCommand c) => c is EndSession),
      isTrue,
      reason: 'after leaving only a late start is ended',
    );
  }
  if (after.status.life == PlaybackLife.closed) {
    expect(after.stream, isNull);
    expect(after.status.sent, isNull);
  }
  if (after.status.sent != null) {
    expect(after.status.life, PlaybackLife.open);
    expect(
      after.status.notice,
      isNull,
      reason: 'a notice is about the last change, not the one in flight',
    );
  }
  if (after.status.slow) {
    expect(
      after.status.sent,
      isNotNull,
      reason: 'only a change in flight can be slow',
    );
  }
  if (after.status.stalled) {
    expect(
      after.watching,
      isTrue,
      reason: 'only a stream being watched stalls',
    );
  }
  if (after.status.holding) {
    expect(after.stream, isNotNull, reason: 'only a stream waits to buffer');
  }
  final bool seeks = commands.any(
    (PlaybackCommand c) => c is SeekPlayer || c is SendSeek,
  );
  if (seeks || after.status.sent is SentLoad) {
    expect(
      after.timeline.ended,
      isFalse,
      reason: 'a seek or a new stream is not at its end',
    );
  }
}
