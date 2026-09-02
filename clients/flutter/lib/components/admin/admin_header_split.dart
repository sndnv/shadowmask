import 'package:flutter/material.dart';

import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';

class AdminHeaderSplit extends StatelessWidget {
  const AdminHeaderSplit({
    super.key,
    required this.start,
    required this.end,
    this.startWidth,
    this.breakpoint = Breakpoints.sm,
  });

  final Widget start;
  final Widget end;
  final double? startWidth;
  final double breakpoint;

  @override
  Widget build(BuildContext context) {
    final double? fixed = startWidth;
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final Widget head = fixed == null
            ? start
            : SizedBox(width: fixed, child: start);
        if (constraints.maxWidth < breakpoint) {
          return Column(
            crossAxisAlignment: fixed == null
                ? CrossAxisAlignment.stretch
                : CrossAxisAlignment.start,
            children: <Widget>[
              head,
              const SizedBox(height: Space.s4),
              end,
            ],
          );
        }
        return Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (fixed == null) Expanded(child: start) else head,
            const SizedBox(width: Space.s4),
            Expanded(child: end),
          ],
        );
      },
    );
  }
}
