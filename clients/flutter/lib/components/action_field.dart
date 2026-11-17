import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class ActionField extends StatelessWidget {
  const ActionField({
    super.key,
    required this.controller,
    required this.enabled,
    required this.label,
    required this.hintText,
    required this.onSubmitted,
    required this.actionIcon,
    required this.actionTooltip,
    this.autofocus = false,
  });

  final TextEditingController controller;
  final bool enabled;
  final String label;
  final String hintText;
  final VoidCallback onSubmitted;
  final IconData actionIcon;
  final String actionTooltip;
  final bool autofocus;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    return SizedBox(
      height: kControlHeight,
      child: Semantics(
        label: label,
        child: TextField(
          controller: controller,
          enabled: enabled,
          autofocus: autofocus,
          textInputAction: TextInputAction.search,
          onSubmitted: (_) => onSubmitted(),
          decoration: InputDecoration(
            isDense: true,
            hintText: hintText,
            suffixIcon: Tooltip(
              message: actionTooltip,
              child: Material(
                color: enabled ? t.accent : t.surfaceAlt,
                shape: shapes.border(Corners.field),
                child: InkWell(
                  onTap: enabled ? onSubmitted : null,
                  customBorder: shapes.border(Corners.field),
                  child: SizedBox(
                    width: 34,
                    height: kControlHeight - 2,
                    child: Icon(
                      actionIcon,
                      size: 18,
                      color: enabled ? t.accentContrast : t.muted,
                    ),
                  ),
                ),
              ),
            ),
            suffixIconConstraints: const BoxConstraints(
              minWidth: 34,
              minHeight: kControlHeight - 2,
            ),
            contentPadding: const EdgeInsets.symmetric(
              horizontal: Space.s3,
              vertical: Space.s2,
            ),
          ),
        ),
      ),
    );
  }
}
