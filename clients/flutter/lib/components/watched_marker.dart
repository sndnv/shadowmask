import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class WatchedMarker extends StatelessWidget {
  const WatchedMarker({super.key});

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    return Tooltip(
      message: Strings.watchedLabel,
      child: shapes.cut
          ? Container(
              width: Marks.watched.width,
              height: Marks.watched.height,
              decoration: ShapeDecoration(
                color: t.ok,
                shape: Shapes.hexagon(Marks.watched),
              ),
              child: Icon(Icons.check, size: 16, color: t.accentContrast),
            )
          : Container(
              width: 22,
              height: 22,
              decoration: shapes.roundBox(
                color: t.ok,
                side: BorderSide(color: t.surface, width: 2),
              ),
              child: const Icon(Icons.check, size: 13, color: Colors.white),
            ),
    );
  }
}
