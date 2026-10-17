import 'package:shadowmask/player/player_snapshot.dart';

const int kPrebufferGiveUpSeconds = 10;

const int kBufferReachMs = 1000;

typedef BufferWatch = ({int aheadMs, int stillS});

const BufferWatch kBufferWatchStart = (aheadMs: 0, stillS: 0);

int bufferTargetMs(PlayerSnapshot reading, int bufferS) {
  final int chosen = bufferS * 1000;
  final int left = reading.durationMs - reading.positionMs;
  return left > 0 && left < chosen ? left : chosen;
}

(BufferWatch, bool) bufferTick(
  BufferWatch watch,
  PlayerSnapshot reading, {
  required int bufferS,
  required bool buffersAhead,
}) {
  final int aheadMs = reading.bufferedAheadMs;
  if (!buffersAhead ||
      aheadMs + kBufferReachMs >= bufferTargetMs(reading, bufferS)) {
    return (kBufferWatchStart, true);
  }
  final int stillS = aheadMs > watch.aheadMs ? 0 : watch.stillS + 1;
  return (
    (aheadMs: aheadMs, stillS: stillS),
    stillS >= kPrebufferGiveUpSeconds,
  );
}
