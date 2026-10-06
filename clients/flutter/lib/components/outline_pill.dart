import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class OutlinePill extends StatelessWidget {
  const OutlinePill({
    super.key,
    required this.icon,
    required this.text,
    this.active = true,
  });

  final IconData icon;
  final String text;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Color fg = active ? t.accent : t.muted;
    final Color borderColor = active
        ? Color.alphaBlend(t.accent.withValues(alpha: 0.45), t.border)
        : t.border;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.s2, vertical: 2),
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radii.pill),
        border: Border.all(color: borderColor),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 14, color: fg),
          const SizedBox(width: 5),
          Text(
            text,
            style: monoStyle.copyWith(
              color: fg,
              fontSize: 11,
              fontWeight: FontWeight.w600,
              fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }
}
