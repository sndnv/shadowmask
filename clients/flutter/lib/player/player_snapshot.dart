class PlayerSnapshot {
  const PlayerSnapshot({
    this.positionMs = 0,
    this.durationMs = 0,
    this.bufferedAheadMs = 0,
    this.playing = false,
    this.ready = false,
    this.ended = false,
    this.error,
  });

  final int positionMs;
  final int durationMs;
  final int bufferedAheadMs;
  final bool playing;
  final bool ready;
  final bool ended;
  final String? error;

  double get fraction =>
      durationMs > 0 ? (positionMs / durationMs).clamp(0.0, 1.0) : 0.0;
}
