import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/edged_box.dart';
import 'package:shadowmask/components/title_bar.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class DialogShell extends StatelessWidget {
  const DialogShell({
    super.key,
    required this.title,
    required this.child,
    this.footer,
    this.onClose,
    this.enableClose = true,
    this.width = 480,
  });

  final String title;
  final Widget child;
  final Widget? footer;
  final VoidCallback? onClose;
  final bool enableClose;
  final double width;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double maxHeight = MediaQuery.of(context).size.height * 0.7;
    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: const EdgeInsets.all(Space.s5),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width, maxHeight: maxHeight),
        child: EdgedBox(
          corner: Corners.panel,
          color: t.surface,
          edge: BorderSide(color: context.shapes.line(t.border)),
          inset: false,
          shadows: <BoxShadow>[
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.5),
              blurRadius: 24,
              offset: const Offset(0, 8),
            ),
          ],
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              TitleBar(
                title: title,
                onClose: !enableClose
                    ? null
                    : (onClose ?? () => Navigator.of(context).pop()),
              ),
              Flexible(
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(Space.s4),
                  child: SelectionArea(child: child),
                ),
              ),
              if (footer != null)
                DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border(top: BorderSide(color: t.border)),
                  ),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: Space.s4,
                      vertical: Space.s3,
                    ),
                    child: footer!,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}
