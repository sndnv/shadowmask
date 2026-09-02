import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kProgressBarHeight = 4;

class ProgressBar extends StatelessWidget {
  const ProgressBar({super.key, required this.percent});

  final int percent;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      height: kProgressBarHeight,
      color: Colors.black.withValues(alpha: 0.45),
      child: FractionallySizedBox(
        alignment: Alignment.centerLeft,
        widthFactor: (percent / 100).clamp(0.0, 1.0),
        heightFactor: 1,
        child: ColoredBox(color: t.accent),
      ),
    );
  }
}
