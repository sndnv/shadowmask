import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ShowMoreButton extends StatelessWidget {
  const ShowMoreButton({
    super.key,
    required this.expanded,
    required this.onPressed,
  });

  final bool expanded;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return SelectionContainer.disabled(
      child: TextButton(
        onPressed: onPressed,
        style: TextButton.styleFrom(
          foregroundColor: context.tokens.accent,
          padding: const EdgeInsets.symmetric(vertical: Space.s1),
          minimumSize: Size.zero,
          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
          textStyle: const TextStyle(
            fontSize: 13,
            fontWeight: FontWeight.w600,
            decoration: TextDecoration.underline,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(expanded ? Strings.showLess : Strings.showMore),
            const SizedBox(width: Space.s1),
            Icon(expanded ? Icons.expand_less : Icons.expand_more, size: 16),
          ],
        ),
      ),
    );
  }
}
