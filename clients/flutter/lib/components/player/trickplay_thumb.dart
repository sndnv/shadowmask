import 'package:flutter/material.dart';

import 'package:shadowmask/components/player/trickplay_tile.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

class TrickplayThumb extends StatelessWidget {
  const TrickplayThumb({
    super.key,
    required this.tile,
    required this.positionMs,
    this.width = 168,
  });

  final TrickplayTile? tile;
  final int positionMs;
  final double width;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double aspect = tile?.aspect ?? 16 / 9;
    final double height = width / aspect;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          width: width,
          height: height,
          clipBehavior: Clip.antiAlias,
          decoration: BoxDecoration(
            color: t.artBg,
            border: Border.all(color: t.border),
            borderRadius: const BorderRadius.all(Radii.sm),
          ),
          child: tile == null
              ? null
              : CustomPaint(painter: _TilePainter(tile!)),
        ),
        const SizedBox(height: Space.s1),
        Container(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.s2,
            vertical: 2,
          ),
          decoration: BoxDecoration(
            color: t.surface,
            borderRadius: const BorderRadius.all(Radii.sm),
          ),
          child: Text(
            clock(positionMs),
            style: monoStyle.copyWith(color: t.text, fontSize: 12),
          ),
        ),
      ],
    );
  }
}

class _TilePainter extends CustomPainter {
  _TilePainter(this.tile);

  final TrickplayTile tile;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawImageRect(
      tile.image,
      tile.src,
      Offset.zero & size,
      Paint()..filterQuality = FilterQuality.medium,
    );
  }

  @override
  bool shouldRepaint(_TilePainter oldDelegate) => oldDelegate.tile != tile;
}
