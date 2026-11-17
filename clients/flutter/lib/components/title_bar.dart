import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class TitleBar extends StatelessWidget {
  const TitleBar({
    super.key,
    required this.title,
    required this.onClose,
    this.leading,
    this.dense = false,
  });

  final String title;
  final VoidCallback? onClose;
  final Widget? leading;
  final bool dense;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final bool bar = shapes.cut;
    final double pad = dense ? Space.s1 : 10;
    final Widget? lead = leading;
    return DecoratedBox(
      decoration: bar
          ? BoxDecoration(color: t.accent)
          : BoxDecoration(
              border: Border(bottom: BorderSide(color: t.border)),
            ),
      child: Padding(
        padding: EdgeInsets.fromLTRB(
          dense ? Space.s2 : Space.s4,
          pad,
          Space.s3,
          pad,
        ),
        child: Row(
          children: <Widget>[
            if (lead != null) ...<Widget>[
              lead,
              const SizedBox(width: Space.s2),
            ],
            Expanded(
              child: Text(
                shapes.caps(title),
                semanticsLabel: shapes.capsLabel(title),
                style: bar
                    ? tabTextStyle(t, MediaQuery.textScalerOf(context))
                    : TextStyle(
                        color: t.text,
                        fontSize: 15,
                        fontWeight: FontWeight.w600,
                      ),
              ),
            ),
            const SizedBox(width: Space.s3),
            IconButton(
              onPressed: onClose,
              tooltip: Strings.close,
              icon: const Icon(Icons.close, size: 18),
              color: bar ? t.accentContrast : t.muted,
              disabledColor: bar
                  ? t.accentContrast.withValues(alpha: 0.4)
                  : null,
              hoverColor: bar
                  ? t.accentContrast.withValues(alpha: 0.12)
                  : t.surfaceAlt,
              padding: const EdgeInsets.all(Space.s1),
              constraints: const BoxConstraints(),
              visualDensity: VisualDensity.compact,
            ),
          ],
        ),
      ),
    );
  }
}
