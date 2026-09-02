import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kActionControlHeight = 42;
const double kToggleControlHeight = 40;

class ToggleButton extends StatelessWidget {
  const ToggleButton({
    super.key,
    required this.icon,
    required this.filledIcon,
    required this.label,
    required this.pressed,
    required this.onToggle,
    this.tooltip,
    this.busy = false,
    this.compact = false,
    this.highlight = true,
    this.height = kToggleControlHeight,
  });

  final IconData icon;
  final IconData filledIcon;
  final String label;
  final String? tooltip;
  final bool pressed;
  final bool busy;
  final bool compact;
  final bool highlight;
  final double height;
  final VoidCallback? onToggle;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final bool lit = pressed && highlight;
    final Color labelColor = lit ? t.accent : t.text;
    final Color iconColor = lit ? t.accent : t.muted;
    final Color bg = lit ? t.accent.withValues(alpha: 0.12) : t.surface;
    final IconData glyph = pressed ? filledIcon : icon;
    final Widget content = compact
        ? Icon(glyph, size: 20, color: iconColor)
        : Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(glyph, size: 20, color: iconColor),
              const SizedBox(width: Space.s2),
              Text(
                label,
                style: TextStyle(
                  color: labelColor,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          );
    final Widget button = Semantics(
      button: true,
      toggled: highlight ? pressed : null,
      label: label,
      child: Opacity(
        opacity: busy ? 0.5 : 1,
        child: SizedBox(
          height: height,
          child: Material(
            color: bg,
            shape: RoundedRectangleBorder(
              borderRadius: const BorderRadius.all(Radii.sm),
              side: BorderSide(color: lit ? t.accent : t.border),
            ),
            child: InkWell(
              borderRadius: const BorderRadius.all(Radii.sm),
              onTap: busy ? null : onToggle,
              child: Padding(
                padding: EdgeInsets.symmetric(horizontal: compact ? 10 : 14),
                child: Center(widthFactor: 1, child: content),
              ),
            ),
          ),
        ),
      ),
    );
    final String? message = tooltip ?? (compact ? label : null);
    return SelectionContainer.disabled(
      child: message == null
          ? button
          : Tooltip(message: message, child: button),
    );
  }
}
