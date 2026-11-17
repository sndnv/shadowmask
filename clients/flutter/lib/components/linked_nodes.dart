import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kNodeHeight = 36;
const double kNodeGap = Space.s2;
const double kLinkWidth = 48;
const double kLinkStroke = 1.2;
const Size kLinkSource = Size(8, 7);

enum NodeRole { opens, choice, plain }

double nodeCentre(int index) =>
    index * (kNodeHeight + kNodeGap) + kNodeHeight / 2;

double nodesHeight(int rows) =>
    rows <= 0 ? 0 : rows * (kNodeHeight + kNodeGap) - kNodeGap;

int rightStart({required int from, required int left, required int right}) =>
    math.min(from, math.max(0, left - right));

const Map<ShortcutActivator, Intent> _arrows = <ShortcutActivator, Intent>{
  SingleActivator(LogicalKeyboardKey.arrowUp): DirectionalFocusIntent(
    TraversalDirection.up,
  ),
  SingleActivator(LogicalKeyboardKey.arrowDown): DirectionalFocusIntent(
    TraversalDirection.down,
  ),
  SingleActivator(LogicalKeyboardKey.arrowLeft): DirectionalFocusIntent(
    TraversalDirection.left,
  ),
  SingleActivator(LogicalKeyboardKey.arrowRight): DirectionalFocusIntent(
    TraversalDirection.right,
  ),
};

class LinkedNodes extends StatefulWidget {
  const LinkedNodes({
    super.key,
    required this.left,
    required this.right,
    required this.from,
    required this.lit,
    this.rightLabel,
    this.rightEnabled = true,
  });

  final List<Widget> left;
  final List<Widget> right;
  final int from;
  final int lit;
  final String? rightLabel;
  final bool rightEnabled;

  @override
  State<LinkedNodes> createState() => _LinkedNodesState();
}

class _LinkedNodesState extends State<LinkedNodes> {
  final FocusScopeNode _scope = FocusScopeNode(
    debugLabel: 'LinkedNodes',
    traversalEdgeBehavior: TraversalEdgeBehavior.parentScope,
  );
  final FocusTraversalPolicy _policy = ReadingOrderTraversalPolicy();

  @override
  void dispose() {
    _scope.dispose();
    super.dispose();
  }

  Widget _column(List<Widget> nodes, {String? label}) => FocusTraversalGroup(
    policy: _policy,
    child: Semantics(
      container: true,
      explicitChildNodes: true,
      label: label,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: kNodeGap,
        children: <Widget>[
          for (final Widget node in nodes)
            SizedBox(height: kNodeHeight, child: node),
        ],
      ),
    ),
  );

  Widget _quiet(Widget child) => AbsorbPointer(
    absorbing: !widget.rightEnabled,
    child: Opacity(opacity: widget.rightEnabled ? 1 : 0.5, child: child),
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final int start = rightStart(
      from: widget.from,
      left: widget.left.length,
      right: widget.right.length,
    );
    return Shortcuts(
      shortcuts: _arrows,
      child: FocusScope(
        node: _scope,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(child: _column(widget.left)),
            _quiet(
              SizedBox(
                width: kLinkWidth,
                height: nodesHeight(
                  math.max(widget.left.length, start + widget.right.length),
                ),
                child: CustomPaint(
                  painter: LinkPainter(
                    from: widget.from,
                    start: start,
                    count: widget.right.length,
                    lit: widget.lit,
                    link: shapes.lineSoft(t.border),
                    dot: shapes.line(t.border),
                    bright: t.accent,
                  ),
                ),
              ),
            ),
            Expanded(
              child: _quiet(
                Padding(
                  padding: EdgeInsets.only(
                    top: start * (kNodeHeight + kNodeGap),
                  ),
                  child: _column(widget.right, label: widget.rightLabel),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class NodeCaption extends StatelessWidget {
  const NodeCaption(this.label, {super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    return Semantics(
      header: true,
      child: Align(
        alignment: Alignment.bottomLeft,
        child: Padding(
          padding: const EdgeInsets.only(bottom: Space.s1),
          child: Text(
            shapes.caps(label),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: shapes.mono(
              TextStyle(
                color: t.muted,
                fontSize: 11,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class NodeItem extends StatelessWidget {
  const NodeItem({
    super.key,
    required this.label,
    required this.selected,
    required this.onTap,
    this.value,
    this.icon,
    this.help,
    this.selectedColor,
    this.role = NodeRole.choice,
    this.enabled = true,
  });

  final String label;
  final bool selected;
  final VoidCallback? onTap;
  final String? value;
  final IconData? icon;
  final Widget? help;
  final Color? selectedColor;
  final NodeRole role;
  final bool enabled;

  Widget? _trailing(Tokens t, Shapes shapes) {
    final String? shown = value;
    if (shown != null) {
      return Text(
        shown,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        textAlign: TextAlign.right,
        style: shapes.mono(TextStyle(color: t.muted, fontSize: 11)),
      );
    }
    final IconData? glyph = icon;
    return glyph == null
        ? null
        : Align(
            alignment: Alignment.centerRight,
            child: Icon(glyph, size: 14, color: t.muted),
          );
  }

  Widget _content(Tokens t, Shapes shapes) {
    final Widget name = Text(
      label,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: TextStyle(
        color: selected ? selectedColor ?? t.text : t.muted,
        fontSize: 13,
        fontWeight: FontWeight.w600,
      ),
    );
    final Widget? help = this.help;
    final Widget? trailing = _trailing(t, shapes);
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints box) => Row(
        children: <Widget>[
          ConstrainedBox(
            constraints: BoxConstraints(
              maxWidth: math.max(
                0,
                box.maxWidth - (trailing == null ? 0 : Space.s2),
              ),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Flexible(child: name),
                ?help,
              ],
            ),
          ),
          if (trailing != null) ...<Widget>[
            const SizedBox(width: Space.s2),
            Expanded(child: trailing),
          ],
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final VoidCallback? tap = onTap;
    final Color? still = enabled ? null : Colors.transparent;
    return Semantics(
      button: tap != null,
      enabled: tap == null ? null : enabled,
      selected: role == NodeRole.choice ? selected : null,
      expanded: role == NodeRole.opens ? selected : null,
      child: Opacity(
        opacity: enabled ? 1 : 0.5,
        child: Material(
          color: selected ? shapes.wash(t.surface) : t.surface,
          clipBehavior: Clip.antiAlias,
          shape: shapes.border(
            Corners.row,
            side: BorderSide(
              color: selected
                  ? shapes.line(t.border)
                  : shapes.lineSoft(t.border),
            ),
          ),
          child: InkWell(
            onTap: tap == null || enabled ? tap : () {},
            hoverColor: still,
            splashColor: still,
            highlightColor: still,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: Space.s3),
              child: _content(t, shapes),
            ),
          ),
        ),
      ),
    );
  }
}

class LinkPainter extends CustomPainter {
  const LinkPainter({
    required this.from,
    required this.start,
    required this.count,
    required this.lit,
    required this.link,
    required this.dot,
    required this.bright,
  });

  final int from;
  final int start;
  final int count;
  final int lit;
  final Color link;
  final Color dot;
  final Color bright;

  @override
  void paint(Canvas canvas, Size size) {
    if (count <= 0) {
      return;
    }
    final double fromY = nodeCentre(from);
    final List<int> order = <int>[
      for (int i = 0; i < count; i++)
        if (i != lit) i,
      if (lit >= 0 && lit < count) lit,
    ];
    final Paint stroke = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = kLinkStroke;
    for (final int i in order) {
      final double toY = nodeCentre(start + i);
      final bool on = i == lit;
      canvas.drawPath(
        Path()
          ..moveTo(0, fromY)
          ..cubicTo(
            size.width / 2,
            fromY,
            size.width / 2,
            toY,
            size.width,
            toY,
          ),
        stroke..color = on ? bright : link,
      );
      _hexagon(
        canvas,
        Offset(size.width - Marks.readoutDot.width / 2, toY),
        Marks.readoutDot,
        on ? bright : dot,
      );
    }
    _hexagon(canvas, Offset(kLinkSource.width / 2, fromY), kLinkSource, bright);
  }

  void _hexagon(Canvas canvas, Offset at, Size size, Color color) {
    final Rect box = Rect.fromCenter(
      center: at,
      width: size.width,
      height: size.height,
    );
    canvas.drawPath(
      Shapes.hexagon(size).getOuterPath(box),
      Paint()..color = color,
    );
  }

  @override
  bool shouldRepaint(LinkPainter oldDelegate) =>
      oldDelegate.from != from ||
      oldDelegate.start != start ||
      oldDelegate.count != count ||
      oldDelegate.lit != lit ||
      oldDelegate.link != link ||
      oldDelegate.dot != dot ||
      oldDelegate.bright != bright;
}
