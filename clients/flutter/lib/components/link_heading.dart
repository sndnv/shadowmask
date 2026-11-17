import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/theme/shapes_context.dart';
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

  Widget _tappable(String route, Widget child) => MouseRegion(
    onEnter: (_) => setState(() => _hovered = true),
    onExit: (_) => setState(() => _hovered = false),
    child: HoverTap(
      onTap: () => Navigator.of(context).pushNamed(route),
      focusRing: false,
      onFocusChange: (bool focused) => setState(() => _focused = focused),
      child: child,
    ),
  );

  Widget _tab() {
    final String? label = widget.linkLabel;
    final String? route = widget.route;
    final bool linked = label != null && route != null;
    final Widget tab = SlantedTab(
      widget.title,
      link: linked ? label : null,
      lit: _hovered || _focused,
    );
    return TabRule(
      child: Align(
        alignment: Alignment.bottomLeft,
        child: linked ? _tappable(route, tab) : tab,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (context.shapes.cut) {
      return _tab();
    }
    final TextStyle? style = Theme.of(context).textTheme.headlineMedium;
    final String? label = widget.linkLabel;
    final String? route = widget.route;
    if (label == null || route == null) {
      return Text(widget.title, style: style);
    }
    final Tokens t = context.tokens;
    final bool lit = _hovered || _focused;
    return _tappable(
      route,
      Text.rich(
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
    );
  }
}
