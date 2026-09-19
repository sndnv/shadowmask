import 'package:flutter/material.dart';

import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/bare_page.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ServerUnreachablePage extends StatelessWidget {
  const ServerUnreachablePage({super.key, required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final TextTheme text = Theme.of(context).textTheme;
    final ServerScope? scope = ServerScope.of(context);
    return Title(
      color: t.accent,
      title: Strings.documentTitle(Strings.serverUnreachableHeading),
      child: BarePage(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: Breakpoints.sm),
            child: Padding(
              padding: const EdgeInsets.all(Space.s5),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Text(
                    Strings.serverUnreachableHeading,
                    textAlign: TextAlign.center,
                    style: text.headlineMedium?.copyWith(color: t.text),
                  ),
                  const SizedBox(height: Space.s3),
                  Text(
                    Strings.serverUnreachableBody,
                    textAlign: TextAlign.center,
                    style: text.bodyMedium?.copyWith(color: t.muted),
                  ),
                  const SizedBox(height: Space.s5),
                  OutlinedButton.icon(
                    onPressed: onRetry,
                    icon: const Icon(Icons.refresh, size: 18),
                    label: const Text(Strings.retry),
                  ),
                  if (scope != null) ...<Widget>[
                    const SizedBox(height: Space.s2),
                    TextButton(
                      onPressed: () => Navigator.of(context).push(
                        MaterialPageRoute<void>(
                          builder: (BuildContext context) =>
                              ServerPage(initialAddress: scope.address),
                        ),
                      ),
                      child: const Text(Strings.changeServer),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
