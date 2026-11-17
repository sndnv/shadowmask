import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/fact.dart';
import 'package:shadowmask/components/fact_readout.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const Set<String> _kReadoutLabels = <String>{
  Strings.factYear,
  Strings.factQuality,
};

class FactsRow extends StatelessWidget {
  const FactsRow(
    this.facts, {
    super.key,
    this.labels = true,
    this.readouts = false,
  });

  final List<Fact> facts;
  final bool labels;
  final bool readouts;

  static List<Fact> of(List<(String, String?)> pairs) => <Fact>[
    for (final (String label, String? value) in pairs)
      if (value != null && value.isNotEmpty) Fact(label, value),
  ];

  static ({List<Fact> readouts, List<Fact> rest}) readoutsOf(
    List<Fact> facts,
  ) => (
    readouts: <Fact>[
      for (final Fact f in facts)
        if (_kReadoutLabels.contains(f.label)) f,
    ],
    rest: <Fact>[
      for (final Fact f in facts)
        if (!_kReadoutLabels.contains(f.label)) f,
    ],
  );

  @override
  Widget build(BuildContext context) {
    if (facts.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    final TextStyle? style = Theme.of(context).textTheme.bodyMedium;
    if (readouts && context.shapes.cut) {
      final ({List<Fact> readouts, List<Fact> rest}) split = readoutsOf(facts);
      return Wrap(
        spacing: Space.s5,
        runSpacing: Space.s3,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: <Widget>[
          for (final Fact f in split.readouts) FactReadout(f),
          if (split.rest.isNotEmpty) _bare(t, style, split.rest),
        ],
      );
    }
    if (!labels) {
      return _bare(t, style, facts);
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

  Widget _bare(Tokens t, TextStyle? style, List<Fact> shown) {
    final TextStyle? bare = style?.copyWith(
      color: t.muted,
      fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
    );
    return Wrap(
      spacing: Space.s2,
      runSpacing: Space.s1,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        for (int i = 0; i < shown.length; i++) ...<Widget>[
          if (i > 0)
            Text(
              '·',
              style: bare?.copyWith(color: t.muted.withValues(alpha: 0.6)),
            ),
          Text(shown[i].value, style: bare),
        ],
      ],
    );
  }
}
