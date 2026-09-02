import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

Future<bool> confirmDialog(
  BuildContext context, {
  required String title,
  required String message,
  String confirmLabel = Strings.delete,
  bool danger = true,
}) async {
  final bool? ok = await showDialog<bool>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.5),
    builder: (BuildContext ctx) {
      final Tokens t = ctx.tokens;
      return DialogShell(
        title: title,
        onClose: () => Navigator.of(ctx).pop(false),
        footer: Wrap(
          alignment: WrapAlignment.end,
          spacing: Space.s3,
          runSpacing: Space.s2,
          children: <Widget>[
            OutlinedButton(
              onPressed: () => Navigator.of(ctx).pop(false),
              child: const Text(Strings.cancel),
            ),
            FilledButton(
              style: danger
                  ? FilledButton.styleFrom(
                      backgroundColor: t.danger,
                      foregroundColor: t.accentContrast,
                    )
                  : null,
              onPressed: () => Navigator.of(ctx).pop(true),
              child: Text(confirmLabel),
            ),
          ],
        ),
        child: Text(
          message,
          style: Theme.of(ctx).textTheme.bodyMedium?.copyWith(color: t.text),
        ),
      );
    },
  );
  return ok ?? false;
}
