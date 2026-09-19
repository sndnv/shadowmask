import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';

class SessionBlock extends StatefulWidget {
  const SessionBlock({
    super.key,
    required this.api,
    required this.userId,
    this.showSignOutEverywhere = true,
  });

  final ApiClient api;
  final String userId;
  final bool showSignOutEverywhere;

  @override
  State<SessionBlock> createState() => _SessionBlockState();
}

class _SessionBlockState extends State<SessionBlock> {
  late final AccountApi _account = AccountApi(widget.api);
  bool _busy = false;

  Future<void> _signOut() async {
    setState(() => _busy = true);
    await _account.signOutThisDevice(widget.userId);
    _goToSignIn();
  }

  void _goToSignIn() {
    if (!mounted) {
      return;
    }
    Navigator.of(
      context,
    ).pushNamedAndRemoveUntil('/', (Route<dynamic> r) => false);
  }

  Future<void> _signOutEverywhere() async {
    setState(() => _busy = true);
    try {
      await _account.signOutEverywhere(widget.userId);
      if (!mounted) {
        return;
      }
      Toasts.of(context).success(Strings.toastAllSessionsRevoked);
      await widget.api.logout();
      _goToSignIn();
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountSessionHeading,
      child: Wrap(
        spacing: Space.s3,
        runSpacing: Space.s3,
        children: <Widget>[
          OutlinedButton(
            onPressed: _busy ? null : _signOut,
            child: const Text(Strings.signOut),
          ),
          if (widget.showSignOutEverywhere)
            FilledButton(
              onPressed: _busy ? null : _signOutEverywhere,
              style: FilledButton.styleFrom(
                backgroundColor: context.tokens.danger,
              ),
              child: const Text(Strings.signOutEverywhere),
            ),
        ],
      ),
    );
  }
}
