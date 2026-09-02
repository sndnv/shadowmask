import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';

class SectionHeading extends StatelessWidget {
  const SectionHeading({super.key, required this.title, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final Widget heading = Text(
      title,
      style: Theme.of(context).textTheme.headlineMedium,
    );
    if (trailing == null) {
      return heading;
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: <Widget>[
        Flexible(child: heading),
        const SizedBox(width: Space.s2),
        trailing!,
      ],
    );
  }
}
