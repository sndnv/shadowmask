import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/view/bar_segments.dart';

const double kSegmentedBarHeight = 6;

class SegmentedBar extends StatelessWidget {
  const SegmentedBar({
    super.key,
    required this.fraction,
    required this.track,
    required this.fill,
  });

  final double fraction;
  final Color track;
  final Color fill;

  @override
  Widget build(BuildContext context) => SizedBox(
    width: double.infinity,
    height: kSegmentedBarHeight,
    child: CustomPaint(
      painter: _SegmentPainter(
        fraction: fraction.clamp(0.0, 1.0),
        track: track,
        fill: fill,
      ),
    ),
  );
}

class _SegmentPainter extends CustomPainter {
  const _SegmentPainter({
    required this.fraction,
    required this.track,
    required this.fill,
  });

  final double fraction;
  final Color track;
  final Color fill;

  @override
  void paint(Canvas canvas, Size size) {
    final double split = size.width * fraction;
    _segments(canvas, size, Rect.fromLTRB(0, 0, split, size.height), fill);
    _segments(
      canvas,
      size,
      Rect.fromLTRB(split, 0, size.width, size.height),
      track,
    );
  }

  void _segments(Canvas canvas, Size size, Rect within, Color color) {
    final Paint paint = Paint()..color = color;
    canvas
      ..save()
      ..clipRect(within);
    for (final ({double left, double right}) s in barSegments(size.width)) {
      canvas.drawRect(Rect.fromLTRB(s.left, 0, s.right, size.height), paint);
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_SegmentPainter oldDelegate) =>
      oldDelegate.fraction != fraction ||
      oldDelegate.track != track ||
      oldDelegate.fill != fill;
}
