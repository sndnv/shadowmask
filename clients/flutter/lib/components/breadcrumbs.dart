import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter/material.dart';

import 'package:shadowmask/components/back_link.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/crumb_strip.dart';
import 'package:shadowmask/components/crumbs_scope.dart';
import 'package:shadowmask/components/page_title.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/util/scoped_value.dart';

class Breadcrumbs extends StatefulWidget {
  const Breadcrumbs(this.crumbs, {super.key});

  final List<Crumb> crumbs;

  @override
  State<Breadcrumbs> createState() => _BreadcrumbsState();
}

class _BreadcrumbsState extends State<Breadcrumbs> {
  List<Crumb> get _all =>
      widget.crumbs.isEmpty ||
          widget.crumbs.first.label == Strings.navigationHome
      ? widget.crumbs
      : <Crumb>[
          Crumb(Strings.navigationHome, route: homeRoute()),
          ...widget.crumbs,
        ];

  void _publishTitle(BuildContext context, List<Crumb> all) {
    final ScopedValue<String?>? sink = PageTitleScope.maybeOf(context);
    final String? label = all.isEmpty ? null : all.last.label;
    if (sink == null || sink.value == label) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        sink.publish(label);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final List<Crumb> all = _all;
    _publishTitle(context, all);
    final ValueNotifier<List<Crumb>>? sink = CrumbsScope.maybeOf(context);
    if (sink == null) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 12),
        child: Row(
          children: <Widget>[
            const BackLink(),
            Flexible(child: CrumbStrip(all)),
          ],
        ),
      );
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && !listEquals(sink.value, all)) {
        sink.value = all;
      }
    });
    return const SizedBox.shrink();
  }
}
