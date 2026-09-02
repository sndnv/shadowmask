import 'package:flutter/material.dart';

import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class BackLink extends StatefulWidget {
  const BackLink({super.key});

  @override
  State<BackLink> createState() => _BackLinkState();
}

class _BackLinkState extends State<BackLink> {
  bool _hovered = false;
  bool _focused = false;

  @override
  Widget build(BuildContext context) {
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (isRootRoute(route?.settings.name) || !Navigator.of(context).canPop()) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Tooltip(
          message: Strings.back,
          child: MouseRegion(
            onEnter: (_) => setState(() => _hovered = true),
            onExit: (_) => setState(() => _hovered = false),
            child: HoverTap(
              onTap: () => Navigator.of(context).pop(),
              semanticsLabel: Strings.back,
              focusRing: false,
              onFocusChange: (bool focused) =>
                  setState(() => _focused = focused),
              child: Icon(
                Icons.arrow_back,
                size: 16,
                color: _hovered || _focused ? t.accent : t.muted,
              ),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: Space.s2),
          child: Text(
            '|',
            style: TextStyle(
              fontSize: 13,
              height: 1.3,
              color: t.muted.withValues(alpha: 0.4),
            ),
          ),
        ),
      ],
    );
  }
}
