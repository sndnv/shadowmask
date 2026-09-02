import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/api/account_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';

class PasswordDialog extends StatefulWidget {
  const PasswordDialog({super.key, required this.api, required this.userId});

  final ApiClient api;
  final String userId;

  static Future<void> show(
    BuildContext context, {
    required ApiClient api,
    required String userId,
  }) => showDialog<void>(
    context: context,
    builder: (BuildContext context) => PasswordDialog(api: api, userId: userId),
  );

  @override
  State<PasswordDialog> createState() => _PasswordDialogState();
}

class _PasswordDialogState extends State<PasswordDialog> {
  late final AccountApi _account = AccountApi(widget.api);
  final TextEditingController _current = TextEditingController();
  final TextEditingController _next = TextEditingController();
  final TextEditingController _confirm = TextEditingController();
  bool _saving = false;
  String? _currentError;
  String? _nextError;
  String? _confirmError;

  @override
  void dispose() {
    _current.dispose();
    _next.dispose();
    _confirm.dispose();
    super.dispose();
  }

  bool _validate() {
    final String? current = _current.text.isEmpty
        ? Strings.requiredCurrentPassword
        : null;
    final String? next = _next.text.isEmpty
        ? Strings.requiredNewPassword
        : null;
    final String? confirm = _confirm.text == _next.text
        ? null
        : Strings.passwordsDoNotMatch;
    setState(() {
      _currentError = current;
      _nextError = next;
      _confirmError = confirm;
    });
    return current == null && next == null && confirm == null;
  }

  Future<void> _submit() async {
    if (!_validate()) {
      return;
    }
    setState(() {
      _saving = true;
    });
    try {
      await _account.changePassword(
        widget.userId,
        current: _current.text,
        newPassword: _next.text,
      );
      if (!mounted) {
        return;
      }
      Toasts.of(context).success(Strings.toastPasswordChanged);
      TextInput.finishAutofillContext();
      await widget.api.logout();
      if (!mounted) {
        return;
      }
      Navigator.of(
        context,
      ).pushNamedAndRemoveUntil('/', (Route<dynamic> r) => false);
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorPasswordChange, e));
        setState(() {
          _saving = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.changePassword,
      enableClose: !_saving,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: <Widget>[
          FilledButton(
            onPressed: _saving ? null : _submit,
            child: const Text(Strings.changePassword),
          ),
        ],
      ),
      child: AutofillGroup(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: Space.s3,
          children: <Widget>[
            _field(
              _current,
              Strings.fieldCurrentPassword,
              _currentError,
              (String? v) => _currentError = v,
              hint: AutofillHints.password,
            ),
            Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.s2),
              child: Divider(color: context.tokens.border, height: 1),
            ),
            _field(
              _next,
              Strings.fieldNewPassword,
              _nextError,
              (String? v) => _nextError = v,
              hint: AutofillHints.newPassword,
            ),
            _field(
              _confirm,
              Strings.fieldConfirmPassword,
              _confirmError,
              (String? v) => _confirmError = v,
              hint: AutofillHints.newPassword,
            ),
          ],
        ),
      ),
    );
  }

  Widget _field(
    TextEditingController controller,
    String label,
    String? error,
    ValueChanged<String?> setError, {
    required String hint,
  }) {
    return LabelledTextField(
      controller: controller,
      label: label,
      obscure: true,
      error: error,
      autofillHints: <String>[hint],
      onChanged: (_) {
        if (error != null) {
          setState(() => setError(null));
        }
      },
    );
  }
}
