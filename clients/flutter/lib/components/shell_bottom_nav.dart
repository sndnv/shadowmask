import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/confirm_sign_out.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/open_section.dart';
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

  void _go(BuildContext context, String route) => openSection(context, route);

  Future<void> _signOut(BuildContext context) async {
    if (!await confirmSignOut(context) || !context.mounted) {
      return;
    }
    await AccountApi(api).signOutThisDevice(user?.id);
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
            if (user != null)
              _AccountCard(
                username: user!.username,
                selected: current == NavSection.account,
                onOpen: () => Navigator.of(
                  sheet,
                ).pop(const _MoreChoice.route('/account')),
                onSignOut: () =>
                    Navigator.of(sheet).pop(const _MoreChoice.signOut()),
              ),
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

class _AccountCard extends StatelessWidget {
  const _AccountCard({
    required this.username,
    required this.selected,
    required this.onOpen,
    required this.onSignOut,
  });

  final String username;
  final bool selected;
  final VoidCallback onOpen;
  final VoidCallback onSignOut;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.s4, Space.s3, Space.s4, 0),
      child: Material(
        color: t.surfaceAlt,
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(
          borderRadius: const BorderRadius.all(Radii.md),
          side: BorderSide(color: selected ? t.accent : t.border),
        ),
        child: ListTile(
          onTap: onOpen,
          selected: selected,
          leading: CircleAvatar(
            backgroundColor: t.accent,
            foregroundColor: t.accentContrast,
            child: Text(
              username.characters.take(1).toUpperCase().toString(),
              style: const TextStyle(fontWeight: FontWeight.w700),
            ),
          ),
          title: Text(
            username,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: t.text, fontWeight: FontWeight.w700),
          ),
          subtitle: Text(
            Strings.navigationAccount,
            style: TextStyle(color: t.muted),
          ),
          trailing: IconButton(
            onPressed: onSignOut,
            icon: const Icon(Icons.logout),
            tooltip: Strings.signOut,
            color: t.muted,
          ),
        ),
      ),
    );
  }
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
