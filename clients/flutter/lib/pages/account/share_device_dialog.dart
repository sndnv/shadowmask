import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/view/failure_reason.dart';

class ShareDeviceDialog extends StatefulWidget {
  const ShareDeviceDialog({super.key, required this.api, required this.device});

  final ApiClient api;
  final Device device;

  static Future<bool?> show(
    BuildContext context, {
    required ApiClient api,
    required Device device,
  }) => showDialog<bool>(
    context: context,
    builder: (BuildContext context) =>
        ShareDeviceDialog(api: api, device: device),
  );

  @override
  State<ShareDeviceDialog> createState() => _ShareDeviceDialogState();
}

class _ShareDeviceDialogState extends State<ShareDeviceDialog> {
  late final AccountApi _account = AccountApi(widget.api);
  final TextEditingController _username = TextEditingController();
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    _username.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final String username = _username.text.trim();
    if (username.isEmpty) {
      setState(() => _error = Strings.requiredUsername);
      return;
    }
    setState(() => _saving = true);
    try {
      await _account.shareDevice(widget.device.id, username);
      if (!mounted) {
        return;
      }
      Toasts.of(
        context,
      ).success(Strings.toastDeviceShared(username), emphasis: username);
      Navigator.of(context).pop(true);
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorShare, e));
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.shareDeviceTitle(widget.device.name),
      enableClose: !_saving,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: <Widget>[
          FilledButton(
            onPressed: _saving ? null : _submit,
            child: const Text(Strings.share),
          ),
        ],
      ),
      child: LabelledTextField(
        controller: _username,
        label: Strings.fieldUsername,
        error: _error,
        onChanged: (_) {
          if (_error != null) {
            setState(() => _error = null);
          }
        },
        enabled: !_saving,
        onSubmitted: (_) => _submit(),
      ),
    );
  }
}
