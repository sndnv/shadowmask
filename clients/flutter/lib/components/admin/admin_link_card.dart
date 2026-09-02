import 'package:flutter/material.dart';

import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double kAdminLinkCardWidth = 260;

class AdminLinkCard extends StatelessWidget {
  const AdminLinkCard({
    super.key,
    required this.title,
    required this.description,
    required this.icon,
    required this.onTap,
  });

  final String title;
  final String description;
  final IconData icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) => SizedBox(
        width: constraints.maxWidth < Breakpoints.sm
            ? constraints.maxWidth
            : kAdminLinkCardWidth,
        child: _card(context),
      ),
    );
  }

  Widget _card(BuildContext context) {
    final Tokens t = context.tokens;
    return Material(
      color: t.surface,
      borderRadius: const BorderRadius.all(Radii.md),
      child: InkWell(
        onTap: onTap,
        borderRadius: const BorderRadius.all(Radii.md),
        child: Container(
          padding: const EdgeInsets.all(Space.s4),
          decoration: BoxDecoration(
            borderRadius: const BorderRadius.all(Radii.md),
            border: Border.all(color: t.border),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Icon(icon, color: t.accent),
              const SizedBox(width: Space.s3),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(title, style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: Space.s1),
                    Text(
                      description,
                      style: Theme.of(
                        context,
                      ).textTheme.bodySmall?.copyWith(color: t.muted),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
