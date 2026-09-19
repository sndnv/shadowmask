import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';

const double kPosterCardWidth = 160;
const double kLandscapeCardWidth = 260;
const double kCastCardWidth = 116;

double cardWidthFor(CardAspect aspect) =>
    aspect == CardAspect.landscape ? kLandscapeCardWidth : kPosterCardWidth;

int minCardColumns(CardAspect aspect) => aspect == CardAspect.landscape ? 1 : 2;

double fittedCardWidth(
  double available,
  CardAspect aspect, {
  double? target,
  int? count,
}) {
  final double preferred = target ?? cardWidthFor(aspect);
  if (!available.isFinite || available <= 0) {
    return preferred;
  }
  if (count != null && count < minCardColumns(aspect)) {
    return preferred;
  }
  final int columns = math.max(
    minCardColumns(aspect),
    ((available + Space.s4) / (preferred + Space.s4)).floor(),
  );
  return math.max(1, (available - Space.s4 * (columns - 1)) / columns);
}

double cardArtHeight(CardAspect aspect) {
  final double width = cardWidthFor(aspect);
  return aspect == CardAspect.landscape ? width * 9 / 16 : width * 3 / 2;
}

double cardRowHeight(CardAspect aspect) =>
    cardArtHeight(aspect) + kCardTextBlockHeight + Space.s4;

CardAspect aspectOf(List<CatalogCard> cards) {
  if (cards.isEmpty) {
    return CardAspect.poster;
  }
  for (final CardAspect aspect in <CardAspect>[
    CardAspect.landscape,
    CardAspect.person,
  ]) {
    if (cards.every((CatalogCard c) => c.aspect == aspect)) {
      return aspect;
    }
  }
  return CardAspect.poster;
}

class CardGrid extends StatelessWidget {
  const CardGrid({
    super.key,
    required this.cards,
    required this.imageBase,
    this.trailing,
    this.aspect,
    this.cardWidth,
  });

  final List<CatalogCard> cards;
  final String imageBase;
  final List<Widget> Function(double width)? trailing;
  final CardAspect? aspect;
  final double? cardWidth;

  @override
  Widget build(BuildContext context) {
    final CardAspect aspect = this.aspect ?? aspectOf(cards);
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double width =
            cardWidth ?? fittedCardWidth(constraints.maxWidth, aspect);
        return Wrap(
          spacing: Space.s4,
          runSpacing: Space.s4,
          children: <Widget>[
            for (final CatalogCard c in cards)
              CatalogCardTile(
                card: c,
                imageBase: imageBase,
                width: width,
                aspect: aspect,
              ),
            ...?trailing?.call(width),
          ],
        );
      },
    );
  }
}
