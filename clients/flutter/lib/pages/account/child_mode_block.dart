import 'package:flutter/material.dart';

import 'package:shadowmask/api/child_mode_scope.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/setting_help.dart';
import 'package:shadowmask/l10n/strings.dart';

class ChildModeBlock extends StatelessWidget {
  const ChildModeBlock({super.key});

  @override
  Widget build(BuildContext context) {
    final ChildModeScope? scope = ChildModeScope.maybeOf(context);
    if (scope == null) {
      return const SizedBox.shrink();
    }
    return SectionBlock(
      title: Strings.childModeHeading,
      child: Material(
        color: Colors.transparent,
        child: ListTile(
          contentPadding: kSettingRowPadding,
          minVerticalPadding: kSettingRowVerticalPadding,
          minTileHeight: kSettingRowMinHeight,
          title: const Text(
            Strings.childModeViewerAge,
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          subtitle: SettingHelp(Strings.childModeStatus(scope.age)),
          trailing: AppDropdown<int?>(
            value: scope.age,
            label: Strings.childModeViewerAge,
            items: <(int?, String)>[
              (null, Strings.childModeOff),
              for (final int age in kChildModeAges) (age, childModeLabel(age)),
            ],
            onChanged: scope.setAge,
          ),
        ),
      ),
    );
  }
}
