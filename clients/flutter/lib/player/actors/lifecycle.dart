import 'package:shadowmask/player/actors/player.dart';
import 'package:shadowmask/player/actors/server.dart';
import 'package:shadowmask/player/actors/session_end.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/playback_timeline.dart';
import 'package:shadowmask/player/state/playback_title.dart';
import 'package:shadowmask/player/state/sent_change.dart';
import 'package:shadowmask/view/playback_controls.dart';

PlaybackStep _same(PlaybackState s) => (s, const <PlaybackCommand>[]);

PlaybackStep lifecycleOpens({
  required String versionId,
  required LocalSettings local,
  required PlaybackControls controls,
  required bool playing,
}) => _same(
  PlaybackState(
    versionId: versionId,
    local: local,
    remote: controls,
    timeline: PlaybackTimeline(playing: playing),
  ),
);

PlaybackStep lifecycleGathered(
  PlaybackState s, {
  required PlaybackTitle title,
  required LocalSettings local,
  required int resumeMs,
}) {
  if (s.status.life == PlaybackLife.closed || s.status.gathered) {
    return _same(s);
  }
  final LocalSettings kept = local.copyWith(
    fullscreen: s.local.fullscreen,
    pictureInPicture: s.local.pictureInPicture,
    mutedByPolicy: s.local.mutedByPolicy,
  );
  final PlaybackState gathered = s.copyWith(
    title: title,
    local: kept,
    remote: s.remote.withDefaultDelivery(kept.defaultDelivery),
    timeline: s.timeline.copyWith(positionMs: resumeMs),
    status: s.status.copyWith(gathered: true),
  );
  if (gathered.status.life != PlaybackLife.open) {
    return (gathered, <PlaybackCommand>[ApplyLocal(kept)]);
  }
  final int id = gathered.status.lastChangeId + 1;
  return (
    gathered.copyWith.status(
      lastChangeId: id,
      sent: SentChange.start(id: id, positionMs: resumeMs),
    ),
    <PlaybackCommand>[
      ApplyLocal(kept),
      SendStart(id: id, controls: gathered.remote, positionMs: resumeMs),
    ],
  );
}

PlaybackStep lifecycleBackgrounds(PlaybackState s) {
  if (s.status.life != PlaybackLife.open) {
    return _same(s);
  }
  final PlaybackStream? stream = s.stream;
  return (
    s.copyWith(
      stream: null,
      timeline: s.timeline.copyWith(seekingToMs: null),
      status: s.status.copyWith(
        life: PlaybackLife.background,
        sent: null,
        slow: false,
        stalled: false,
        holding: false,
      ),
    ),
    <PlaybackCommand>[
      const PausePlayer(),
      if (stream != null) endOf(s, stream),
      const UnloadPlayer(),
    ],
  );
}

PlaybackStep lifecycleForegrounds(PlaybackState s, {required bool playing}) {
  if (s.status.life != PlaybackLife.background) {
    return _same(s);
  }
  if (!s.status.gathered) {
    return _same(
      s.copyWith(
        timeline: s.timeline.copyWith(playing: playing),
        status: s.status.copyWith(life: PlaybackLife.open),
      ),
    );
  }
  final int id = s.status.lastChangeId + 1;
  final int positionMs = s.timeline.positionMs;
  return (
    s.copyWith(
      timeline: s.timeline.copyWith(playing: playing),
      status: s.status.copyWith(
        life: PlaybackLife.open,
        lastChangeId: id,
        notice: null,
        error: null,
        sent: SentChange.start(id: id, positionMs: positionMs),
      ),
    ),
    <PlaybackCommand>[
      SendStart(id: id, controls: s.remote, positionMs: positionMs),
    ],
  );
}

PlaybackStep lifecycleLeaves(PlaybackState s) {
  if (s.status.life == PlaybackLife.closed) {
    return _same(s);
  }
  final PlaybackStream? stream = s.stream;
  return (
    s.copyWith(
      stream: null,
      status: s.status.copyWith(
        life: PlaybackLife.closed,
        sent: null,
        slow: false,
        stalled: false,
        holding: false,
      ),
    ),
    <PlaybackCommand>[if (stream != null) endOf(s, stream)],
  );
}

PlaybackStep heartbeatDue(PlaybackState s) {
  final PlaybackStream? stream = s.stream;
  if (s.status.life != PlaybackLife.open || stream == null) {
    return _same(s);
  }
  return (
    s,
    <PlaybackCommand>[
      SendProgress(
        sessionId: stream.sessionId,
        positionMs: s.timeline.positionMs,
        playing: s.reportsPlaying,
      ),
    ],
  );
}

PlaybackStep changeSlow(PlaybackState s, int id) =>
    s.status.life == PlaybackLife.open && s.status.sent?.id == id
    ? _same(s.copyWith.status(slow: true))
    : _same(s);

PlaybackStep changeTimedOut(
  PlaybackState s,
  SentChange sent, {
  required String message,
}) {
  final PlaybackStream? stream = s.stream;
  if (s.status.life != PlaybackLife.open || s.status.sent != sent) {
    return _same(s);
  }
  return switch (sent) {
    SentStart() => serverRefusedStart(s, sent.id, message),
    SentLoad() => playerFailedToLoad(s, sent.id, message),
    SentControls(:final PlaybackControls previous) when stream != null =>
      streamFails(s.copyWith(remote: previous), stream, message),
    SentSeek(:final int previousMs, :final bool previousPlaying)
        when stream != null =>
      streamFails(
        s.copyWith.timeline(positionMs: previousMs, playing: previousPlaying),
        stream,
        message,
      ),
    _ => _same(s),
  };
}

PlaybackStep seekSettled(PlaybackState s, int targetMs) =>
    s.timeline.seekingToMs == targetMs
    ? _same(s.copyWith.timeline(seekingToMs: null))
    : _same(s);

PlaybackStep stallChanged(PlaybackState s, bool stalled) =>
    s.watching ? _same(s.copyWith.status(stalled: stalled)) : _same(s);
