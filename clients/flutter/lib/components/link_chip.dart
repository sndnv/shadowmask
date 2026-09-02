import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kChipHeight = 28;

class LinkChip extends StatefulWidget {
  const LinkChip({
    super.key,
    required this.label,
    this.leading,
    this.value,
    this.labelColor,
    this.onTap,
  });

  final String label;
  final Widget? leading;
  final String? value;
  final Color? labelColor;
  final VoidCallback? onTap;

  @override
  State<LinkChip> createState() => _LinkChipState();
}

class _LinkChipState extends State<LinkChip> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final bool tappable = widget.onTap != null;
    final bool lit = _hovered && tappable;
    return SelectionContainer.disabled(
      child: SizedBox(
        height: kChipHeight,
        child: Material(
          color: lit ? t.accent.withValues(alpha: 0.12) : t.surfaceAlt,
          shape: RoundedRectangleBorder(
            borderRadius: const BorderRadius.all(Radii.pill),
            side: BorderSide(color: tappable ? t.accent : t.border),
          ),
          child: InkWell(
            borderRadius: const BorderRadius.all(Radii.pill),
            onTap: widget.onTap,
            onHover: (bool hovered) => setState(() => _hovered = hovered),
            child: Padding(
              padding: EdgeInsets.only(
                left: widget.leading == null ? Space.s3 : 3,
                right: Space.s3,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (widget.leading != null) ...<Widget>[
                    widget.leading!,
                    const SizedBox(width: Space.s2),
                  ],
                  Text(
                    widget.label,
                    style: TextStyle(
                      color: lit ? t.accent : (widget.labelColor ?? t.text),
                      fontSize: 13,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  if (widget.value != null) ...<Widget>[
                    const SizedBox(width: Space.s1),
                    Text(
                      widget.value!,
                      style: TextStyle(
                        color: lit ? t.accent : t.text,
                        fontSize: 13,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
