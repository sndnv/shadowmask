import 'package:flutter/material.dart';

import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class TitleAction {
  const TitleAction({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
    this.danger = false,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;
  final bool danger;
}

class TitleHeading extends StatelessWidget {
  const TitleHeading({
    super.key,
    required this.title,
    this.pager = const <TitleAction>[],
    this.actions = const <TitleAction>[],
    this.trailing,
  });

  final String title;
  final List<TitleAction> pager;
  final List<TitleAction> actions;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget heading = Text(
      title,
      style: Theme.of(context).textTheme.headlineLarge,
    );
    if (pager.isEmpty && actions.isEmpty && trailing == null) {
      return heading;
    }
    Widget button(TitleAction action) => IconButton(
      tooltip: action.tooltip,
      onPressed: action.onPressed,
      visualDensity: VisualDensity.compact,
      icon: Icon(
        action.icon,
        size: 18,
        color: action.danger && action.onPressed != null ? t.danger : t.muted,
      ),
    );
    final List<Widget> controls = <Widget>[
      for (final TitleAction action in pager) button(action),
      if (pager.isNotEmpty && actions.isNotEmpty)
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4),
          child: Text(
            '|',
            style: TextStyle(color: t.muted.withValues(alpha: 0.5)),
          ),
        ),
      for (final TitleAction action in actions) button(action),
      ?trailing,
    ];
    final bool inlineable =
        pager.isEmpty && actions.length == 1 && trailing == null;
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < Breakpoints.sm) {
          if (inlineable) {
            return Text.rich(
              TextSpan(
                children: <InlineSpan>[
                  TextSpan(text: title),
                  WidgetSpan(
                    alignment: PlaceholderAlignment.middle,
                    child: button(actions.single),
                  ),
                ],
              ),
              style: Theme.of(context).textTheme.headlineLarge,
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              heading,
              const SizedBox(height: Space.s2),
              Wrap(
                crossAxisAlignment: WrapCrossAlignment.center,
                children: controls,
              ),
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Flexible(child: heading),
            const SizedBox(width: Space.s2),
            ...controls,
          ],
        );
      },
    );
  }
}
