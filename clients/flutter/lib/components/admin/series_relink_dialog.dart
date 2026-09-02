import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/util/external_id.dart';
import 'package:shadowmask/view/failure_reason.dart';

Future<bool> showSeriesRelinkDialog(
  BuildContext context, {
  required AdminApi admin,
  required String seriesId,
  required String title,
}) async {
  final bool? done = await showDialog<bool>(
    context: context,
    builder: (BuildContext _) =>
        _SeriesRelinkDialog(admin: admin, seriesId: seriesId, title: title),
  );
  return done ?? false;
}

class _SeriesRelinkDialog extends StatefulWidget {
  const _SeriesRelinkDialog({
    required this.admin,
    required this.seriesId,
    required this.title,
  });

  final AdminApi admin;
  final String seriesId;
  final String title;

  @override
  State<_SeriesRelinkDialog> createState() => _SeriesRelinkDialogState();
}

class _SeriesRelinkDialogState extends State<_SeriesRelinkDialog> {
  final TextEditingController _value = TextEditingController();
  String? _error;
  bool _submitting = false;

  @override
  void dispose() {
    _value.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final Map<String, String>? external = externalIdFor(
      _value.text,
      TitleKind.series,
    );
    if (external == null) {
      setState(() => _error = Strings.requiredTmdbId);
      return;
    }
    setState(() => _error = null);
    final bool ok = await confirmDialog(
      context,
      title: Strings.confirmRelinkHeading,
      message: Strings.confirmRelinkSeries(widget.title),
      confirmLabel: Strings.relink,
      danger: false,
    );
    if (!ok || !mounted) {
      return;
    }
    setState(() => _submitting = true);
    try {
      await widget.admin.relinkSeries(widget.seriesId, <String, dynamic>{
        'kind': 'provider',
        ...external,
      });
      if (mounted) {
        Toasts.of(context).success(Strings.toastRelinkQueued);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorRelink, e));
        setState(() => _submitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.relinkSeries,
      submitLabel: Strings.relink,
      submitting: _submitting,
      onSubmit: _submit,
      child: LabelledTextField(
        controller: _value,
        label: Strings.fieldTmdbSeriesId,
        help: Strings.seriesRelinkHelp,
        enabled: !_submitting,
        error: _error,
        onSubmitted: (_) => _submit(),
      ),
    );
  }
}
