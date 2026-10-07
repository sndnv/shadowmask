import 'package:flutter/material.dart';

import 'package:shadowmask/theme/tokens_context.dart';

class MutedNote extends StatelessWidget {
  const MutedNote(String this.text, {super.key, this.maxLines}) : span = null;

  const MutedNote.rich(InlineSpan this.span, {super.key, this.maxLines})
    : text = null;

  final String? text;
  final InlineSpan? span;
  final int? maxLines;

  @override
  Widget build(BuildContext context) => Text.rich(
    span ?? TextSpan(text: text),
    maxLines: maxLines,
    overflow: maxLines == null ? null : TextOverflow.ellipsis,
    style: Theme.of(
      context,
    ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
  );
}
