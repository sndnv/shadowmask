import 'package:flutter/material.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';

class SetPasswordDialog extends StatefulWidget {
  const SetPasswordDialog({
    super.key,
    required this.api,
    required this.userId,
    required this.username,
  });

  final ApiClient api;
  final String userId;
  final String username;

  static Future<bool?> show(
    BuildContext context, {
    required ApiClient api,
    required String userId,
    required String username,
  }) => showDialog<bool>(
    context: context,
    builder: (BuildContext context) =>
        SetPasswordDialog(api: api, userId: userId, username: username),
  );

  @override
  State<SetPasswordDialog> createState() => _SetPasswordDialogState();
}

class _SetPasswordDialogState extends State<SetPasswordDialog> {
  late final AccountApi _account = AccountApi(widget.api);
  final TextEditingController _next = TextEditingController();
  final TextEditingController _confirm = TextEditingController();
  bool _saving = false;
  String? _nextError;
  String? _confirmError;

  @override
  void dispose() {
    _next.dispose();
    _confirm.dispose();
    super.dispose();
  }

  bool _validate() {
    final String? next = _next.text.isEmpty
        ? Strings.requiredNewPassword
        : null;
    final String? confirm = _confirm.text == _next.text
        ? null
        : Strings.passwordsDoNotMatch;
    setState(() {
      _nextError = next;
      _confirmError = confirm;
    });
    return next == null && confirm == null;
  }

  Future<void> _submit() async {
    if (!_validate()) {
      return;
    }
    setState(() => _saving = true);
    try {
      await _account.changePassword(widget.userId, newPassword: _next.text);
      if (!mounted) {
        return;
      }
      Toasts.of(context).success(
        Strings.toastPasswordSet(widget.username),
        emphasis: widget.username,
      );
      Navigator.of(context).pop(true);
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorSetPassword, e));
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.setPasswordTitle(widget.username),
      enableClose: !_saving,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: <Widget>[
          FilledButton(
            onPressed: _saving ? null : _submit,
            child: const Text(Strings.setPassword),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: Space.s3,
        children: <Widget>[
          _field(
            _next,
            Strings.fieldNewPassword,
            _nextError,
            (String? v) => _nextError = v,
          ),
          _field(
            _confirm,
            Strings.fieldConfirmPassword,
            _confirmError,
            (String? v) => _confirmError = v,
          ),
        ],
      ),
    );
  }

  Widget _field(
    TextEditingController controller,
    String label,
    String? error,
    ValueChanged<String?> setError,
  ) {
    return LabelledTextField(
      controller: controller,
      label: label,
      obscure: true,
      error: error,
      enabled: !_saving,
      autofillHints: const <String>[AutofillHints.newPassword],
      onChanged: (_) {
        if (error != null) {
          setState(() => setError(null));
        }
      },
    );
  }
}
