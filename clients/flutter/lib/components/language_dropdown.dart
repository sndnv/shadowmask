import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/util/languages.dart';

class LanguageDropdown extends StatelessWidget {
  const LanguageDropdown({
    super.key,
    required this.value,
    required this.emptyLabel,
    required this.onChanged,
    this.label,
    this.help,
    this.error,
    this.enabled = true,
    this.width,
  });

  final String value;
  final String emptyLabel;
  final ValueChanged<String> onChanged;
  final String? label;
  final String? help;
  final String? error;
  final bool enabled;
  final double? width;

  @override
  Widget build(BuildContext context) {
    final String? message = error;
    final Widget dropdown = AppDropdown<String>(
      value: value,
      width: width,
      items: <(String, String)>[
        ('', emptyLabel),
        ...languageOptions(keep: value),
      ],
      onChanged: enabled ? onChanged : (String _) {},
    );
    final Widget control = message == null
        ? dropdown
        : Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[dropdown, FieldError(message)],
          );
    final String? heading = label;
    if (heading == null) {
      return control;
    }
    return FieldLabel(label: heading, help: help, child: control);
  }
}
