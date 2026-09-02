import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class MenuField extends StatelessWidget {
  const MenuField({
    super.key,
    required this.open,
    required this.onTap,
    required this.child,
    this.height = kControlHeight,
    this.width,
    this.enabled = true,
  });

  final bool open;
  final VoidCallback onTap;
  final Widget child;
  final double height;
  final double? width;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SelectionContainer.disabled(
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) {
          final bool fills = width != null || constraints.hasTightWidth;
          final Widget box = Container(
            height: height,
            width: width,
            padding: const EdgeInsets.symmetric(horizontal: Space.s2),
            decoration: BoxDecoration(
              color: enabled ? t.surface : t.surfaceAlt,
              borderRadius: const BorderRadius.all(Radii.sm),
              border: Border.all(color: t.border),
            ),
            child: Row(
              mainAxisSize: fills ? MainAxisSize.max : MainAxisSize.min,
              children: <Widget>[
                if (fills) Expanded(child: child) else Flexible(child: child),
                const SizedBox(width: Space.s2),
                AnimatedRotation(
                  turns: open ? 0.5 : 0,
                  duration: MediaQuery.disableAnimationsOf(context)
                      ? Duration.zero
                      : const Duration(milliseconds: 150),
                  child: Icon(
                    Icons.expand_more,
                    color: enabled ? t.muted : t.border,
                    size: 18,
                  ),
                ),
              ],
            ),
          );
          if (!enabled) {
            return box;
          }
          return InkWell(
            onTap: onTap,
            borderRadius: const BorderRadius.all(Radii.sm),
            child: box,
          );
        },
      ),
    );
  }
}
