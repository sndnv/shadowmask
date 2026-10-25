import 'dart:convert';

import 'package:flutter/material.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const JsonEncoder _pretty = JsonEncoder.withIndent('  ');

const Map<String, String> _headings = <String, String>{
  'client': Strings.reportClient,
  'capabilities': Strings.reportCapabilities,
  'history': Strings.reportHistory,
  'panel': Strings.reportPanel,
  'log': Strings.playerLog,
};

String reportSectionText(Object? value) => switch (value) {
  List<Object?>(isEmpty: true) => '',
  List<Object?> items when items.every((Object? item) => item is String) =>
    items.join('\n'),
  List<Object?> items when items.every((Object? item) => item is Map) =>
    items.map(_entryLine).join('\n'),
  _ => _pretty.convert(value),
};

String _entryLine(Object? entry) {
  final Map<String, Object?> facts = Map<String, Object?>.from(entry! as Map);
  final Object? at = facts.remove('at');
  final Object? event = facts.remove('event');
  return <Object?>[
    at,
    event,
    if (facts.isNotEmpty) jsonEncode(facts),
  ].join(' ');
}

class ReportView extends StatelessWidget {
  const ReportView({super.key, required this.payload});

  final Map<String, Object?> payload;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (final MapEntry<String, Object?> section in payload.entries)
          if (section.key != 'schema') ...<Widget>[
            Text(
              _headings[section.key] ?? section.key,
              style: TextStyle(color: t.text, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: Space.s2),
            _SectionBody(text: reportSectionText(section.value)),
            const SizedBox(height: Space.s4),
          ],
      ],
    );
  }
}

class _SectionBody extends StatelessWidget {
  const _SectionBody({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(Space.s3),
      decoration: BoxDecoration(
        color: t.bg,
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: Text(
        text.isEmpty ? Strings.reportNothing : text,
        style: monoStyle.copyWith(
          color: text.isEmpty ? t.muted : t.text,
          fontSize: 12,
          height: 1.4,
        ),
      ),
    );
  }
}
