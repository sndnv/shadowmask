import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';

const String kScheduleCustom = 'custom';
const String kScheduleDefaultCustom = '0 0 3 * * * *';

const List<(String, String)> kSchedulePresets = <(String, String)>[
  ('', Strings.scheduleOff),
  ('0 0 * * * * *', Strings.scheduleHourly),
  ('0 0 */6 * * * *', Strings.scheduleSixHourly),
  (kScheduleDefaultCustom, Strings.scheduleDaily),
  ('0 0 3 * * Sun *', Strings.scheduleWeekly),
  (kScheduleCustom, Strings.scheduleCustom),
];

bool isSchedulePreset(String value) =>
    kSchedulePresets.any(((String, String) p) => p.$1 == value);

class ScanScheduleField extends StatefulWidget {
  const ScanScheduleField({
    super.key,
    required this.value,
    required this.onChanged,
    this.enabled = true,
  });

  final String value;
  final ValueChanged<String> onChanged;
  final bool enabled;

  @override
  State<ScanScheduleField> createState() => _ScanScheduleFieldState();
}

class _ScanScheduleFieldState extends State<ScanScheduleField> {
  late String _value = widget.value;
  late bool _custom =
      widget.value.isNotEmpty && !isSchedulePreset(widget.value);
  late final TextEditingController _expression = TextEditingController(
    text: widget.value.isNotEmpty && !isSchedulePreset(widget.value)
        ? widget.value
        : kScheduleDefaultCustom,
  );

  @override
  void didUpdateWidget(ScanScheduleField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.value != oldWidget.value && widget.value != _value) {
      _value = widget.value;
      _custom = _value.isNotEmpty && !isSchedulePreset(_value);
    }
  }

  @override
  void dispose() {
    _expression.dispose();
    super.dispose();
  }

  void _pick(String preset) {
    if (preset == kScheduleCustom) {
      setState(() => _custom = true);
      _value = _expression.text.trim();
      widget.onChanged(_value);
      return;
    }
    setState(() {
      _custom = false;
      _value = preset;
    });
    widget.onChanged(preset);
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        const FieldLabel(
          label: Strings.fieldScanSchedule,
          help: Strings.scanScheduleHelp,
        ),
        const SizedBox(height: Space.s2),
        AppDropdown<String>(
          value: _custom ? kScheduleCustom : _value,
          items: kSchedulePresets,
          onChanged: widget.enabled ? _pick : (String _) {},
        ),
        if (_custom) ...<Widget>[
          const SizedBox(height: Space.s2),
          TextField(
            controller: _expression,
            enabled: widget.enabled,
            onChanged: (String v) {
              _value = v.trim();
              widget.onChanged(_value);
            },
            decoration: const InputDecoration(
              isDense: true,
              labelText: Strings.fieldCronExpression,
            ),
          ),
        ],
      ],
    );
  }
}
