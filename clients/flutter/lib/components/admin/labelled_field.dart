import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';

class LabelledTextField extends StatelessWidget {
  const LabelledTextField({
    super.key,
    required this.controller,
    required this.label,
    this.help,
    this.hint,
    this.error,
    this.enabled = true,
    this.readOnly = false,
    this.obscure = false,
    this.maxLines = 1,
    this.keyboardType,
    this.autofillHints = const <String>[],
    this.onChanged,
    this.onSubmitted,
  });

  final TextEditingController controller;
  final String label;
  final String? help;
  final String? hint;
  final String? error;
  final bool enabled;
  final bool readOnly;
  final bool obscure;
  final int maxLines;
  final TextInputType? keyboardType;
  final List<String> autofillHints;
  final ValueChanged<String>? onChanged;
  final ValueChanged<String>? onSubmitted;

  @override
  Widget build(BuildContext context) => FieldLabel(
    label: label,
    help: help,
    child: Semantics(
      label: label,
      child: TextField(
        controller: controller,
        enabled: enabled,
        readOnly: readOnly,
        obscureText: obscure,
        maxLines: maxLines,
        keyboardType: keyboardType,
        autofillHints: autofillHints,
        onChanged: onChanged,
        onSubmitted: onSubmitted,
        decoration: InputDecoration(
          isDense: true,
          hintText: hint,
          errorText: error,
        ),
      ),
    ),
  );
}

class LabelledDropdown<T> extends StatelessWidget {
  const LabelledDropdown({
    super.key,
    required this.label,
    required this.value,
    required this.items,
    required this.onChanged,
    this.help,
    this.error,
    this.enabled = true,
  });

  final String label;
  final T value;
  final List<(T, String)> items;
  final ValueChanged<T> onChanged;
  final String? help;
  final String? error;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final String? message = error;
    return FieldLabel(
      label: label,
      help: help,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Semantics(
            label: label,
            child: AppDropdown<T>(
              value: value,
              items: items,
              onChanged: onChanged,
              enabled: enabled,
            ),
          ),
          if (message != null) FieldError(message),
        ],
      ),
    );
  }
}

class FieldError extends StatelessWidget {
  const FieldError(this.message, {super.key});

  final String message;

  @override
  Widget build(BuildContext context) {
    final ThemeData theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(left: 12, top: 8),
      child: Text(
        message,
        style:
            theme.inputDecorationTheme.errorStyle ??
            TextStyle(color: theme.colorScheme.error, fontSize: 12),
      ),
    );
  }
}
