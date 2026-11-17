import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';

class EdgedBox extends StatelessWidget {
  const EdgedBox({
    super.key,
    required this.corner,
    required this.edge,
    this.color,
    this.shadows,
    this.width,
    this.height,
    this.inset = true,
    this.clipBehavior = Clip.antiAlias,
    this.child,
  });

  final Corner corner;
  final BorderSide edge;
  final Color? color;
  final List<BoxShadow>? shadows;
  final double? width;
  final double? height;
  final bool inset;
  final Clip clipBehavior;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final Shapes shapes = context.shapes;
    final bool cut = shapes.cut;
    return Container(
      width: width,
      height: height,
      clipBehavior: clipBehavior,
      padding: cut && inset ? EdgeInsets.all(edge.strokeInset) : null,
      decoration: shapes.box(
        corner,
        color: color,
        side: cut ? null : edge,
        shadows: shadows,
      ),
      foregroundDecoration: cut ? shapes.box(corner, side: edge) : null,
      child: child,
    );
  }
}
