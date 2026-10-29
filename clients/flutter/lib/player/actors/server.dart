import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/player/actors/progress.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_notice.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/player_tracks.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/episode_neighbours.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';

PlaybackStep _same(PlaybackState s) => (s, const <PlaybackCommand>[]);

bool _awaits(PlaybackState s, int id) =>
    s.status.life == PlaybackLife.open && s.status.sent?.id == id;

PlaybackStep _load(PlaybackState s, PlaybackStream stream, int id, int atMs) {
  final int offsetMs = stream.sequential && !stream.seeksInStream
      ? 0
      : atMs - stream.originMs;
  return (
    s.copyWith(
      stream: stream,
      timeline: s.timeline.copyWith(
        positionMs: atMs,
        seekingToMs: atMs,
        reachMs: atMs,
        ended: false,
      ),
      status: s.status.copyWith(
        sent: SentChange.load(id: id, positionMs: atMs),
        error: null,
      ),
    ),
    <PlaybackCommand>[
      LoadStream(
        id: id,
        stream: stream,
        offsetMs: offsetMs,
        tracks: playerTracks(stream.selected, s.title.version),
      ),
    ],
  );
}

PlaybackStream _replaced(
  PlaybackStream stream,
  Negotiation negotiation,
  bool seeksInStream,
) => stream.copyWith(
  manifestUrl: negotiation.manifestUrl,
  originMs: negotiation.originMs,
  sequential: negotiation.sequential,
  mode: negotiation.mode,
  container: negotiation.container,
  selected: negotiation.selected ?? stream.selected,
  audioChannels: negotiation.audioChannels ?? stream.audioChannels,
  seeksInStream: seeksInStream,
);

PlaybackStep _retuned(
  PlaybackControls was,
  PlaybackControls back,
  PlaybackStep step,
) => was.sameTuningAs(back)
    ? step
    : (
        step.$1,
        <PlaybackCommand>[
          TuneSound(
            normalize: back.normalize ?? true,
            dialogue: back.dialogue ?? DialogueLift.none,
          ),
          ...step.$2,
        ],
      );

PlaybackStep _resume(PlaybackState before, PlaybackState s) =>
    withProgress(before, (
      s,
      <PlaybackCommand>[
        if (s.timeline.playing && !s.status.holding) const PlayPlayer(),
      ],
    ));

PlaybackStep serverStarted(
  PlaybackState s, {
  required int id,
  required PlaybackSession session,
  required int positionMs,
  required bool seeksInStream,
}) {
  if (!_awaits(s, id) || s.status.sent is! SentStart) {
    return (s, <PlaybackCommand>[EndSession(sessionId: session.sessionId)]);
  }
  final PlaybackStream stream = PlaybackStream(
    sessionId: session.sessionId,
    manifestUrl: session.manifestUrl,
    originMs: session.originMs,
    sequential: session.sequential,
    mode: session.mode,
    container: session.container,
    selected: session.selected,
    audioChannels: session.audioChannels,
    heartbeatIntervalS: session.heartbeatIntervalS,
    seeksInStream: seeksInStream,
    trickplay: session.trickplay,
  );
  return _load(
    s.copyWith(
      remote: s.remote.withSelection(session.selected),
      status: s.status.copyWith(
        holding: s.local.waitForBuffer && s.timeline.playing,
      ),
    ),
    stream,
    id,
    positionMs,
  );
}

PlaybackStep serverRefusedStart(PlaybackState s, int id, String message) {
  if (!_awaits(s, id) || s.status.sent is! SentStart) {
    return _same(s);
  }
  return _same(s.copyWith.status(sent: null, slow: false, error: message));
}

PlaybackStep serverUpdated(
  PlaybackState s, {
  required int id,
  required Negotiation negotiation,
  required bool seeksInStream,
}) {
  final PlaybackStream? stream = s.stream;
  final SentChange? sent = s.status.sent;
  if (!_awaits(s, id) || sent is! SentControls || stream == null) {
    return _same(s);
  }
  return _load(
    s,
    _replaced(stream, negotiation, seeksInStream),
    id,
    sent.positionMs,
  );
}

PlaybackStep serverSeeked(
  PlaybackState s, {
  required int id,
  required Negotiation negotiation,
  required bool seeksInStream,
}) {
  final PlaybackStream? stream = s.stream;
  final SentChange? sent = s.status.sent;
  if (!_awaits(s, id) || sent is! SentSeek || stream == null) {
    return _same(s);
  }
  return _load(
    s,
    _replaced(stream, negotiation, seeksInStream),
    id,
    sent.positionMs,
  );
}

PlaybackStep serverFailed(PlaybackState s, int id) {
  if (!_awaits(s, id)) {
    return _same(s);
  }
  return switch (s.status.sent) {
    SentControls(:final PlaybackControls previous) => _retuned(
      s.remote,
      previous,
      _resume(
        s,
        s.copyWith(
          remote: previous,
          status: s.status.copyWith(
            sent: null,
            slow: false,
            notice: PlaybackNotice.changeFailed,
          ),
        ),
      ),
    ),
    SentSeek(:final int previousMs, :final bool previousPlaying) => _resume(
      s,
      s.copyWith(
        timeline: s.timeline.copyWith(
          positionMs: previousMs,
          playing: previousPlaying,
        ),
        status: s.status.copyWith(
          sent: null,
          slow: false,
          notice: PlaybackNotice.seekFailed,
        ),
      ),
    ),
    _ => _same(s),
  };
}

PlaybackStep serverDescribesTitle(
  PlaybackState s, {
  required String? name,
  required EpisodeLink? previous,
  required EpisodeLink? next,
}) => _same(s.copyWith.title(name: name, previous: previous, next: next));

PlaybackStep serverRefreshesVersion(PlaybackState s, VersionDetail version) {
  final int knownMs = s.title.version?.durationMs ?? 0;
  return _same(
    s.copyWith.title(
      version: version.durationMs > 0
          ? version
          : version.copyWith(durationMs: knownMs),
    ),
  );
}
