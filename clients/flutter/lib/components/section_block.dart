import 'package:flutter/material.dart';

import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class SectionHeaderHeight extends InheritedWidget {
  const SectionHeaderHeight({
    super.key,
    required this.minHeight,
    required super.child,
  });

  final double minHeight;

  static double of(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<SectionHeaderHeight>()
          ?.minHeight ??
      0;

  @override
  bool updateShouldNotify(SectionHeaderHeight old) =>
      old.minHeight != minHeight;
}

class SectionBlock extends StatelessWidget {
  const SectionBlock({
    super.key,
    required this.title,
    required this.child,
    this.actions = const <Widget>[],
    this.actionItems = const <PageAction>[],
    this.framed = true,
  });

  final String title;
  final Widget child;
  final List<Widget> actions;
  final List<PageAction> actionItems;
  final bool framed;

  bool get _hasActions => actions.isNotEmpty || actionItems.isNotEmpty;

  Widget _header(BuildContext context, double width, bool narrow) => Wrap(
    alignment: WrapAlignment.spaceBetween,
    crossAxisAlignment: WrapCrossAlignment.center,
    runAlignment: WrapAlignment.center,
    spacing: Space.s3,
    runSpacing: Space.s2,
    children: <Widget>[
      ConstrainedBox(
        constraints: BoxConstraints(maxWidth: width),
        child: Text(
          title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
      ),
      if (_hasActions)
        Wrap(
          spacing: Space.s2,
          runSpacing: Space.s2,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: <Widget>[
            ...actions,
            for (final PageAction action in actionItems)
              pageActionButton(context, action, iconOnly: narrow),
          ],
        ),
    ],
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: SectionHeaderHeight.of(context),
          ),
          child: LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) =>
                SizedBox(
                  width: constraints.maxWidth,
                  child: _header(
                    context,
                    constraints.maxWidth,
                    constraints.maxWidth < Breakpoints.sm,
                  ),
                ),
          ),
        ),
        const SizedBox(height: Space.s3),
        if (framed)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(Space.s4),
            decoration: BoxDecoration(
              color: t.surface,
              borderRadius: const BorderRadius.all(Radii.md),
              border: Border.all(color: t.border),
            ),
            child: child,
          )
        else
          SizedBox(width: double.infinity, child: child),
        const SizedBox(height: Space.s6),
      ],
    );
  }
}
