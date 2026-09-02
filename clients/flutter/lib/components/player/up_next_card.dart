import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class UpNextCard extends StatelessWidget {
  const UpNextCard({
    super.key,
    required this.label,
    required this.remainingSeconds,
    required this.onPlay,
    required this.onCancel,
  });

  final String label;
  final int remainingSeconds;
  final VoidCallback onPlay;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Material(
      color: t.surface,
      clipBehavior: Clip.antiAlias,
      shape: RoundedRectangleBorder(
        borderRadius: const BorderRadius.all(Radii.md),
        side: BorderSide(color: t.border),
      ),
      child: Padding(
        padding: const EdgeInsets.all(Space.s4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              Strings.playerUpNextIn(remainingSeconds),
              style: Theme.of(
                context,
              ).textTheme.labelSmall?.copyWith(color: t.muted),
            ),
            const SizedBox(height: Space.s1),
            Text(
              label,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: t.text,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: Space.s3),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                FilledButton(
                  onPressed: onPlay,
                  child: const Text(Strings.playerPlayNow),
                ),

                const SizedBox(width: Space.s2),
                TextButton(
                  onPressed: onCancel,
                  child: const Text(Strings.cancel),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
