import 'package:flutter/material.dart';

import 'package:shadowmask/api/child_mode_scope.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ChildModeBlock extends StatelessWidget {
  const ChildModeBlock({super.key});

  @override
  Widget build(BuildContext context) {
    final ChildModeScope? scope = ChildModeScope.maybeOf(context);
    if (scope == null) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.childModeHeading,
      child: Material(
        color: Colors.transparent,
        child: ListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text(
            Strings.childModeViewerAge,
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          subtitle: Text(
            Strings.childModeStatus(scope.age),
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: t.muted),
          ),
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
