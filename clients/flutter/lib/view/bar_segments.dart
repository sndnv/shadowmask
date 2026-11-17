import 'dart:math' as math;

const double kBarSegment = 6;
const double kBarGap = 2;

List<({double left, double right})> barSegments(double width) =>
    <({double left, double right})>[
      for (double left = 0; left < width; left += kBarSegment + kBarGap)
        (left: left, right: math.min(left + kBarSegment, width)),
    ];
