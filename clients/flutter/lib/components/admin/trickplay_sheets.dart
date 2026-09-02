import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/player/trickplay_loader.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class TrickplaySheets extends StatefulWidget {
  const TrickplaySheets({
    super.key,
    required this.api,
    required this.versionId,
    required this.refs,
    this.sheetWidth = 480,
  });

  final PlaybackApi api;
  final String versionId;
  final List<TrickplayRef> refs;
  final double sheetWidth;

  @override
  State<TrickplaySheets> createState() => _TrickplaySheetsState();
}

class _TrickplaySheetsState extends State<TrickplaySheets> {
  late final TrickplayLoader _loader = TrickplayLoader(
    api: widget.api,
    versionId: widget.versionId,
    refs: widget.refs,
  );

  @override
  void dispose() {
    _loader.dispose();
    super.dispose();
  }

  double get _gridAspect {
    if (widget.refs.isEmpty) {
      return 16 / 9;
    }
    final TrickplayRef r = widget.refs.first;
    final int width = r.columns * r.tileWidth;
    final int height = r.rows * r.tileHeight;
    return height > 0 ? width / height : 16 / 9;
  }

  Widget _sheet(Tokens t, int sheet) {
    final ui.Image? image = _loader.sheetImage(sheet, () {
      if (mounted) {
        setState(() {});
      }
    });
    final double aspect = image == null
        ? _gridAspect
        : image.width / image.height;
    return Container(
      width: widget.sheetWidth,
      height: widget.sheetWidth / aspect,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: t.artBg,
        border: Border.all(color: t.border),
        borderRadius: const BorderRadius.all(Radii.sm),
      ),
      child: image == null ? null : CustomPaint(painter: _SheetPainter(image)),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final int count = _loader.sheetCount;
    if (count <= 0) {
      return const SizedBox.shrink();
    }
    return Wrap(
      spacing: Space.s3,
      runSpacing: Space.s3,
      children: <Widget>[
        for (int sheet = 1; sheet <= count; sheet++) _sheet(t, sheet),
      ],
    );
  }
}

class _SheetPainter extends CustomPainter {
  _SheetPainter(this.image);

  final ui.Image image;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawImageRect(
      image,
      Offset.zero & Size(image.width.toDouble(), image.height.toDouble()),
      Offset.zero & size,
      Paint()..filterQuality = FilterQuality.medium,
    );
  }

  @override
  bool shouldRepaint(_SheetPainter oldDelegate) => oldDelegate.image != image;
}
