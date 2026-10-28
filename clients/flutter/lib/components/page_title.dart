import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/page_director.dart';

class PageTitleScope extends InheritedWidget {
  const PageTitleScope({super.key, required this.titles, required super.child});

  final PageDirector<String?> titles;

  static PageDirector<String?>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<PageTitleScope>()?.titles;

  @override
  bool updateShouldNotify(PageTitleScope oldWidget) =>
      titles != oldWidget.titles;
}

class TabTitle extends StatefulWidget {
  const TabTitle({super.key, required this.titles, required this.child});

  final PageDirector<String?> titles;
  final Widget child;

  @override
  State<TabTitle> createState() => _TabTitleState();
}

class _TabTitleState extends State<TabTitle> {
  Color? _color;

  @override
  void initState() {
    super.initState();
    widget.titles.addListener(_name);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final Color color = context.tokens.accent;
    if (color != _color) {
      _color = color;
      _name();
    }
  }

  @override
  void didUpdateWidget(TabTitle oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.titles != widget.titles) {
      oldWidget.titles.removeListener(_name);
      widget.titles.addListener(_name);
      _name();
    }
  }

  @override
  void dispose() {
    widget.titles.removeListener(_name);
    super.dispose();
  }

  void _name() {
    final String? label = widget.titles.shown;
    final Color? color = _color;
    if (label == null || color == null) {
      return;
    }
    SystemChrome.setApplicationSwitcherDescription(
      ApplicationSwitcherDescription(
        label: label,
        primaryColor: color.toARGB32(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
