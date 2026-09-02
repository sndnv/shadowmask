import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/link_code_text.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/link_code.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class LinkCodesBlock extends StatefulWidget {
  const LinkCodesBlock({super.key, required this.api, required this.userId});

  final ApiClient api;
  final String userId;

  @override
  State<LinkCodesBlock> createState() => _LinkCodesBlockState();
}

class _LinkCodesBlockState extends State<LinkCodesBlock>
    with Mutations<LinkCodesBlock> {
  late final AccountApi _account = AccountApi(widget.api);
  late Future<List<LinkCode>> _future = _account.linkCodes(widget.userId);

  void _reload() {
    setState(() {
      _future = _account.linkCodes(widget.userId);
    });
  }

  Future<void> _create() => mutate(
    () => _account.createLinkCode(widget.userId),
    successText: Strings.toastCodeCreated,
    errorText: Strings.errorCreate,
    then: _reload,
  );

  Future<void> _revoke(String code) => mutate(
    key: code,
    () => _account.revokeLinkCode(widget.userId, code),
    successText: Strings.toastCodeRevoked,
    errorText: Strings.errorRevoke,
    then: _reload,
  );

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountLinkCodesHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.add,
          label: Strings.createLinkCode,
          primary: true,
          onPressed: busy() ? null : _create,
        ),
      ],
      child: buildBlock<List<LinkCode>>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, List<LinkCode> codes) {
          if (codes.isEmpty) {
            return Text(
              Strings.emptyLinkCodes,
              style: TextStyle(color: context.tokens.muted),
            );
          }
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: Space.s2,
            children: <Widget>[
              for (final LinkCode c in codes) _codeRow(context, c),
            ],
          );
        },
      ),
    );
  }

  Widget _codeRow(BuildContext context, LinkCode c) {
    final Widget code = Align(
      alignment: Alignment.centerLeft,
      child: LinkCodeText(c.code),
    );
    final Widget expiry = Text(
      Strings.expiresAt(dateTimeText(c.expiresAt) ?? c.expiresAt),
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: context.tokens.muted,
        fontWeight: FontWeight.w700,
      ),
    );
    final Widget revoke = TextButton(
      onPressed: busy(c.code) ? null : () => _revoke(c.code),
      child: const Text(Strings.revoke),
    );
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < Breakpoints.sm) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              code,
              Row(
                children: <Widget>[
                  Expanded(child: expiry),
                  revoke,
                ],
              ),
            ],
          );
        }
        return Row(
          children: <Widget>[
            Expanded(child: code),
            Expanded(child: expiry),
            revoke,
          ],
        );
      },
    );
  }
}
