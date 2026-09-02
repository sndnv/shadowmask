import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/load_more.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

const int kPlaceholderCards = 4;
const int kPrefetchRows = 3;

class PagedCardGrid extends StatelessWidget {
  const PagedCardGrid({
    super.key,
    required this.cards,
    required this.imageBase,
    required this.remaining,
    required this.loading,
    required this.failed,
    required this.onLoad,
  });

  final List<CatalogCard> cards;
  final String imageBase;
  final int remaining;
  final bool loading;
  final bool failed;
  final VoidCallback onLoad;

  @override
  Widget build(BuildContext context) {
    final CardAspect aspect = aspectOf(cards);
    final int placeholders = failed
        ? 0
        : math.min(kPlaceholderCards, math.max(0, remaining));
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        LoadMore(
          onLoad: onLoad,
          enabled: !loading && !failed && remaining > 0,
          threshold: cardRowHeight(aspect) * kPrefetchRows,
          child: CardGrid(
            cards: cards,
            imageBase: imageBase,
            aspect: aspect,
            trailing: (double width) => <Widget>[
              for (int i = 0; i < placeholders; i++)
                SkeletonCard(aspect: aspect, width: width),
            ],
          ),
        ),
        if (failed) _MoreFailed(onRetry: onLoad),
      ],
    );
  }
}

class _MoreFailed extends StatelessWidget {
  const _MoreFailed({required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SelectionContainer.disabled(
      child: Padding(
        padding: const EdgeInsets.only(top: Space.s4),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            Text(
              Strings.couldNotLoadMore,
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(color: t.muted),
            ),
            const SizedBox(width: Space.s3),
            OutlinedButton(
              onPressed: onRetry,
              style: OutlinedButton.styleFrom(
                backgroundColor: t.surface,
                foregroundColor: t.text,
                side: BorderSide(color: t.border),
                textStyle: const TextStyle(
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
              child: const Text(Strings.retry),
            ),
          ],
        ),
      ),
    );
  }
}
