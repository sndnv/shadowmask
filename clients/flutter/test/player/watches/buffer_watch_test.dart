import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/player/player_snapshot.dart';
import 'package:shadowmask/player/watches/buffer_watch.dart';

PlayerSnapshot ahead(int ms, {int positionMs = 0}) => PlayerSnapshot(
  durationMs: 600000,
  positionMs: positionMs,
  bufferedAheadMs: ms,
  ready: true,
);

(BufferWatch, bool) tick(
  BufferWatch watch,
  PlayerSnapshot reading, {
  bool buffersAhead = true,
}) => bufferTick(watch, reading, bufferS: 60, buffersAhead: buffersAhead);

void main() {
  test('the buffer is ready within its reach of the target', () {
    expect(tick(kBufferWatchStart, ahead(59000)).$2, isTrue);
    expect(tick(kBufferWatchStart, ahead(59000)).$1, kBufferWatchStart);
    expect(tick(kBufferWatchStart, ahead(58999)).$2, isFalse);
    expect(tick(kBufferWatchStart, ahead(90000)).$2, isTrue);
  });

  test('the target is what is left when that is shorter', () {
    expect(bufferTargetMs(ahead(0, positionMs: 570000), 60), 30000);
    expect(bufferTargetMs(ahead(0, positionMs: 540000), 60), 60000);
    expect(bufferTargetMs(ahead(0, positionMs: 600000), 60), 60000);
    expect(
      bufferTargetMs(const PlayerSnapshot(positionMs: 5000), 60),
      60000,
      reason: 'an unknown length waits for the whole target',
    );
    expect(
      tick(kBufferWatchStart, ahead(29000, positionMs: 570000)).$2,
      isTrue,
    );
  });

  test('a buffer that stops growing gives up after the give-up time', () {
    BufferWatch watch = kBufferWatchStart;
    final List<bool> verdicts = <bool>[];
    for (int i = 0; i <= kPrebufferGiveUpSeconds; i++) {
      final (BufferWatch next, bool ready) = tick(watch, ahead(1000));
      watch = next;
      verdicts.add(ready);
    }

    expect(verdicts.sublist(0, kPrebufferGiveUpSeconds), everyElement(isFalse));
    expect(verdicts.last, isTrue);
  });

  test('growth starts the give-up count again', () {
    final BufferWatch still = (aheadMs: 1000, stillS: 9);

    final (BufferWatch grown, bool ready) = tick(still, ahead(1500));

    expect(ready, isFalse);
    expect(grown, (aheadMs: 1500, stillS: 0));
    expect(tick(still, ahead(1000)).$1, (aheadMs: 1000, stillS: 10));
    expect(
      tick((aheadMs: 1000, stillS: 10), ahead(1000)).$2,
      isTrue,
      reason: 'a give-up the state did not take yet is given again',
    );
  });

  test('a player that cannot buffer ahead is ready at once', () {
    expect(tick(kBufferWatchStart, ahead(0), buffersAhead: false).$2, isTrue);
  });
}
