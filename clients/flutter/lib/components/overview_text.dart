import 'package:flutter/material.dart';

import 'package:shadowmask/components/show_more_button.dart';

class OverviewText extends StatefulWidget {
  const OverviewText(this.text, {super.key, this.maxLines = 4});

  final String text;
  final int maxLines;

  @override
  State<OverviewText> createState() => _OverviewTextState();
}

class _OverviewTextState extends State<OverviewText> {
  bool _expanded = false;

  bool _overflows(TextStyle style, double maxWidth) {
    final TextPainter painter = TextPainter(
      text: TextSpan(text: widget.text, style: style),
      maxLines: widget.maxLines,
      textDirection: Directionality.of(context),
    )..layout(maxWidth: maxWidth);
    final bool exceeded = painter.didExceedMaxLines;
    painter.dispose();
    return exceeded;
  }

  @override
  Widget build(BuildContext context) {
    final TextStyle style =
        Theme.of(context).textTheme.bodyLarge ?? const TextStyle();
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final bool clipped =
            constraints.maxWidth.isFinite &&
            _overflows(style, constraints.maxWidth);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              widget.text,
              style: style,
              maxLines: _expanded ? null : widget.maxLines,
              overflow: _expanded ? TextOverflow.clip : TextOverflow.ellipsis,
            ),
            if (clipped)
              ShowMoreButton(
                expanded: _expanded,
                onPressed: () => setState(() => _expanded = !_expanded),
              ),
          ],
        );
      },
    );
  }
}
