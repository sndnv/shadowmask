import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class MenuOption extends StatelessWidget {
  const MenuOption({
    super.key,
    required this.leading,
    required this.label,
    required this.onTap,
    this.selected = false,
  });

  final Widget leading;
  final String label;
  final VoidCallback onTap;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SelectionContainer.disabled(
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.s3,
            vertical: Space.s1,
          ),
          child: Row(
            children: <Widget>[
              SizedBox(width: 24, height: 24, child: Center(child: leading)),
              const SizedBox(width: Space.s2),
              Expanded(
                child: Text(
                  label,
                  style: TextStyle(
                    color: selected ? t.accent : t.text,
                    fontSize: 14,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
