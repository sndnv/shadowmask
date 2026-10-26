import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Future<bool?> showWipeConfirmDialog(
  BuildContext context, {
  required String title,
  required String message,
  required String confirmLabel,
}) => showDialog<bool>(
  context: context,
  barrierColor: Colors.black.withValues(alpha: 0.5),
  builder: (BuildContext ctx) => _WipeConfirmDialog(
    title: title,
    message: message,
    confirmLabel: confirmLabel,
  ),
);

class _WipeConfirmDialog extends StatefulWidget {
  const _WipeConfirmDialog({
    required this.title,
    required this.message,
    required this.confirmLabel,
  });

  final String title;
  final String message;
  final String confirmLabel;

  @override
  State<_WipeConfirmDialog> createState() => _WipeConfirmDialogState();
}

class _WipeConfirmDialogState extends State<_WipeConfirmDialog> {
  bool _wipeUserData = false;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DialogShell(
      title: widget.title,
      onClose: () => Navigator.of(context).pop(),
      footer: Wrap(
        alignment: WrapAlignment.end,
        spacing: Space.s3,
        runSpacing: Space.s2,
        children: <Widget>[
          OutlinedButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text(Strings.cancel),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: t.danger,
              foregroundColor: t.accentContrast,
            ),
            onPressed: () => Navigator.of(context).pop(_wipeUserData),
            child: Text(widget.confirmLabel),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            widget.message,
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: t.text),
          ),
          const SizedBox(height: Space.s3),
          Material(
            type: MaterialType.transparency,
            child: CheckboxListTile(
              contentPadding: EdgeInsets.zero,
              controlAffinity: ListTileControlAffinity.leading,
              value: _wipeUserData,
              title: const Text(Strings.wipeUserData),
              onChanged: (bool? v) =>
                  setState(() => _wipeUserData = v ?? false),
            ),
          ),
        ],
      ),
    );
  }
}
