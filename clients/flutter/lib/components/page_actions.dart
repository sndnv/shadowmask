import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/space.dart';

class PageAction {
  const PageAction({
    required this.icon,
    required this.label,
    required this.onPressed,
    this.primary = false,
    this.danger = false,
    this.tooltip,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onPressed;
  final bool primary;
  final bool danger;
  final String? tooltip;
}

Widget pageActionButton(
  BuildContext context,
  PageAction action, {
  bool compact = false,
  bool iconOnly = false,
}) {
  final Widget icon = Icon(action.icon, size: 18);
  final Widget label = Text(action.label);
  final ButtonStyle? danger = action.danger
      ? OutlinedButton.styleFrom(
          foregroundColor: Theme.of(context).colorScheme.error,
        )
      : null;
  final ButtonStyle? base = action.primary
      ? (compact ? kFieldButtonStyle : null)
      : (compact ? kFieldButtonStyle.merge(danger) : danger);
  if (iconOnly) {
    final ButtonStyle squeezed = OutlinedButton.styleFrom(
      padding: const EdgeInsets.symmetric(horizontal: Space.s2),
    );
    final ButtonStyle style = base == null ? squeezed : base.merge(squeezed);
    return Tooltip(
      message: action.tooltip ?? action.label,
      child: action.primary
          ? FilledButton(onPressed: action.onPressed, style: style, child: icon)
          : OutlinedButton(
              onPressed: action.onPressed,
              style: style,
              child: icon,
            ),
    );
  }
  final Widget button = action.primary
      ? FilledButton.icon(
          onPressed: action.onPressed,
          style: base,
          icon: icon,
          label: label,
        )
      : OutlinedButton.icon(
          onPressed: action.onPressed,
          style: base,
          icon: icon,
          label: label,
        );
  final String? message = action.tooltip;
  return message == null ? button : Tooltip(message: message, child: button);
}

class PageActions extends StatelessWidget {
  const PageActions(this.actions, {super.key});

  final List<PageAction> actions;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      child: Wrap(
        alignment: WrapAlignment.end,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: Space.s3,
        runSpacing: Space.s2,
        children: <Widget>[
          for (final PageAction action in actions)
            pageActionButton(context, action),
        ],
      ),
    );
  }
}
