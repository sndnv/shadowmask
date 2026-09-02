import 'package:flutter/material.dart';

class StartEllipsisText extends StatelessWidget {
  const StartEllipsisText(this.text, {super.key, this.style});

  final String text;
  final TextStyle? style;

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
        return Tooltip(message: text, child: line);
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
      int lo = 1;
      int hi = chars.length;
      while (lo < hi) {
        final int mid = lo + (hi - lo) ~/ 2;
        if (widthOf('…${chars.sublist(mid).join()}') <= available) {
          hi = mid;
        } else {
          lo = mid + 1;
        }
      }
      return '…${chars.sublist(lo).join()}';
    } finally {
      painter.dispose();
    }
  }
}
