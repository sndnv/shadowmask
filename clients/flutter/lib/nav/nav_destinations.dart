import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_section.dart';

const int kMaxBottomTabs = 5;

const List<NavDestination> navDestinations = <NavDestination>[
  NavDestination(
    NavSection.home,
    Strings.navigationHome,
    '/home',
    icon: Icons.home_outlined,
    selectedIcon: Icons.home,
    primary: true,
  ),
  NavDestination(
    NavSection.movies,
    Strings.navigationMovies,
    '/movies',
    icon: Icons.movie_outlined,
    selectedIcon: Icons.movie,
    primary: true,
  ),
  NavDestination(
    NavSection.series,
    Strings.navigationSeries,
    '/series',
    icon: Icons.tv_outlined,
    selectedIcon: Icons.tv,
    primary: true,
  ),
  NavDestination(
    NavSection.collections,
    Strings.navigationCollections,
    '/collections',
    icon: Icons.collections_bookmark_outlined,
    selectedIcon: Icons.collections_bookmark,
  ),
  NavDestination(
    NavSection.search,
    Strings.navigationSearch,
    '/search',
    icon: Icons.search_outlined,
    selectedIcon: Icons.search,
    primary: true,
  ),
  NavDestination(
    NavSection.admin,
    Strings.navigationAdmin,
    '/admin',
    icon: Icons.tune_outlined,
    selectedIcon: Icons.tune,
    adminOnly: true,
  ),
];

Iterable<NavDestination> destinationsFor(SelfUser? user) => navDestinations
    .where((NavDestination d) => !d.adminOnly || (user?.isAdmin ?? false));

({List<NavDestination> tabs, List<NavDestination> overflow}) bottomNavSplit(
  SelfUser? user,
) {
  final List<NavDestination> all = destinationsFor(user).toList();
  final List<NavDestination> tabs = all
      .where((NavDestination d) => d.primary)
      .take(kMaxBottomTabs - 1)
      .toList();
  final Set<NavSection> taken = tabs
      .map((NavDestination d) => d.section)
      .toSet();
  return (
    tabs: tabs,
    overflow: all
        .where((NavDestination d) => !taken.contains(d.section))
        .toList(),
  );
}
