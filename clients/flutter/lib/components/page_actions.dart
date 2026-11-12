import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

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
    final String? reason = action.tooltip;
    return Tooltip(
      message: reason == null ? action.label : '${action.label}: $reason',
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

const int kMaxUpfrontActions = 3;

List<Widget> pageActionButtons(
  BuildContext context,
  List<PageAction> actions, {
  bool iconOnly = false,
}) {
  if (actions.length <= kMaxUpfrontActions) {
    return <Widget>[
      for (final PageAction action in actions)
        pageActionButton(context, action, iconOnly: iconOnly),
    ];
  }
  const int shown = kMaxUpfrontActions - 1;
  return <Widget>[
    for (final PageAction action in actions.take(shown))
      pageActionButton(context, action, iconOnly: iconOnly),
    _ActionsMenu(actions: actions.skip(shown).toList()),
  ];
}

class _ActionsMenu extends StatelessWidget {
  const _ActionsMenu({required this.actions});

  final List<PageAction> actions;

  Widget _item(PageAction action, Color danger) {
    final Widget item = MenuItemButton(
      onPressed: action.onPressed,
      style: action.danger
          ? MenuItemButton.styleFrom(foregroundColor: danger, iconColor: danger)
          : null,
      leadingIcon: Icon(action.icon, size: 18),
      child: Text(action.label),
    );
    final String? message = action.tooltip;
    return message == null ? item : Tooltip(message: message, child: item);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Color danger = Theme.of(context).colorScheme.error;
    return MenuAnchor(
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: <Widget>[
        for (final PageAction action in actions) _item(action, danger),
      ],
      builder: (BuildContext context, MenuController controller, Widget? _) =>
          pageActionButton(
            context,
            PageAction(
              icon: Icons.more_horiz,
              label: Strings.moreActions,
              onPressed: () =>
                  controller.isOpen ? controller.close() : controller.open(),
            ),
            iconOnly: true,
          ),
    );
  }
}

class PageActions extends StatelessWidget {
  const PageActions(this.actions, {super.key});

  final List<PageAction> actions;

  @override
  Widget build(BuildContext context) {
    final bool iconOnly = compactViewport(context);
    return SizedBox(
      width: double.infinity,
      child: Wrap(
        alignment: WrapAlignment.end,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: Space.s3,
        runSpacing: Space.s2,
        children: pageActionButtons(context, actions, iconOnly: iconOnly),
      ),
    );
  }
}
