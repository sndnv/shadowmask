import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/api_token.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class TokensBlock extends StatefulWidget {
  const TokensBlock({super.key, required this.api, required this.userId});

  final ApiClient api;
  final String userId;

  @override
  State<TokensBlock> createState() => _TokensBlockState();
}

class _TokensBlockState extends State<TokensBlock> with Mutations<TokensBlock> {
  late final AccountApi _account = AccountApi(widget.api);
  late Future<List<ApiToken>> _future = _account.tokens(widget.userId);

  Future<void> _revoke(String tokenId) => mutate(
    key: tokenId,
    () => _account.revokeToken(widget.userId, tokenId),
    successText: Strings.toastTokenRevoked,
    errorText: Strings.errorRevoke,
    then: () => setState(() {
      _future = _account.tokens(widget.userId);
    }),
  );

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountTokensHeading,
      child: buildBlock<List<ApiToken>>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, List<ApiToken> tokens) {
          if (tokens.isEmpty) {
            return Text(
              Strings.emptyTokens,
              style: TextStyle(color: context.tokens.muted),
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: Space.s2,
            children: <Widget>[
              for (final ApiToken tk in tokens)
                Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        tk.deviceId,
                        style: monoStyle.copyWith(color: context.tokens.text),
                      ),
                    ),
                    if (tk.lastUsedAt != null)
                      Text(
                        Strings.lastSeen(
                          relativeText(tk.lastUsedAt) ?? tk.lastUsedAt!,
                        ),
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: context.tokens.muted,
                        ),
                      ),
                    TextButton(
                      onPressed: busy(tk.id) ? null : () => _revoke(tk.id),
                      child: const Text(Strings.revoke),
                    ),
                  ],
                ),
            ],
          );
        },
      ),
    );
  }
}
