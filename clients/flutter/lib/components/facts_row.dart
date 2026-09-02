import 'package:flutter/material.dart';

import 'package:shadowmask/components/fact.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class FactsRow extends StatelessWidget {
  const FactsRow(this.facts, {super.key, this.labels = true});

  final List<Fact> facts;
  final bool labels;

  static List<Fact> of(List<(String, String?)> pairs) => <Fact>[
    for (final (String label, String? value) in pairs)
      if (value != null && value.isNotEmpty) Fact(label, value),
  ];

  @override
  Widget build(BuildContext context) {
    if (facts.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    final TextStyle? style = Theme.of(context).textTheme.bodyMedium;
    if (!labels) {
      final TextStyle? bare = style?.copyWith(
        color: t.muted,
        fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
      );
      return Wrap(
        spacing: Space.s2,
        runSpacing: Space.s1,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: <Widget>[
          for (int i = 0; i < facts.length; i++) ...<Widget>[
            if (i > 0)
              Text(
                '·',
                style: bare?.copyWith(color: t.muted.withValues(alpha: 0.6)),
              ),
            Text(facts[i].value, style: bare),
          ],
        ],
      );
    }
    return Wrap(
      spacing: Space.s4,
      runSpacing: Space.s2,
      children: <Widget>[
        for (final Fact f in facts)
          Text.rich(
            TextSpan(
              children: <TextSpan>[
                TextSpan(
                  text: '${f.label}: ',
                  style: style?.copyWith(color: t.muted),
                ),
                TextSpan(
                  text: f.value,
                  style: style?.copyWith(color: t.text),
                ),
              ],
            ),
          ),
      ],
    );
  }
}
