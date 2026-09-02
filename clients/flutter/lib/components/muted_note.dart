import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens_context.dart';

class MutedNote extends StatelessWidget {
  const MutedNote(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Text(
    text,
    style: Theme.of(
      context,
    ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
  );
}
