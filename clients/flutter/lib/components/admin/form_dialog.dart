import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';

class FormDialog extends StatelessWidget {
  const FormDialog({
    super.key,
    required this.title,
    required this.child,
    required this.onSubmit,
    this.submitLabel = Strings.save,
    this.submitting = false,
  });

  final String title;
  final Widget child;
  final VoidCallback? onSubmit;
  final String submitLabel;
  final bool submitting;

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: title,
      enableClose: !submitting,
      footer: Align(
        alignment: Alignment.centerRight,
        child: FilledButton(
          onPressed: submitting ? null : onSubmit,
          child: submitting
              ? const SizedBox(
                  width: Space.s4,
                  height: Space.s4,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(submitLabel),
        ),
      ),
      child: child,
    );
  }
}
