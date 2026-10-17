import 'package:shadowmask/player/player_snapshot.dart';

const int kStallSeconds = 20;

typedef StallWatch = ({
  int progressMs,
  int positionMs,
  int stuckS,
  int? errorAtMs,
});

typedef StallTick = ({StallWatch watch, bool? stalled, bool movedPastError});

const StallWatch kStallWatchStart = (
  progressMs: -1,
  positionMs: -1,
  stuckS: 0,
  errorAtMs: null,
);

int _progressOf(PlayerSnapshot reading) =>
    reading.positionMs + reading.bufferedAheadMs;

StallWatch stallErred(StallWatch watch, PlayerSnapshot reading) => (
  progressMs: watch.progressMs,
  positionMs: watch.positionMs,
  stuckS: watch.stuckS,
  errorAtMs: _progressOf(reading),
);

StallWatch stallIdle(PlayerSnapshot reading, {required bool errored}) =>
    errored ? stallErred(kStallWatchStart, reading) : kStallWatchStart;

StallTick stallTick(
  StallWatch watch,
  PlayerSnapshot reading, {
  required bool errored,
}) {
  final int progressMs = _progressOf(reading);
  final bool movedPastError = errored && progressMs != watch.errorAtMs;
  final bool erred = errored && !movedPastError;
  if (erred && reading.ended) {
    return (
      watch: (
        progressMs: progressMs,
        positionMs: reading.positionMs,
        stuckS: kStallSeconds,
        errorAtMs: watch.errorAtMs,
      ),
      stalled: true,
      movedPastError: false,
    );
  }
  final bool paused = !reading.playing && !reading.waiting && !reading.ended;
  final bool frozen =
      erred && !paused && reading.positionMs == watch.positionMs;
  if (!(reading.waiting || frozen) || progressMs != watch.progressMs) {
    return (
      watch: (
        progressMs: progressMs,
        positionMs: reading.positionMs,
        stuckS: 0,
        errorAtMs: watch.errorAtMs,
      ),
      stalled: false,
      movedPastError: movedPastError,
    );
  }
  final int stuckS = watch.stuckS + 1;
  return (
    watch: (
      progressMs: progressMs,
      positionMs: reading.positionMs,
      stuckS: stuckS,
      errorAtMs: watch.errorAtMs,
    ),
    stalled: stuckS >= kStallSeconds ? true : null,
    movedPastError: movedPastError,
  );
}
