import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class LogView extends StatelessWidget {
  const LogView({super.key, required this.entries});

  final List<String> entries;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      width: double.infinity,
      constraints: const BoxConstraints(maxHeight: 360),
      padding: const EdgeInsets.all(Space.s3),
      decoration: BoxDecoration(
        color: t.bg,
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: SelectionArea(
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              for (final String entry in entries) LogLine(entry),
            ],
          ),
        ),
      ),
    );
  }
}
