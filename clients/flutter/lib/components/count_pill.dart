import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class CountPill extends StatelessWidget {
  const CountPill({super.key, required this.count});

  final int count;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 1),
      decoration: BoxDecoration(
        color: t.accent,
        borderRadius: const BorderRadius.all(Radii.pill),
      ),
      child: Text(
        '$count',
        style: monoStyle.copyWith(
          color: t.accentContrast,
          fontSize: 11,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}
