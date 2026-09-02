import 'package:flutter/material.dart';

import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';

String catalogCardLabel(CatalogCard card) {
  final String? subtitle = card.subtitle;
  return subtitle == null || subtitle.isEmpty
      ? card.title
      : '${card.title} ($subtitle)';
}

class MatchRow extends StatelessWidget {
  const MatchRow({
    super.key,
    required this.label,
    required this.tooltip,
    required this.onTap,
    this.enabled = true,
  });

  final String label;
  final String tooltip;
  final VoidCallback onTap;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: enabled ? onTap : null,
        borderRadius: const BorderRadius.all(Radii.sm),
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: Space.s2,
            vertical: Space.s2,
          ),
          child: Text(
            label,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: t.text),
          ),
        ),
      ),
    );
  }
}
