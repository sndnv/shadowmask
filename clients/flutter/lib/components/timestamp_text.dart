import 'package:flutter/material.dart';

import 'package:shadowmask/util/format.dart';

class TimestampText extends StatelessWidget {
  const TimestampText(
    this.iso, {
    super.key,
    this.placeholder = '-',
    this.style,
    this.relative = false,
  });

  final String? iso;
  final String placeholder;
  final TextStyle? style;
  final bool relative;

  @override
  Widget build(BuildContext context) {
    final String? absolute = dateTimeText(iso);
    if (absolute == null) {
      return Text(placeholder, style: style);
    }
    final Widget text = Text(absolute, style: style);
    if (!relative) {
      return text;
    }
    final String? ago = relativeText(iso);
    return ago == null ? text : Tooltip(message: ago, child: text);
  }
}
