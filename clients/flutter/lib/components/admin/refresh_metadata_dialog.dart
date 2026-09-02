import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/edited_notice.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';

enum RefreshChoice { keep, force }

Future<bool> showRefreshMetadataDialog(
  BuildContext context, {
  required AdminApi admin,
  required TitleKind kind,
  required String id,
  bool manuallyEdited = false,
}) async {
  final RefreshChoice? choice = manuallyEdited
      ? await _pickRefreshChoice(context, kind)
      : (await confirmDialog(
              context,
              title: Strings.confirmRefreshHeading,
              message: Strings.confirmRefreshBody,
              confirmLabel: Strings.refreshMetadata,
              danger: false,
            )
            ? RefreshChoice.keep
            : null);
  if (choice == null || !context.mounted) {
    return false;
  }
  try {
    await admin.refreshMetadata(kind, id, force: choice == RefreshChoice.force);
    if (context.mounted) {
      Toasts.of(context).success(Strings.toastMetadataQueued);
    }
    return true;
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorRefresh, e));
    }
    return false;
  }
}

Future<RefreshChoice?> _pickRefreshChoice(
  BuildContext context,
  TitleKind kind,
) => showDialog<RefreshChoice>(
  context: context,
  barrierColor: Colors.black.withValues(alpha: 0.5),
  builder: (BuildContext ctx) {
    final Tokens t = ctx.tokens;
    final TextTheme text = Theme.of(ctx).textTheme;
    return DialogShell(
      title: Strings.confirmRefreshHeading,
      onClose: () => Navigator.of(ctx).pop(),
      footer: Wrap(
        alignment: WrapAlignment.end,
        spacing: Space.s3,
        runSpacing: Space.s2,
        children: <Widget>[
          TextButton(
            style: TextButton.styleFrom(foregroundColor: t.danger),
            onPressed: () => Navigator.of(ctx).pop(RefreshChoice.force),
            child: const Text(Strings.forceRefresh),
          ),
          FilledButton(
            onPressed: () => Navigator.of(ctx).pop(RefreshChoice.keep),
            child: const Text(Strings.keepEdits),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          const EditedNotice(),
          const SizedBox(height: Space.s3),
          Text(
            Strings.refreshChoiceKeep,
            style: text.bodyMedium?.copyWith(color: t.muted),
          ),
          const SizedBox(height: Space.s2),
          Text(
            kind == TitleKind.series
                ? Strings.refreshChoiceDiscardSeries
                : Strings.refreshChoiceDiscardMovie,
            style: text.bodyMedium?.copyWith(color: t.muted),
          ),
        ],
      ),
    );
  },
);
