import 'package:flutter/material.dart';

import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class LinkHeading extends StatefulWidget {
  const LinkHeading({
    super.key,
    required this.title,
    this.linkLabel,
    this.route,
  });

  final String title;
  final String? linkLabel;
  final String? route;

  @override
  State<LinkHeading> createState() => _LinkHeadingState();
}

class _LinkHeadingState extends State<LinkHeading> {
  bool _hovered = false;
  bool _focused = false;

  @override
  Widget build(BuildContext context) {
    final TextStyle? style = Theme.of(context).textTheme.headlineMedium;
    final String? label = widget.linkLabel;
    final String? route = widget.route;
    if (label == null || route == null) {
      return Text(widget.title, style: style);
    }
    final Tokens t = context.tokens;
    final bool lit = _hovered || _focused;
    return MouseRegion(
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: HoverTap(
        onTap: () => Navigator.of(context).pushNamed(route),
        focusRing: false,
        onFocusChange: (bool focused) => setState(() => _focused = focused),
        child: Text.rich(
          TextSpan(
            style: style,
            children: <InlineSpan>[
              TextSpan(text: widget.title),
              TextSpan(
                text: label,
                style: TextStyle(
                  decoration: lit ? TextDecoration.underline : null,
                  decorationColor: t.accent,
                  color: lit ? t.accent : null,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
