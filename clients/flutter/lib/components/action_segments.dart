import 'package:flutter/material.dart';

import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/radii.dart';
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

const BorderRadius kSegmentLeading = BorderRadius.horizontal(left: Radii.sm);
const BorderRadius kSegmentTrailing = BorderRadius.horizontal(right: Radii.sm);

ButtonStyle segmented(ButtonStyle base, BorderRadius radius) => base.copyWith(
  shape: WidgetStatePropertyAll<OutlinedBorder>(
    RoundedRectangleBorder(borderRadius: radius),
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
        style: segmented(kActionButtonStyle, kSegmentTrailing).copyWith(
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
