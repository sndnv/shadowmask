import 'package:flutter/material.dart';

import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class LogLine extends StatelessWidget {
  const LogLine(this.entry, {super.key});

  final String entry;

  @override
  Widget build(BuildContext context) {
    return Text(
      entry,
      style: monoStyle.copyWith(
        color: _colorOf(context.tokens, entry),
        fontSize: 12,
      ),
    );
  }
}

Color _colorOf(Tokens t, String entry) {
  final List<String> words = entry
      .split('\n')
      .first
      .trim()
      .split(RegExp(r'\s+'));
  final String level = words.length > 1 ? words[1].toUpperCase() : '';
  return switch (level) {
    'DEBUG' => t.muted,
    'WARN' => t.warn,
    'ERROR' => t.danger,
    _ => t.text,
  };
}
