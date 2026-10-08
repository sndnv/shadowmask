class EncoderTotals {
  const EncoderTotals({
    required this.encoder,
    this.produced = 0,
    this.failed = 0,
    this.timeMs = 0,
    this.timed = 0,
    this.mediaMs = 0,
  });

  final String encoder;
  final int produced;
  final int failed;
  final double timeMs;
  final int timed;
  final double mediaMs;

  double? get averageMs => timed == 0 ? null : timeMs / timed;

  double? get realtime => timeMs <= 0 ? null : mediaMs / timeMs;
}
