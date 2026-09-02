import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';

const Duration kFilterDebounce = Duration(milliseconds: 600);

class AdminFilterField extends StatelessWidget {
  const AdminFilterField({
    super.key,
    required this.controller,
    required this.hintText,
    required this.onChanged,
    this.width = 320,
  });

  final TextEditingController controller;
  final String hintText;
  final ValueChanged<String> onChanged;
  final double width;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: BoxConstraints.tightFor(
      height: kControlHeight,
    ).copyWith(maxWidth: width),
    child: Semantics(
      label: Strings.fieldFilter,
      child: TextField(
        controller: controller,
        onChanged: onChanged,
        textAlignVertical: TextAlignVertical.center,
        decoration: InputDecoration(
          isDense: true,
          hintText: hintText,
          contentPadding: const EdgeInsets.symmetric(horizontal: Space.s2),
          prefixIcon: const Icon(Icons.filter_list, size: 18),
          prefixIconConstraints: const BoxConstraints(
            minWidth: 34,
            minHeight: kControlHeight,
          ),
        ),
      ),
    ),
  );
}
