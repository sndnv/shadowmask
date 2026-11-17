import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const Offset kMenuOffset = Offset(0, Space.s1);

const double kCardMenuWidth = 300;
const double kCardMenuDividerInset = kCardMenuWidth * 0.1;

MenuStyle appMenuStyle(BuildContext context, {double? minWidth}) {
  final Tokens t = context.tokens;
  final Shapes shapes = context.shapes;
  return MenuStyle(
    backgroundColor: WidgetStatePropertyAll<Color>(t.surface),
    surfaceTintColor: const WidgetStatePropertyAll<Color>(Colors.transparent),
    minimumSize: minWidth == null
        ? null
        : WidgetStatePropertyAll<Size>(Size(minWidth, 0)),
    shape: WidgetStatePropertyAll<OutlinedBorder>(
      shapes.border(
        Corners.panel,
        side: BorderSide(color: shapes.line(t.border)),
      ),
    ),
    padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
      EdgeInsets.symmetric(vertical: Space.s2),
    ),
  );
}
