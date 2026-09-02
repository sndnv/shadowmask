import 'package:flutter/material.dart';

import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';

class CardRail extends StatefulWidget {
  const CardRail({
    super.key,
    required this.title,
    required this.cards,
    required this.imageBase,
    this.onDismiss,
    this.dismissBusy,
    this.dismissTooltip = Strings.dismiss,
    this.dismissesByTitle = false,
    this.cardWidth,
    this.titleLink,
    this.titleRoute,
  });

  final String title;
  final List<CatalogCard> cards;
  final String imageBase;
  final void Function(CatalogCard card)? onDismiss;
  final bool Function(CatalogCard card)? dismissBusy;
  final String dismissTooltip;
  final bool dismissesByTitle;
  final double? cardWidth;
  final String? titleLink;
  final String? titleRoute;

  @override
  State<CardRail> createState() => _CardRailState();
}

class _CardRailState extends State<CardRail> {
  final ScrollController _controller = ScrollController();
  bool _canLeft = false;
  bool _canRight = false;

  bool _dismissible(CatalogCard card) =>
      widget.onDismiss != null &&
      (widget.dismissesByTitle || card.dismissVersionId != null);

  @override
  void initState() {
    super.initState();
    _controller.addListener(_update);
    WidgetsBinding.instance.addPostFrameCallback((_) => _update());
  }

  @override
  void didUpdateWidget(CardRail oldWidget) {
    super.didUpdateWidget(oldWidget);
    WidgetsBinding.instance.addPostFrameCallback((_) => _update());
  }

  @override
  void dispose() {
    _controller.removeListener(_update);
    _controller.dispose();
    super.dispose();
  }

  void _update() {
    if (!mounted || !_controller.hasClients) {
      return;
    }
    final ScrollPosition p = _controller.position;
    final bool left = p.pixels > p.minScrollExtent + 1;
    final bool right = p.pixels < p.maxScrollExtent - 1;
    if (left != _canLeft || right != _canRight) {
      setState(() {
        _canLeft = left;
        _canRight = right;
      });
    }
  }

  void _nudge(double sign) {
    if (!_controller.hasClients) {
      return;
    }
    final ScrollPosition p = _controller.position;
    final double target = (p.pixels + sign * p.viewportDimension * 0.8).clamp(
      p.minScrollExtent,
      p.maxScrollExtent,
    );
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.jumpTo(target);
      return;
    }
    _controller.animateTo(
      target,
      duration: const Duration(milliseconds: 240),
      curve: Curves.easeOut,
    );
  }

  @override
  Widget build(BuildContext context) {
    final CardAspect aspect = aspectOf(widget.cards);
    final bool landscape = aspect == CardAspect.landscape;
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double w = fittedCardWidth(
          constraints.maxWidth,
          aspect,
          target: widget.cardWidth,
        );
        final double artHeight = landscape ? w * 9 / 16 : w * 3 / 2;
        final double railHeight = artHeight + kCardTextBlockHeight;
        return _body(w, artHeight, railHeight, aspect);
      },
    );
  }

  Widget _body(
    double w,
    double artHeight,
    double railHeight,
    CardAspect aspect,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _Heading(
          title: widget.title,
          linkLabel: widget.titleLink,
          route: widget.titleRoute,
        ),
        const SizedBox(height: Space.s3),
        SizedBox(
          height: railHeight,
          child: NotificationListener<ScrollMetricsNotification>(
            onNotification: (ScrollMetricsNotification _) {
              _update();
              return false;
            },
            child: Stack(
              children: <Widget>[
                ListView.separated(
                  controller: _controller,
                  scrollDirection: Axis.horizontal,
                  itemCount: widget.cards.length,
                  separatorBuilder: (BuildContext _, int _) =>
                      const SizedBox(width: Space.s4),
                  itemBuilder: (BuildContext _, int i) => CatalogCardTile(
                    card: widget.cards[i],
                    imageBase: widget.imageBase,
                    width: w,
                    aspect: aspect,
                    onDismiss: _dismissible(widget.cards[i])
                        ? widget.onDismiss
                        : null,
                    dismissBusy:
                        widget.dismissBusy?.call(widget.cards[i]) ?? false,
                    dismissTooltip: widget.dismissTooltip,
                  ),
                ),
                if (_canLeft)
                  _RailArrow(
                    height: artHeight,
                    alignment: Alignment.centerLeft,
                    icon: Icons.chevron_left,
                    tooltip: Strings.previous,
                    onTap: () => _nudge(-1),
                  ),
                if (_canRight)
                  _RailArrow(
                    height: artHeight,
                    alignment: Alignment.centerRight,
                    icon: Icons.chevron_right,
                    tooltip: Strings.next,
                    onTap: () => _nudge(1),
                  ),
              ],
            ),
          ),
        ),
        const SizedBox(height: Space.s5),
      ],
    );
  }
}

class _RailArrow extends StatelessWidget {
  const _RailArrow({
    required this.height,
    required this.alignment,
    required this.icon,
    required this.tooltip,
    required this.onTap,
  });

  final double height;
  final Alignment alignment;
  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Positioned(
      top: 0,
      height: height,
      left: alignment == Alignment.centerLeft ? Space.s1 : null,
      right: alignment == Alignment.centerRight ? Space.s1 : null,
      child: Center(
        child: Material(
          color: t.surface.withValues(alpha: 0.9),
          shape: CircleBorder(side: BorderSide(color: t.border)),
          clipBehavior: Clip.antiAlias,
          child: Tooltip(
            message: tooltip,
            child: InkWell(
              onTap: onTap,
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(icon, size: 22, color: t.text),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Heading extends StatefulWidget {
  const _Heading({
    required this.title,
    required this.linkLabel,
    required this.route,
  });

  final String title;
  final String? linkLabel;
  final String? route;

  @override
  State<_Heading> createState() => _HeadingState();
}

class _HeadingState extends State<_Heading> {
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
