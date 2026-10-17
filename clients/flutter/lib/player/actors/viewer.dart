import 'package:shadowmask/player/actors/progress.dart';
import 'package:shadowmask/player/actors/session_end.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_status.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/playback_controls.dart';

PlaybackStep _same(PlaybackState s) => (s, const <PlaybackCommand>[]);

bool _reachable(PlaybackState s, PlaybackStream stream, int targetMs) {
  if (!stream.sequential) {
    return true;
  }
  if (!stream.seeksInStream || targetMs < stream.originMs) {
    return false;
  }
  return targetMs <= s.timeline.reachMs + kSeekReachMs;
}

PlaybackStatus _changing(PlaybackState s, SentChange sent) => s.status.copyWith(
  lastChangeId: sent.id,
  notice: null,
  stalled: false,
  sent: sent,
);

PlaybackStep viewerChangesLocal(PlaybackState s, LocalSettings local) {
  if (s.status.life == PlaybackLife.closed) {
    return _same(s);
  }
  return (
    s.copyWith(local: local),
    <PlaybackCommand>[ApplyLocal(local), SaveLocal(local)],
  );
}

PlaybackStep viewerSeeks(PlaybackState s, int positionMs) {
  final PlaybackStream? stream = s.stream;
  if (!s.watching || stream == null) {
    return _same(s);
  }
  final int durationMs = _lengthOf(s);
  final int target = durationMs > 0
      ? positionMs.clamp(0, durationMs)
      : (positionMs < 0 ? 0 : positionMs);
  if (_reachable(s, stream, target)) {
    return (
      s.copyWith.timeline(
        positionMs: target,
        seekingToMs: target,
        ended: false,
      ),
      <PlaybackCommand>[SeekPlayer(offsetMs: target - stream.originMs)],
    );
  }
  final SentChange sent = SentChange.seek(
    id: s.status.lastChangeId + 1,
    positionMs: target,
    previousMs: s.timeline.positionMs,
    previousPlaying: s.timeline.playing,
  );
  return (
    s.copyWith(
      timeline: s.timeline.copyWith(
        positionMs: target,
        seekingToMs: null,
        ended: false,
      ),
      status: _changing(s, sent),
    ),
    <PlaybackCommand>[
      const PausePlayer(),
      SendSeek(id: sent.id, sessionId: stream.sessionId, positionMs: target),
    ],
  );
}

int _lengthOf(PlaybackState s) => s.title.version?.durationMs ?? 0;

PlaybackStep viewerSeeksBy(PlaybackState s, int deltaMs) => _lengthOf(s) <= 0
    ? _same(s)
    : viewerSeeks(s, s.timeline.positionMs + deltaMs);

PlaybackStep viewerSeeksToFraction(PlaybackState s, double fraction) {
  final int lengthMs = _lengthOf(s);
  return lengthMs <= 0
      ? _same(s)
      : viewerSeeks(s, (lengthMs * fraction).round());
}

PlaybackStep viewerToggles(PlaybackState s) {
  if (s.timeline.ended) {
    return viewerReplays(s);
  }
  return s.timeline.playing ? viewerPauses(s) : viewerPlays(s);
}

PlaybackStep viewerPlays(PlaybackState s) => _play(s, true);

PlaybackStep viewerPauses(PlaybackState s) => _play(s, false);

PlaybackStep _play(PlaybackState s, bool playing) {
  if (!s.watching) {
    return _same(s);
  }
  return withProgress(s, (
    s.copyWith(
      timeline: s.timeline.copyWith(playing: playing),
      status: s.status.copyWith(holding: false),
    ),
    <PlaybackCommand>[playing ? const PlayPlayer() : const PausePlayer()],
  ));
}

PlaybackStep viewerReplays(PlaybackState s) {
  if (!s.watching) {
    return _same(s);
  }
  final (PlaybackState sought, List<PlaybackCommand> seeking) = viewerSeeks(
    s,
    0,
  );
  final PlaybackState after = sought.copyWith(
    timeline: sought.timeline.copyWith(playing: true),
    status: sought.status.copyWith(holding: false),
  );
  if (after.status.sent != null) {
    return (after, seeking);
  }
  return withProgress(s, (
    after,
    <PlaybackCommand>[...seeking, const PlayPlayer()],
  ));
}

PlaybackStep viewerChangesRemote(PlaybackState s, PlaybackControls controls) {
  if (s.status.life == PlaybackLife.closed || s.status.sent != null) {
    return _same(s);
  }
  final PlaybackStream? stream = s.stream;
  if (stream == null ||
      s.status.life != PlaybackLife.open ||
      controls.sameUpdateAs(s.remote)) {
    return (s.copyWith(remote: controls), const <PlaybackCommand>[]);
  }
  final SentChange sent = SentChange.controls(
    id: s.status.lastChangeId + 1,
    controls: controls,
    previous: s.remote,
    positionMs: s.timeline.positionMs,
  );
  return (
    s.copyWith(remote: controls, status: _changing(s, sent)),
    <PlaybackCommand>[
      const PausePlayer(),
      SendUpdate(
        id: sent.id,
        sessionId: stream.sessionId,
        controls: controls,
        positionMs: s.timeline.positionMs,
      ),
    ],
  );
}

PlaybackStep viewerRetries(PlaybackState s) {
  if (s.status.life != PlaybackLife.open ||
      s.status.sent != null ||
      s.status.error == null) {
    return _same(s);
  }
  final PlaybackStream? stream = s.stream;
  final SentChange sent = SentChange.start(
    id: s.status.lastChangeId + 1,
    positionMs: s.timeline.positionMs,
  );
  return (
    s.copyWith(
      stream: null,
      timeline: s.timeline.copyWith(seekingToMs: null),
      status: _changing(s, sent).copyWith(error: null, holding: false),
    ),
    <PlaybackCommand>[
      if (stream != null) endOf(s, stream),
      SendStart(id: sent.id, controls: s.remote, positionMs: sent.positionMs),
    ],
  );
}

PlaybackStep viewerKeepsWaiting(PlaybackState s) => s.watching
    ? _same(s.copyWith.status(stalled: false, error: null))
    : _same(s);
