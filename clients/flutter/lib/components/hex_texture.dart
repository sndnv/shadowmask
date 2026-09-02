import 'dart:math' as math;

import 'package:flutter/widgets.dart';

import 'package:shadowmask/theme/tokens_context.dart';

class HexTexture extends StatelessWidget {
  const HexTexture({super.key});

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: CustomPaint(
        size: Size.infinite,
        painter: _HexPainter(context.tokens.text.withValues(alpha: 0.06)),
      ),
    );
  }
}

class _HexPainter extends CustomPainter {
  const _HexPainter(this.color);

  final Color color;

  static const double _cell = 14.0;

  @override
  void paint(Canvas canvas, Size size) {
    final double rowHeight = _cell * math.sqrt(3) / 2;
    final Path path = Path();
    int row = 0;
    for (double y = 0; y <= size.height + rowHeight; y += rowHeight) {
      final double xOffset = row.isOdd ? _cell / 2 : 0;
      for (double x = xOffset; x <= size.width + _cell; x += _cell) {
        _addHexagon(path, x, y);
      }
      row++;
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.fill,
    );
  }

  void _addHexagon(Path path, double cx, double cy) {
    const double halfWidth = 1.82;
    const double halfHeight = 2.1;
    const double midHeight = 1.05;
    path.moveTo(cx, cy - halfHeight);
    path.lineTo(cx + halfWidth, cy - midHeight);
    path.lineTo(cx + halfWidth, cy + midHeight);
    path.lineTo(cx, cy + halfHeight);
    path.lineTo(cx - halfWidth, cy + midHeight);
    path.lineTo(cx - halfWidth, cy - midHeight);
    path.close();
  }

  @override
  bool shouldRepaint(_HexPainter oldDelegate) => oldDelegate.color != color;
}
