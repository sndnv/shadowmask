import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class StatusText extends StatelessWidget {
  const StatusText(this.text, {super.key, this.live = false});

  final String text;
  final bool live;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(Space.s5),
      child: Center(
        child: Semantics(
          liveRegion: live,
          child: Text(
            text,
            textAlign: TextAlign.center,
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: context.tokens.muted),
          ),
        ),
      ),
    );
  }
}
