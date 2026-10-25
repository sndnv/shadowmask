import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/l10n/strings.dart';

const String kDefaultLogLevel = 'INFO';

class LogLevelDropdown extends StatelessWidget {
  const LogLevelDropdown({
    super.key,
    required this.value,
    required this.onChanged,
  });

  final String value;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return AppDropdown<String>(
      value: value,
      width: 130,
      label: Strings.jobLogsHeading,
      items: const <(String, String)>[
        ('DEBUG', 'Debug'),
        ('INFO', 'Info'),
        ('WARN', 'Warn'),
        ('ERROR', 'Error'),
      ],
      onChanged: onChanged,
    );
  }
}
