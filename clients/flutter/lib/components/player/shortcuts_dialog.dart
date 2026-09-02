import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/player_shortcuts.dart';

Future<void> showShortcutsDialog(BuildContext context) => showDialog<void>(
  context: context,
  builder: (BuildContext _) => const ShortcutsDialog(),
);

class ShortcutsDialog extends StatelessWidget {
  const ShortcutsDialog({super.key});

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DialogShell(
      title: Strings.shortcutsHeading,
      width: 400,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (final ShortcutRow row in kPlayerShortcuts)
            _Row(keys: row.keys, label: row.label, tokens: t),
          _Row(
            keys: Strings.shortcutDigitKeys,
            label: Strings.shortcutDigits,
            tokens: t,
          ),
        ],
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({required this.keys, required this.label, required this.tokens});

  final String keys;
  final String label;
  final Tokens tokens;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s1),
      child: Row(
        children: <Widget>[
          SizedBox(
            width: 96,
            child: Container(
              padding: const EdgeInsets.symmetric(
                horizontal: Space.s2,
                vertical: 2,
              ),
              decoration: BoxDecoration(
                color: tokens.surfaceAlt,
                borderRadius: const BorderRadius.all(Radii.sm),
                border: Border.all(color: tokens.border),
              ),
              child: Text(
                keys,
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: tokens.text,
                  fontSize: 12.5,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          const SizedBox(width: Space.s3),
          Expanded(
            child: Text(
              label,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: tokens.muted),
            ),
          ),
        ],
      ),
    );
  }
}
