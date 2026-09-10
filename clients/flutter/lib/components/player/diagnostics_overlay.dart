import 'dart:async';
import 'dart:ui' show DisplayFeature, DisplayFeatureType;

import 'package:flutter/material.dart';

import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DiagnosticsOverlay extends StatefulWidget {
  const DiagnosticsOverlay({
    super.key,
    required this.controller,
    this.sourceLine,
  });

  final PlayerController controller;
  final String? sourceLine;

  @override
  State<DiagnosticsOverlay> createState() => _DiagnosticsOverlayState();
}

class _DiagnosticsOverlayState extends State<DiagnosticsOverlay> {
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) {
        setState(() {});
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final List<String> lines = <String>[
      if (widget.sourceLine != null) widget.sourceLine!,
      _insetsLine(context),
      ...widget.controller.diagnostics().lines,
    ];
    return Container(
      padding: const EdgeInsets.all(Space.s2),
      decoration: BoxDecoration(
        color: t.surface.withValues(alpha: 0.72),
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: SelectableText(
        lines.join('\n'),
        style: monoStyle.copyWith(color: t.text, fontSize: 12, height: 1.4),
      ),
    );
  }

  String _insetsLine(BuildContext context) {
    final int cutouts = MediaQuery.displayFeaturesOf(context)
        .where(
          (DisplayFeature feature) => feature.type == DisplayFeatureType.cutout,
        )
        .length;
    return 'insets: padding ${_edges(MediaQuery.paddingOf(context))}'
        '  view ${_edges(MediaQuery.viewPaddingOf(context))}'
        '  cutouts $cutouts';
  }

  String _edges(EdgeInsets insets) =>
      '${insets.left.round()},${insets.top.round()},'
      '${insets.right.round()},${insets.bottom.round()}';
}
