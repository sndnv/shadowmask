import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/segmented_bar.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kProgressBarHeight = 4;

class ProgressBar extends StatelessWidget {
  const ProgressBar({super.key, required this.percent});

  final int percent;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final double fraction = (percent / 100).clamp(0.0, 1.0);
    if (shapes.cut) {
      return SegmentedBar(
        fraction: fraction,
        track: shapes.track(t.border),
        fill: t.accent,
      );
    }
    return Container(
      height: kProgressBarHeight,
      color: Colors.black.withValues(alpha: 0.45),
      child: FractionallySizedBox(
        alignment: Alignment.centerLeft,
        widthFactor: fraction,
        heightFactor: 1,
        child: ColoredBox(color: t.accent),
      ),
    );
  }
}
