import 'package:flutter/material.dart';

import 'package:shadowmask/components/link_chip.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';

Color? sourceColor(Tokens t, String source) => switch (source.toLowerCase()) {
  'internet movie database' => t.warn,
  'rotten tomatoes' => t.danger,
  'metacritic' => t.ok,
  _ => null,
};

class RatingChips extends StatelessWidget {
  const RatingChips(this.ratings, {super.key});

  final List<Rating> ratings;

  LinkChip _chip(Tokens t, Rating r) => LinkChip(
    label: '${scoreSource(r.source)}:',
    value: scoreText(r.source, r.value),
    labelColor: sourceColor(t, r.source),
  );

  @override
  Widget build(BuildContext context) {
    if (ratings.isEmpty) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return Wrap(
      spacing: Space.s2,
      runSpacing: Space.s2,
      children: <Widget>[for (final Rating r in ratings) _chip(t, r)],
    );
  }
}
