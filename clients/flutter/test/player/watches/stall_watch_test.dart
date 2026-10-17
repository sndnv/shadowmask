import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/watches/stall_watch.dart';

const PlayerSnapshot kWaiting = PlayerSnapshot(
  durationMs: 100000,
  positionMs: 5000,
  ready: true,
  buffering: true,
  playing: true,
);

const PlayerSnapshot kPlaying = PlayerSnapshot(
  durationMs: 100000,
  positionMs: 5000,
  ready: true,
  playing: true,
);

const PlayerSnapshot kPaused = PlayerSnapshot(
  durationMs: 100000,
  positionMs: 5000,
  ready: true,
);

(StallWatch, List<StallTick>) ticks(
  StallWatch watch,
  PlayerSnapshot reading,
  int count, {
  bool errored = false,
}) {
  final List<StallTick> out = <StallTick>[];
  StallWatch at = watch;
  for (int i = 0; i < count; i++) {
    final StallTick tick = stallTick(at, reading, errored: errored);
    at = tick.watch;
    out.add(tick);
  }
  return (at, out);
}

List<bool?> verdicts(List<StallTick> out) =>
    out.map((StallTick t) => t.stalled).toList();

void main() {
  test('waiting without progress stalls after the stall time', () {
    final (StallWatch watch, List<StallTick> out) = ticks(
      kStallWatchStart,
      kWaiting,
      kStallSeconds + 1,
    );

    expect(out.first.stalled, isFalse, reason: 'the first reading moves');
    expect(verdicts(out).sublist(1, kStallSeconds), everyElement(isNull));
    expect(out.last.stalled, isTrue);
    expect(watch.stuckS, kStallSeconds);
    expect(out.map((StallTick t) => t.movedPastError), everyElement(isFalse));
    expect(
      stallTick(watch, kWaiting, errored: false).stalled,
      isTrue,
      reason: 'a stall that goes on is still a stall',
    );
  });

  test('progress while waiting starts the count again', () {
    final (StallWatch stuck, _) = ticks(
      kStallWatchStart,
      kWaiting,
      kStallSeconds,
    );

    final StallTick moved = stallTick(
      stuck,
      kWaiting.copyWith(bufferedAheadMs: 500),
      errored: false,
    );

    expect(moved.stalled, isFalse);
    expect(moved.watch.stuckS, 0);
  });

  test('a reading that plays or is paused is never a stall', () {
    for (final PlayerSnapshot reading in <PlayerSnapshot>[kPlaying, kPaused]) {
      final (_, List<StallTick> out) = ticks(
        kStallWatchStart,
        reading,
        kStallSeconds + 1,
      );
      expect(verdicts(out), everyElement(isFalse), reason: '$reading');
    }
  });

  test('a paused reading after an error is not a stall and keeps the '
      'error', () {
    final StallWatch erred = stallErred(kStallWatchStart, kPaused);

    final (_, List<StallTick> out) = ticks(
      erred,
      kPaused,
      kStallSeconds + 1,
      errored: true,
    );

    expect(verdicts(out), everyElement(isFalse));
    expect(out.map((StallTick t) => t.movedPastError), everyElement(isFalse));
  });

  test('after an error a position that stops moving counts as a stall', () {
    final StallWatch erred = stallErred(kStallWatchStart, kPlaying);

    final (_, List<StallTick> out) = ticks(
      erred,
      kPlaying,
      kStallSeconds + 1,
      errored: true,
    );

    expect(out.last.stalled, isTrue);
  });

  test('an error is kept while playback stays where it came, and passes '
      'once it moves on', () {
    const PlayerSnapshot playingOut = PlayerSnapshot(
      durationMs: 100000,
      positionMs: 5000,
      bufferedAheadMs: 3000,
      ready: true,
      playing: true,
    );
    final StallWatch erred = stallErred(kStallWatchStart, playingOut);

    final StallTick same = stallTick(
      erred,
      playingOut.copyWith(positionMs: 6000, bufferedAheadMs: 2000),
      errored: true,
    );
    expect(same.movedPastError, isFalse);
    expect(same.stalled, isFalse);

    final StallTick moved = stallTick(
      same.watch,
      playingOut.copyWith(positionMs: 7000),
      errored: true,
    );
    expect(moved.movedPastError, isTrue);
    expect(
      stallTick(
        same.watch,
        playingOut.copyWith(positionMs: 7000),
        errored: false,
      ).movedPastError,
      isFalse,
      reason: 'with no error there is nothing to pass',
    );
  });

  test('a position that stops where an error was passed is not frozen', () {
    final StallWatch erred = stallErred(kStallWatchStart, kPlaying);
    final StallWatch moved = stallTick(
      erred,
      kPlaying.copyWith(bufferedAheadMs: 1000),
      errored: true,
    ).watch;

    final StallTick next = stallTick(
      moved,
      kPlaying.copyWith(bufferedAheadMs: 1000),
      errored: true,
    );

    expect(next.stalled, isFalse);
    expect(next.watch.stuckS, 0);
  });

  test('an error at the end where it happened is a stall at once', () {
    const PlayerSnapshot failing = PlayerSnapshot(
      durationMs: 100000,
      positionMs: 90000,
      bufferedAheadMs: 2000,
      ready: true,
      playing: true,
    );
    final StallWatch erred = stallErred(kStallWatchStart, failing);
    final PlayerSnapshot ended = failing.copyWith(playing: false, ended: true);

    expect(erred.errorAtMs, 92000);
    final StallTick atEnd = stallTick(erred, ended, errored: true);
    expect(atEnd.stalled, isTrue);
    expect(atEnd.movedPastError, isFalse);
    expect(atEnd.watch.stuckS, kStallSeconds);
    expect(atEnd.watch.progressMs, 92000);
    expect(atEnd.watch.positionMs, 90000);
    expect(atEnd.watch.errorAtMs, 92000);
    expect(
      stallTick(erred, ended, errored: false).stalled,
      isFalse,
      reason: 'an end with no error is just the end',
    );
    expect(
      stallTick(
        erred,
        ended.copyWith(bufferedAheadMs: 0),
        errored: true,
      ).stalled,
      isFalse,
      reason: 'it ended somewhere else',
    );
  });

  test('an error keeps the count and the position it came at', () {
    final (StallWatch stuck, _) = ticks(kStallWatchStart, kWaiting, 3);

    final StallWatch erred = stallErred(stuck, kWaiting);

    expect(erred.stuckS, stuck.stuckS);
    expect(erred.progressMs, stuck.progressMs);
    expect(erred.positionMs, stuck.positionMs);
    expect(erred.errorAtMs, 5000);
    expect(stallTick(erred, kWaiting, errored: true).watch.errorAtMs, 5000);
    expect(stallTick(erred, kPlaying, errored: true).watch.errorAtMs, 5000);
  });

  test('while not watching the watch starts over, keeping a held error '
      'marked where the player is', () {
    expect(stallIdle(kWaiting, errored: false), kStallWatchStart);

    final StallWatch held = stallIdle(
      const PlayerSnapshot(positionMs: 5000, bufferedAheadMs: 2000),
      errored: true,
    );
    expect(held.errorAtMs, 7000);
    expect(held.stuckS, 0);
    expect(held.progressMs, kStallWatchStart.progressMs);
  });
}
