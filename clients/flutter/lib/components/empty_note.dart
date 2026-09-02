import 'package:flutter/material.dart';

import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/empty_state.dart';

class EmptyNote extends StatelessWidget {
  const EmptyNote(this.state, {super.key});

  final EmptyState state;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          Text(
            state.text,
            textAlign: TextAlign.center,
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: context.tokens.muted),
          ),
          if (state.hasAction) ...<Widget>[
            const SizedBox(height: Space.s3),
            LinkChip(
              label: state.actionLabel!,
              onTap: () => Navigator.of(context).pushNamed(state.actionRoute!),
            ),
          ],
        ],
      ),
    );
  }
}
