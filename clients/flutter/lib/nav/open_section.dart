import 'package:flutter/widgets.dart';

import 'package:shadowmask/nav/browser_history.dart';
import 'package:shadowmask/nav/nav_destination.dart';
import 'package:shadowmask/nav/nav_destinations.dart';
import 'package:shadowmask/nav/routes.dart';

void openSection(BuildContext context, String route) {
  final String home = homeRoute();
  Navigator.of(context).pushNamedAndRemoveUntil(
    route,
    (Route<dynamic> below) => route != home && below.settings.name == home,
  );
}

void openCrumb(BuildContext context, String route) {
  if (route == homeRoute()) {
    openSection(context, route);
    return;
  }
  final Uri target = Uri.parse(route);
  final Object? delegate = Router.maybeOf(context)?.routerDelegate;
  final Iterable<Uri> open = delegate is BrowserHistory
      ? delegate.pages.reversed.where((Uri page) => samePage(page, target))
      : const <Uri>[];
  if (open.isNotEmpty) {
    _reopen(Navigator.of(context), open.first);
  } else if (_isSection(route)) {
    openSection(context, route);
  } else {
    Navigator.of(context).pushNamed(route);
  }
}

void _reopen(NavigatorState navigator, Uri page) {
  bool passed = false;
  navigator.pushNamedAndRemoveUntil(page.toString(), (Route<dynamic> below) {
    if (passed) {
      return true;
    }
    final String? name = below.settings.name;
    passed = name != null && samePage(Uri.parse(name), page);
    return false;
  });
}

bool _isSection(String route) =>
    route == '/account' ||
    navDestinations.any((NavDestination section) => section.route == route);
