import 'package:flutter/material.dart';

class StartEllipsisText extends StatelessWidget {
  const StartEllipsisText(this.text, {super.key, this.style, this.tooltip})
    : _middle = false;

  const StartEllipsisText.middle(
    this.text, {
    super.key,
    this.style,
    this.tooltip,
  }) : _middle = true;

  final String text;
  final TextStyle? style;
  final String? tooltip;
  final bool _middle;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final Widget line = Text(
          _clip(context, constraints.maxWidth),
          style: style,
          maxLines: 1,
          softWrap: false,
        );
        return Tooltip(message: tooltip ?? text, child: line);
      },
    );
  }

  String _clip(BuildContext context, double available) {
    if (!available.isFinite || available <= 0 || text.isEmpty) {
      return text;
    }
    final TextStyle effective = DefaultTextStyle.of(context).style.merge(style);
    final TextPainter painter = TextPainter(
      textDirection: Directionality.of(context),
      maxLines: 1,
    );
    double widthOf(String candidate) {
      painter.text = TextSpan(text: candidate, style: effective);
      painter.layout();
      return painter.width;
    }

    try {
      if (widthOf(text) <= available) {
        return text;
      }
      final List<String> chars = text.characters.toList();
      final String Function(int kept) shorten = _middle
          ? (int kept) => _keepEnds(chars, kept)
          : (int kept) => '…${chars.sublist(chars.length - kept).join()}';
      int lo = 0;
      int hi = chars.length - 1;
      while (lo < hi) {
        final int mid = hi - (hi - lo) ~/ 2;
        if (widthOf(shorten(mid)) <= available) {
          lo = mid;
        } else {
          hi = mid - 1;
        }
      }
      return shorten(lo);
    } finally {
      painter.dispose();
    }
  }

  static String _keepEnds(List<String> chars, int kept) {
    final int dot = chars.lastIndexOf('.');
    final int extension = dot > 0 ? chars.length - dot : 0;
    final int tail = (kept ~/ 3 < extension ? extension : kept ~/ 3).clamp(
      0,
      kept,
    );
    final int head = kept - tail;
    return '${chars.sublist(0, head).join()}…'
        '${chars.sublist(chars.length - tail).join()}';
  }
}
