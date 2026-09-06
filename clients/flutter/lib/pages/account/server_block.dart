import 'package:flutter/material.dart';

import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ServerBlock extends StatelessWidget {
  const ServerBlock({super.key});

  @override
  Widget build(BuildContext context) {
    final ServerScope? scope = ServerScope.of(context);
    if (scope == null) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.serverHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SelectableText(
            scope.address ?? '',
            style: const TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: Space.s2),
          Text(
            Strings.changeServerHelp,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: t.muted),
          ),
          const SizedBox(height: Space.s3),
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton(
              onPressed: () => Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (BuildContext context) =>
                      ServerPage(initialAddress: scope.address),
                ),
              ),
              child: const Text(Strings.changeServer),
            ),
          ),
        ],
      ),
    );
  }
}
