import 'package:flutter/widgets.dart';

import 'package:shadowmask/components/crumb.dart';

class CrumbsScope extends InheritedWidget {
  const CrumbsScope({super.key, required this.crumbs, required super.child});

  final ValueNotifier<List<Crumb>> crumbs;

  static ValueNotifier<List<Crumb>>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<CrumbsScope>()?.crumbs;

  @override
  bool updateShouldNotify(CrumbsScope oldWidget) => crumbs != oldWidget.crumbs;
}
