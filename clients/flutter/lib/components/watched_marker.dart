import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class WatchedMarker extends StatelessWidget {
  const WatchedMarker({super.key});

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Tooltip(
      message: Strings.watchedLabel,
      child: Container(
        width: 22,
        height: 22,
        decoration: BoxDecoration(
          color: t.ok,
          shape: BoxShape.circle,
          border: Border.all(color: t.surface, width: 2),
        ),
        child: const Icon(Icons.check, size: 13, color: Colors.white),
      ),
    );
  }
}
