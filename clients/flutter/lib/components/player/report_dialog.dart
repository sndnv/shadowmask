import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/diagnostics/report_view.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';

typedef ReportSender = Future<void> Function(String? note);

Future<void> showReportDialog(
  BuildContext context, {
  required Map<String, Object?> payload,
  required ReportSender send,
}) => showDialog<void>(
  context: context,
  builder: (BuildContext _) => ReportDialog(payload: payload, send: send),
);

class ReportDialog extends StatefulWidget {
  const ReportDialog({super.key, required this.payload, required this.send});

  final Map<String, Object?> payload;
  final ReportSender send;

  @override
  State<ReportDialog> createState() => _ReportDialogState();
}

class _ReportDialogState extends State<ReportDialog> {
  final TextEditingController _note = TextEditingController();
  bool _sending = false;

  @override
  void dispose() {
    _note.dispose();
    super.dispose();
  }

  Future<void> _send() async {
    setState(() => _sending = true);
    final String note = _note.text.trim();
    try {
      await widget.send(note.isEmpty ? null : note);
      if (!mounted) {
        return;
      }
      Toasts.of(context).success(Strings.toastReportSent);
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _sending = false);
      Toasts.of(context).error(failureText(Strings.errorSendReport, error));
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.reportTitle,
      width: 720,
      enableClose: !_sending,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: <Widget>[
          TextButton(
            onPressed: _sending ? null : () => Navigator.of(context).pop(),
            child: const Text(Strings.cancel),
          ),
          const SizedBox(width: Space.s2),
          FilledButton(
            onPressed: _sending ? null : _send,
            child: _sending
                ? const SizedBox(
                    width: Space.s4,
                    height: Space.s4,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Text(Strings.reportSend),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          TextField(
            controller: _note,
            enabled: !_sending,
            minLines: 1,
            maxLines: 3,
            decoration: const InputDecoration(labelText: Strings.reportNote),
          ),
          const SizedBox(height: Space.s4),
          ReportView(payload: widget.payload),
        ],
      ),
    );
  }
}
