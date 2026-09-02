import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double _kReadingWidth = 48;

class ProgressMeter extends StatelessWidget {
  const ProgressMeter({super.key, required this.percent, this.width = 90});

  final int percent;
  final double width;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double fraction = (percent / 100).clamp(0.0, 1.0);
    final Text reading = Text(
      '$percent%',
      style: TextStyle(color: t.muted, fontSize: 12),
    );
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < width + _kReadingWidth) {
          return reading;
        }
        return _bar(t, fraction, reading);
      },
    );
  }

  Widget _bar(Tokens t, double fraction, Widget reading) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          width: width,
          height: 6,
          clipBehavior: Clip.antiAlias,
          decoration: BoxDecoration(
            color: t.surfaceAlt,
            borderRadius: const BorderRadius.all(Radii.pill),
          ),
          child: FractionallySizedBox(
            alignment: Alignment.centerLeft,
            widthFactor: fraction,
            heightFactor: 1,
            child: ColoredBox(color: t.accent),
          ),
        ),
        const SizedBox(width: Space.s2),
        reading,
      ],
    );
  }
}
