import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kBottomNavHeight = 64;

class ShellBottomNav extends StatelessWidget {
  const ShellBottomNav({
    super.key,
    required this.api,
    required this.current,
    this.user,
  });

  final ApiClient api;
  final NavSection current;
  final SelfUser? user;

  void _go(BuildContext context, String route) =>
      Navigator.of(context).pushReplacementNamed(route);

  Future<void> _signOut(BuildContext context) async {
    await api.logout();
    if (context.mounted) {
      Navigator.of(
        context,
      ).pushNamedAndRemoveUntil('/', (Route<dynamic> r) => false);
    }
  }

  Future<void> _openMore(
    BuildContext context,
    List<NavDestination> overflow,
  ) async {
    final Tokens t = context.tokens;
    final _MoreChoice? picked = await showModalBottomSheet<_MoreChoice>(
      context: context,
      backgroundColor: t.surface,
      showDragHandle: true,
      builder: (BuildContext sheet) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (final NavDestination d in overflow)
              _MoreTile(
                icon: d.section == current ? d.selectedIcon : d.icon,
                label: d.label,
                accent: d.adminOnly,
                selected: d.section == current,
                onTap: () =>
                    Navigator.of(sheet).pop(_MoreChoice.route(d.route)),
              ),
            if (user != null) ...<Widget>[
              _MoreTile(
                icon: current == NavSection.account
                    ? Icons.person
                    : Icons.person_outline,
                label: user!.username,
                selected: current == NavSection.account,
                onTap: () => Navigator.of(
                  sheet,
                ).pop(const _MoreChoice.route('/account')),
              ),
              Divider(color: t.border, height: 1),
              _MoreTile(
                icon: Icons.logout,
                label: Strings.signOut,
                selected: false,
                onTap: () =>
                    Navigator.of(sheet).pop(const _MoreChoice.signOut()),
              ),
            ],
            const SizedBox(height: Space.s2),
          ],
        ),
      ),
    );
    if (picked == null || !context.mounted) {
      return;
    }
    if (picked.route != null) {
      _go(context, picked.route!);
      return;
    }
    await _signOut(context);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final ({List<NavDestination> tabs, List<NavDestination> overflow}) split =
        bottomNavSplit(user);
    final int active = split.tabs.indexWhere(
      (NavDestination d) => d.section == current,
    );
    final int more = split.tabs.length;

    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(top: BorderSide(color: t.border)),
      ),
      child: NavigationBarTheme(
        data: NavigationBarThemeData(
          labelTextStyle: WidgetStateProperty.resolveWith<TextStyle>(
            (Set<WidgetState> states) => TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: states.contains(WidgetState.selected) ? t.text : t.muted,
            ),
          ),
        ),
        child: NavigationBar(
          height: kBottomNavHeight,
          backgroundColor: t.surface,
          surfaceTintColor: Colors.transparent,
          indicatorColor: t.accent,
          indicatorShape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.all(Radii.sm),
          ),
          overlayColor: WidgetStatePropertyAll<Color>(t.rowHover),
          labelBehavior: NavigationDestinationLabelBehavior.alwaysShow,
          selectedIndex: active >= 0 ? active : more,
          onDestinationSelected: (int index) => index < split.tabs.length
              ? _go(context, split.tabs[index].route)
              : _openMore(context, split.overflow),
          destinations: <Widget>[
            for (final NavDestination d in split.tabs)
              NavigationDestination(
                icon: Icon(d.icon, color: t.muted),
                selectedIcon: Icon(d.selectedIcon, color: t.accentContrast),
                label: d.label,
              ),
            NavigationDestination(
              icon: Icon(Icons.more_horiz, color: t.muted),
              selectedIcon: Icon(Icons.more_horiz, color: t.accentContrast),
              label: Strings.navigationMore,
            ),
          ],
        ),
      ),
    );
  }
}

class _MoreChoice {
  const _MoreChoice.route(this.route);
  const _MoreChoice.signOut() : route = null;

  final String? route;
}

class _MoreTile extends StatelessWidget {
  const _MoreTile({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
    this.accent = false,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;
  final bool accent;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return ListTile(
      leading: Icon(icon, color: selected ? t.accent : t.muted),
      title: Text(
        label,
        style: TextStyle(
          color: accent ? t.accent : t.text,
          fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
        ),
      ),
      selected: selected,
      onTap: onTap,
    );
  }
}
