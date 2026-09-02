import 'package:flutter/material.dart';

import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const int _kTailCrumbs = 2;

class CrumbStrip extends StatelessWidget {
  const CrumbStrip(this.crumbs, {super.key});

  final List<Crumb> crumbs;

  static void _reload(BuildContext context) {
    final String? name = ModalRoute.of(context)?.settings.name;
    if (name != null) {
      Navigator.of(context).pushReplacementNamed(name);
    }
  }

  bool _collapses(double available) =>
      crumbs.length > _kTailCrumbs + 1 && available < Breakpoints.sm;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DefaultTextStyle.merge(
      style: const TextStyle(fontSize: 13, height: 1.3),
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) => Row(
          mainAxisSize: MainAxisSize.min,
          children: _collapses(constraints.maxWidth)
              ? _collapsed(context, t)
              : _full(context, t),
        ),
      ),
    );
  }

  List<Widget> _full(BuildContext context, Tokens t) {
    final List<Widget> children = <Widget>[];
    for (int i = 0; i < crumbs.length; i++) {
      children.add(
        _crumb(context, t, crumbs[i], isLast: i == crumbs.length - 1),
      );
      if (i != crumbs.length - 1) {
        children.add(_separator(t));
      }
    }
    return children;
  }

  List<Widget> _collapsed(BuildContext context, Tokens t) {
    final List<Crumb> hidden = crumbs.sublist(1, crumbs.length - _kTailCrumbs);
    final List<Widget> children = <Widget>[
      _crumb(context, t, crumbs.first, isLast: false),
      _separator(t),
      _HiddenCrumbs(hidden: hidden),
      _separator(t),
    ];
    for (int i = crumbs.length - _kTailCrumbs; i < crumbs.length; i++) {
      children.add(
        _crumb(context, t, crumbs[i], isLast: i == crumbs.length - 1),
      );
      if (i != crumbs.length - 1) {
        children.add(_separator(t));
      }
    }
    return children;
  }

  Widget _crumb(
    BuildContext context,
    Tokens t,
    Crumb c, {
    required bool isLast,
  }) {
    final bool isLink = c.route != null && !isLast;
    final Widget label = Text(
      c.label,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: TextStyle(
        color: isLink ? t.accent : t.muted,
        fontWeight: isLink ? FontWeight.w600 : FontWeight.w400,
      ),
    );
    if (isLink) {
      return InkWell(
        onTap: () => Navigator.of(context).pushNamed(c.route!),
        child: label,
      );
    }
    return Flexible(
      child: Tooltip(
        message: Strings.refreshThisPage,
        child: InkWell(onTap: () => _reload(context), child: label),
      ),
    );
  }

  Widget _separator(Tokens t) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 8),
    child: Text('›', style: TextStyle(color: t.muted.withValues(alpha: 0.6))),
  );
}

class _HiddenCrumbs extends StatefulWidget {
  const _HiddenCrumbs({required this.hidden});

  final List<Crumb> hidden;

  @override
  State<_HiddenCrumbs> createState() => _HiddenCrumbsState();
}

class _HiddenCrumbsState extends State<_HiddenCrumbs> {
  final MenuController _controller = MenuController();

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return MenuAnchor(
      controller: _controller,
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: <Widget>[
        for (final Crumb c in widget.hidden)
          MenuOption(
            label: c.label,
            leading: const SizedBox.shrink(),
            onTap: () {
              _controller.close();
              final String? route = c.route;
              if (route != null) {
                Navigator.of(context).pushNamed(route);
              }
            },
          ),
      ],
      builder: (BuildContext context, MenuController controller, Widget? _) {
        return Tooltip(
          message: Strings.hiddenLevels,
          child: InkWell(
            onTap: () =>
                controller.isOpen ? controller.close() : controller.open(),
            child: Semantics(
              button: true,
              label: Strings.hiddenLevels,
              child: Text(
                '…',
                style: TextStyle(color: t.accent, fontWeight: FontWeight.w600),
              ),
            ),
          ),
        );
      },
    );
  }
}
