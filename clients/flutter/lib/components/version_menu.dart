import 'package:flutter/material.dart';

import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/version_order.dart';

class VersionMenu extends StatelessWidget {
  const VersionMenu({
    super.key,
    required this.controller,
    required this.ordered,
    required this.onPlay,
    required this.child,
  });

  final MenuController controller;
  final List<Version> ordered;
  final ValueChanged<Version> onPlay;
  final Widget child;

  static void toggle(MenuController controller) =>
      controller.isOpen ? controller.close() : controller.open();

  @override
  Widget build(BuildContext context) => MenuAnchor(
    controller: controller,
    alignmentOffset: kMenuOffset,
    style: appMenuStyle(context.tokens),
    menuChildren: <Widget>[
      for (int i = 0; i < ordered.length; i++)
        if (ordered[i].available)
          VersionMenuItem(
            version: ordered[i],
            number: versionNumberLabel(i),
            onTap: () => onPlay(ordered[i]),
          ),
    ],
    child: child,
  );
}

class VersionMenuItem extends StatelessWidget {
  const VersionMenuItem({
    super.key,
    required this.version,
    required this.number,
    required this.onTap,
  });

  final Version version;
  final String number;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuItemButton(
      onPressed: onTap,
      leadingIcon: Icon(Icons.play_arrow, size: 18, color: t.accent),
      child: Text.rich(
        TextSpan(
          style: monoStyle.copyWith(color: t.text, fontSize: 13),
          children: <InlineSpan>[
            TextSpan(
              text: '$number · ',
              style: TextStyle(color: t.muted),
            ),
            TextSpan(
              text: version.quality.label,
              style: TextStyle(color: t.accent),
            ),
            TextSpan(text: ' · ${version.container}'),
          ],
        ),
      ),
    );
  }
}
