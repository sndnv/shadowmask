import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const ButtonStyle kActionButtonStyle = ButtonStyle(
  padding: WidgetStatePropertyAll<EdgeInsetsGeometry>(
    EdgeInsets.symmetric(horizontal: 20),
  ),
  minimumSize: WidgetStatePropertyAll<Size>(Size(0, kActionControlHeight)),
  maximumSize: WidgetStatePropertyAll<Size>(
    Size(double.infinity, kActionControlHeight),
  ),
  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  textStyle: WidgetStatePropertyAll<TextStyle>(
    TextStyle(fontSize: 15, fontWeight: FontWeight.w700),
  ),
);

ButtonStyle segmented(ButtonStyle base, Shapes shapes, CornerPart part) =>
    base.copyWith(
      shape: WidgetStatePropertyAll<OutlinedBorder>(
        shapes.border(Corners.button, part: part),
      ),
    );

const double kDismissTint = 0.35;

class DismissSegment extends StatelessWidget {
  const DismissSegment({super.key, required this.onPressed});

  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Tooltip(
      message: Strings.dismissResume,
      child: FilledButton(
        onPressed: onPressed,
        style:
            segmented(
              kActionButtonStyle,
              context.shapes,
              CornerPart.trailing,
            ).copyWith(
              padding: const WidgetStatePropertyAll<EdgeInsetsGeometry>(
                EdgeInsets.symmetric(horizontal: Space.s3),
              ),
              backgroundColor: WidgetStatePropertyAll<Color>(
                Color.alphaBlend(
                  t.accent.withValues(alpha: kDismissTint),
                  t.surface,
                ),
              ),
              foregroundColor: WidgetStatePropertyAll<Color>(t.text),
            ),
        child: const Icon(Icons.close, size: 20),
      ),
    );
  }
}
