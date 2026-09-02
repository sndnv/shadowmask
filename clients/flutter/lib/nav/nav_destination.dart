import 'package:flutter/widgets.dart';

import 'package:shadowmask/nav/nav_section.dart';

class NavDestination {
  const NavDestination(
    this.section,
    this.label,
    this.route, {
    required this.icon,
    required this.selectedIcon,
    this.adminOnly = false,
    this.primary = false,
  });

  final NavSection section;
  final String label;
  final String route;
  final IconData icon;
  final IconData selectedIcon;
  final bool adminOnly;
  final bool primary;
}
