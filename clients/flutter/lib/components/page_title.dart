import 'package:flutter/widgets.dart';

import 'package:shadowmask/util/scoped_value.dart';

class PageTitleScope extends InheritedWidget {
  const PageTitleScope({super.key, required this.title, required super.child});

  final ScopedValue<String?> title;

  static ScopedValue<String?>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<PageTitleScope>()?.title;

  @override
  bool updateShouldNotify(PageTitleScope oldWidget) => title != oldWidget.title;
}
