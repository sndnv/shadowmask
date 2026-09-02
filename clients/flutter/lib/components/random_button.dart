import 'package:flutter/material.dart';

import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/random_pick.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';

class RandomButton extends StatefulWidget {
  const RandomButton({super.key, required this.pick, required this.tooltip});

  final Future<RandomPick> Function() pick;
  final String tooltip;

  @override
  State<RandomButton> createState() => _RandomButtonState();
}

class _RandomButtonState extends State<RandomButton> {
  bool _busy = false;

  Future<void> _press() async {
    setState(() => _busy = true);
    try {
      final RandomPick pick = await widget.pick();
      if (!mounted) {
        return;
      }
      setState(() => _busy = false);
      Navigator.of(context).pushNamed(watchRoute(pick.versionId));
    } catch (e) {
      if (!mounted) {
        return;
      }
      setState(() => _busy = false);
      Toasts.of(context).error(
        failureCode(e) == 'not_found'
            ? Strings.randomNothingToPlay
            : failureText(Strings.errorRandom, e),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return IconButton(
      tooltip: widget.tooltip,
      onPressed: _busy ? null : _press,
      visualDensity: VisualDensity.compact,
      icon: Icon(Icons.shuffle, size: 18, color: t.muted),
    );
  }
}
