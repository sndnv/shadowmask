class PlayerSnapshot {
  const PlayerSnapshot({
    this.positionMs = 0,
    this.durationMs = 0,
    this.bufferedAheadMs = 0,
    this.playing = false,
    this.ready = false,
    this.buffering = false,
    this.bufferingPercent = 0,
    this.ended = false,
    this.error,
  });

  final int positionMs;
  final int durationMs;
  final int bufferedAheadMs;
  final bool playing;
  final bool ready;
  final bool buffering;
  final double bufferingPercent;
  final bool ended;
  final String? error;

  double get fraction =>
      durationMs > 0 ? (positionMs / durationMs).clamp(0.0, 1.0) : 0.0;

  bool get waiting => error == null && !ended && (buffering || !ready);

  PlayerSnapshot onTimeline({required int originMs, required int fullMs}) {
    if (originMs == 0) {
      return this;
    }
    return PlayerSnapshot(
      positionMs: positionMs + originMs,
      durationMs: fullMs > 0 ? fullMs : durationMs + originMs,
      bufferedAheadMs: bufferedAheadMs,
      playing: playing,
      ready: ready,
      buffering: buffering,
      bufferingPercent: bufferingPercent,
      ended: ended,
      error: error,
    );
  }
}
