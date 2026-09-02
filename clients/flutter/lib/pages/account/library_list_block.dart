import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';

class LibraryListBlock extends StatefulWidget {
  const LibraryListBlock({
    super.key,
    required this.catalog,
    required this.title,
    required this.emptyMessage,
    required this.loadRefs,
    required this.onRemove,
    required this.removeLabel,
  });

  final CatalogApi catalog;
  final String title;
  final String emptyMessage;
  final Future<List<TitleRef>> Function() loadRefs;
  final Future<void> Function(TitleRef ref) onRemove;
  final String removeLabel;

  @override
  State<LibraryListBlock> createState() => _LibraryListBlockState();
}

class _LibraryListBlockState extends State<LibraryListBlock>
    with Mutations<LibraryListBlock> {
  late final Future<List<CatalogCard>> _future = _load();
  List<CatalogCard>? _cards;

  Future<List<CatalogCard>> _load() async {
    final List<TitleRef> refs = await widget.loadRefs();
    if (refs.isEmpty) {
      return <CatalogCard>[];
    }
    return widget.catalog.titleCards(refs, asSeriesPoster: true);
  }

  Future<void> _remove(CatalogCard card) => mutate(
    key: card.ref.key,
    () => widget.onRemove(card.ref),
    successText: Strings.toastRemovedTitle(card.title),
    emphasis: card.title,
    errorText: Strings.errorRemove,
    then: () => setState(() => _cards?.remove(card)),
  );

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: widget.title,
      child: buildBlock<List<CatalogCard>>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        loading: const SkeletonCards(count: 4),
        builder: (BuildContext context, List<CatalogCard> loaded) {
          _cards ??= List<CatalogCard>.of(loaded);
          final List<CatalogCard> cards = _cards!;
          if (cards.isEmpty) {
            return Text(
              widget.emptyMessage,
              style: TextStyle(color: context.tokens.muted),
            );
          }
          return LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) {
              final double width = fittedCardWidth(
                constraints.maxWidth,
                aspectOf(cards),
              );
              return Wrap(
                spacing: Space.s4,
                runSpacing: Space.s4,
                children: <Widget>[
                  for (final CatalogCard c in cards)
                    CatalogCardTile(
                      card: c,
                      imageBase: widget.catalog.imageBase,
                      width: width,
                      dismissTooltip: widget.removeLabel,
                      onDismiss: _remove,
                      dismissBusy: busy(c.ref.key),
                    ),
                ],
              );
            },
          );
        },
      ),
    );
  }
}
