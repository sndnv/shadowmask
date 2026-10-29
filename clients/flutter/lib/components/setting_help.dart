import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const EdgeInsets kSettingRowPadding = EdgeInsets.symmetric(
  horizontal: Space.s2,
);
const double kSettingRowVerticalPadding = Space.s3;
const double kSettingRowMinHeight = 0;

class SettingHelp extends StatelessWidget {
  const SettingHelp(this.text, {super.key, this.color});

  final String text;
  final Color? color;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(left: Space.s2, top: Space.s1),
    child: Text(
      text,
      style: Theme.of(
        context,
      ).textTheme.bodySmall?.copyWith(color: color ?? context.tokens.muted),
    ),
  );
}
