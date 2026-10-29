import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/player/actors/progress.dart';
import 'package:shadowmask/player/actors/session_end.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/player/state/playback_command.dart';
import 'package:shadowmask/player/state/playback_life.dart';
import 'package:shadowmask/player/state/playback_state.dart';
import 'package:shadowmask/player/state/playback_step.dart';
import 'package:shadowmask/player/state/playback_stream.dart';
import 'package:shadowmask/player/state/sent_change.dart';

PlaybackStep _same(PlaybackState s) => (s, const <PlaybackCommand>[]);

bool _loading(PlaybackState s) =>
    s.status.life == PlaybackLife.open && s.status.sent is SentLoad;

bool _loadingId(PlaybackState s, int id) =>
    _loading(s) && s.status.sent?.id == id;

PlaybackStep playerReads(PlaybackState s, PlayerSnapshot reading) {
  final PlaybackStream? stream = s.stream;
  if (!s.watching || stream == null) {
    return _same(s);
  }
  final int atMs = stream.originMs + reading.positionMs;
  final int? seekingToMs = s.timeline.seekingToMs;
  final bool settled =
      seekingToMs == null ||
      (atMs - seekingToMs).abs() <= kSeekSettleMs ||
      (reading.ended && atMs < seekingToMs);
  final bool playing = s.status.holding ? s.timeline.playing : reading.playing;
  final VersionDetail? version = s.title.version;
  final bool fillsDuration =
      version != null && version.durationMs <= 0 && reading.durationMs > 0;
  return withProgress(s, (
    s.copyWith(
      title: fillsDuration
          ? s.title.copyWith(
              version: version.copyWith(
                durationMs: stream.originMs + reading.durationMs,
              ),
            )
          : s.title,
      timeline: s.timeline.copyWith(
        positionMs: settled ? atMs : s.timeline.positionMs,
        seekingToMs: settled ? null : seekingToMs,
        reachMs: settled ? atMs + reading.bufferedAheadMs : s.timeline.reachMs,
        bufferedAheadMs: reading.bufferedAheadMs,
        playing: playing,
        ended: settled ? reading.ended : s.timeline.ended,
        waiting: reading.waiting,
        bufferingPercent: reading.bufferingPercent,
      ),
    ),
    const <PlaybackCommand>[],
  ));
}

PlaybackStep playerLoaded(PlaybackState s, int id) {
  if (!_loadingId(s, id)) {
    return _same(s);
  }
  return withProgress(s, (
    s.copyWith.status(sent: null, slow: false),
    <PlaybackCommand>[
      if (s.timeline.playing && !s.status.holding) const PlayPlayer(),
    ],
  ));
}

PlaybackStep playerFailedToLoad(PlaybackState s, int id, String message) {
  final PlaybackStream? stream = s.stream;
  if (!_loadingId(s, id) || stream == null) {
    return _same(s);
  }
  return streamFails(s, stream, message);
}

PlaybackStep playerLostStream(PlaybackState s, String message) {
  final PlaybackStream? stream = s.stream;
  if (!(s.watching || _loading(s)) || stream == null) {
    return _same(s);
  }
  return streamFails(s, stream, message);
}

PlaybackStep playerErrs(PlaybackState s, String message) =>
    s.watching || _loading(s)
    ? _same(s.copyWith.status(error: message))
    : _same(s);

PlaybackStep playerMovedPastError(PlaybackState s) =>
    s.watching ? _same(s.copyWith.status(error: null)) : _same(s);

PlaybackStep playerBufferReady(PlaybackState s) {
  if (!s.watching || !s.status.holding) {
    return _same(s);
  }
  return withProgress(s, (
    s.copyWith.status(holding: false),
    <PlaybackCommand>[if (s.timeline.playing) const PlayPlayer()],
  ));
}

PlaybackStep playerReportsLocal(PlaybackState s, LocalSettings local) =>
    s.status.life == PlaybackLife.closed
    ? _same(s)
    : _same(s.copyWith(local: local));
