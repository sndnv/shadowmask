import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';

class StatusNode extends StatelessWidget {
  const StatusNode({super.key, required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) => Container(
    width: Marks.node.width,
    height: Marks.node.height,
    decoration: ShapeDecoration(
      color: color,
      shape: Shapes.hexagon(Marks.node),
    ),
  );
}
