import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

class TrickplayTile {
  const TrickplayTile({
    required this.image,
    required this.src,
    required this.aspect,
  });

  final ui.Image image;
  final Rect src;
  final double aspect;
}
