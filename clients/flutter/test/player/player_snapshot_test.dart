import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/player_snapshot.dart';

void main() {
  const PlayerSnapshot paused = PlayerSnapshot(
    positionMs: 1000,
    durationMs: 100000,
    bufferedAheadMs: 5000,
    ready: true,
    bufferingPercent: 40,
  );

  test('two readings of a paused player are the same snapshot', () {
    const PlayerSnapshot again = PlayerSnapshot(
      positionMs: 1000,
      durationMs: 100000,
      bufferedAheadMs: 5000,
      ready: true,
      bufferingPercent: 40,
    );

    expect(again, paused);
    expect(again.hashCode, paused.hashCode);
  });

  test('any changed reading is a different snapshot', () {
    expect(paused.copyWith(positionMs: 1250), isNot(paused));
    expect(paused.copyWith(bufferedAheadMs: 6000), isNot(paused));
    expect(paused.copyWith(playing: true), isNot(paused));
    expect(paused.copyWith(buffering: true), isNot(paused));
    expect(paused.copyWith(ended: true), isNot(paused));
    expect(paused.copyWith(error: 'gone'), isNot(paused));
  });

  test('a repeated reading does not notify the player', () {
    final ValueNotifier<PlayerSnapshot> feed = ValueNotifier<PlayerSnapshot>(
      paused,
    );
    addTearDown(feed.dispose);
    int notified = 0;
    feed.addListener(() => notified++);

    feed.value = paused.copyWith();
    expect(notified, 0);

    feed.value = paused.copyWith(positionMs: 1250);
    expect(notified, 1);
  });

  test('the timeline shifts the position and duration by the origin', () {
    final PlayerSnapshot shifted = paused.onTimeline(
      originMs: 60000,
      fullMs: 0,
    );

    expect(shifted.positionMs, 61000);
    expect(shifted.durationMs, 160000);
    expect(shifted.bufferedAheadMs, 5000);
    expect(shifted.ready, isTrue);
    expect(
      paused.onTimeline(originMs: 60000, fullMs: 200000).durationMs,
      200000,
    );
    expect(
      identical(paused.onTimeline(originMs: 0, fullMs: 0), paused),
      isTrue,
    );
  });

  test('progress and waiting read from the fields', () {
    expect(paused.fraction, 0.01);
    expect(paused.waiting, isFalse);
    expect(const PlayerSnapshot().fraction, 0);
    expect(const PlayerSnapshot().waiting, isTrue);
    expect(const PlayerSnapshot(error: 'gone').waiting, isFalse);
    expect(paused.copyWith(buffering: true).waiting, isTrue);
  });
}
