import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

import 'package:shadowmask/theme/tokens_context.dart';

class HexTexture extends StatelessWidget {
  const HexTexture({super.key});

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      child: IgnorePointer(
        child: CustomPaint(
          size: Size.infinite,
          painter: _HexPainter(
            context.tokens.text.withValues(alpha: 0.06),
            MediaQuery.devicePixelRatioOf(context),
          ),
        ),
      ),
    );
  }
}

class _HexPainter extends CustomPainter {
  const _HexPainter(this.color, this.pixelRatio);

  final Color color;
  final double pixelRatio;

  static const double _cell = 14.0;
  static const double _rowHeight = 12.0;
  static const double _tileWidth = _cell;
  static const double _tileHeight = _rowHeight * 2;
  static const List<Offset> _centers = <Offset>[
    Offset.zero,
    Offset(_tileWidth, 0),
    Offset(0, _tileHeight),
    Offset(_tileWidth, _tileHeight),
    Offset(_cell / 2, _rowHeight),
  ];
  static final Map<(Color, double), ui.Image> _tiles =
      <(Color, double), ui.Image>{};

  @override
  void paint(Canvas canvas, Size size) {
    final ui.Image tile = _tiles.putIfAbsent((color, pixelRatio), _drawTile);
    final Float64List matrix = Matrix4.diagonal3Values(
      _tileWidth / tile.width,
      _tileHeight / tile.height,
      1,
    ).storage;
    canvas.drawRect(
      Offset.zero & size,
      Paint()
        ..shader = ui.ImageShader(
          tile,
          ui.TileMode.repeated,
          ui.TileMode.repeated,
          matrix,
          filterQuality: FilterQuality.low,
        ),
    );
  }

  ui.Image _drawTile() {
    final int width = (_tileWidth * pixelRatio).ceil();
    final int height = (_tileHeight * pixelRatio).ceil();
    final ui.PictureRecorder recorder = ui.PictureRecorder();
    final Canvas canvas = Canvas(recorder)
      ..scale(width / _tileWidth, height / _tileHeight);
    final Path path = Path();
    for (final Offset center in _centers) {
      _addHexagon(path, center.dx, center.dy);
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.fill,
    );
    final ui.Picture picture = recorder.endRecording();
    final ui.Image image = picture.toImageSync(width, height);
    picture.dispose();
    return image;
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
  bool shouldRepaint(_HexPainter oldDelegate) =>
      oldDelegate.color != color || oldDelegate.pixelRatio != pixelRatio;
}
