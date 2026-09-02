import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/when_visible.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

const int kCastCardLimit = 20;

List<Credit> billedCast(List<Credit> credits) {
  final List<Credit> actors = credits
      .where((Credit c) => c.role == CreditRole.actor)
      .toList();
  final List<Credit> all = actors.isEmpty ? credits : actors;
  final List<Credit> ordered = List<Credit>.of(all)
    ..sort((Credit a, Credit b) => a.order.compareTo(b.order));
  final Set<String> seen = <String>{};
  final List<Credit> unique = <Credit>[];
  for (final Credit c in ordered) {
    if (seen.add(c.person.id)) {
      unique.add(c);
    }
  }
  return unique.take(kCastCardLimit).toList();
}

class CastRail extends StatefulWidget {
  const CastRail({super.key, required this.catalog, required this.credits});

  final CatalogApi catalog;
  final List<Credit> credits;

  @override
  State<CastRail> createState() => _CastRailState();
}

class _CastRailState extends State<CastRail> {
  late List<Credit> _cast = billedCast(widget.credits);
  Map<String, Artwork?> _photos = <String, Artwork?>{};
  bool _started = false;

  @override
  void didUpdateWidget(CastRail oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.credits != oldWidget.credits) {
      _cast = billedCast(widget.credits);
    }
  }

  Future<void> _load() async {
    if (_started || _cast.isEmpty) {
      return;
    }
    _started = true;
    final List<CatalogCard> people;
    try {
      people = await widget.catalog.peopleCards(
        _cast.map((Credit c) => c.person.id).toList(),
      );
    } catch (_) {
      return;
    }
    if (!mounted) {
      return;
    }
    setState(() {
      _photos = <String, Artwork?>{
        for (final CatalogCard p in people) p.ref.id: p.artwork,
      };
    });
  }

  List<CatalogCard> get _cards => _cast
      .map(
        (Credit c) => CatalogCard(
          ref: TitleRef(type: TitleKind.person, id: c.person.id),
          route: personRoute(c.person.id),
          title: c.person.name,
          subtitle: c.character,
          artwork: _photos[c.person.id],
          aspect: CardAspect.person,
        ),
      )
      .toList();

  @override
  Widget build(BuildContext context) {
    if (_cast.isEmpty) {
      return const SizedBox.shrink();
    }
    return WhenVisible(
      onVisible: _load,
      child: CardRail(
        title: Strings.castHeading,
        cards: _cards,
        cardWidth: kCastCardWidth,
        imageBase: widget.catalog.imageBase,
      ),
    );
  }
}
