class DeliveryTotals {
  const DeliveryTotals({
    required this.delivery,
    this.sessions = 0,
    this.started = 0,
    this.failed = 0,
    this.firstSegmentMs = 0,
    this.firstSegments = 0,
  });

  final String delivery;
  final int sessions;
  final int started;
  final int failed;
  final double firstSegmentMs;
  final int firstSegments;

  bool get streams => delivery != 'direct';

  double? get averageFirstSegmentMs =>
      firstSegments == 0 ? null : firstSegmentMs / firstSegments;
}
