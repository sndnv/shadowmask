import 'dart:math' as math;

class TimelineMarks {
  const TimelineMarks({
    required this.fillTo,
    required this.previewTo,
    required this.thumbAt,
    required this.ghostAt,
  });

  factory TimelineMarks.of({
    required double width,
    required double fraction,
    double? preview,
  }) {
    final double at = width * fraction.clamp(0.0, 1.0);
    if (preview == null) {
      return TimelineMarks(
        fillTo: at,
        previewTo: at,
        thumbAt: at,
        ghostAt: null,
      );
    }
    final double target = width * preview.clamp(0.0, 1.0);
    return TimelineMarks(
      fillTo: math.min(at, target),
      previewTo: math.max(at, target),
      thumbAt: at,
      ghostAt: target,
    );
  }

  final double fillTo;
  final double previewTo;
  final double thumbAt;
  final double? ghostAt;

  bool get hasPreview => previewTo > fillTo;

  @override
  bool operator ==(Object other) =>
      other is TimelineMarks &&
      other.fillTo == fillTo &&
      other.previewTo == previewTo &&
      other.thumbAt == thumbAt &&
      other.ghostAt == ghostAt;

  @override
  int get hashCode => Object.hash(fillTo, previewTo, thumbAt, ghostAt);
}
