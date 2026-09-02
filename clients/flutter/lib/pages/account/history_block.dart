import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/watch_history.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

class HistoryBlock extends StatefulWidget {
  const HistoryBlock({super.key, required this.api, required this.userId});

  final CatalogApi api;
  final String userId;

  @override
  State<HistoryBlock> createState() => _HistoryBlockState();
}

class _HistoryBlockState extends State<HistoryBlock>
    with Mutations<HistoryBlock> {
  late Future<(List<WatchHistory>, Map<String, CatalogCard>)> _future = _load();

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _remove(CatalogCard card) => mutate(
    key: card.ref.id,
    () => widget.api.removeFromHistory(widget.userId, card.ref.id),
    successText: Strings.toastHistoryRemoved,
    errorText: Strings.errorDelete,
    then: _reload,
  );

  Future<void> _clear() async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.clearHistory,
      message: Strings.confirmClearHistory,
      confirmLabel: Strings.clearHistory,
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => widget.api.clearHistory(widget.userId),
      successText: Strings.toastHistoryCleared,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  Future<(List<WatchHistory>, Map<String, CatalogCard>)> _load() async {
    final List<WatchHistory> items = (await widget.api.history(
      widget.userId,
    )).items;
    final Map<String, CatalogCard> byRef = <String, CatalogCard>{};
    if (items.isNotEmpty) {
      try {
        final List<CatalogCard> cards = await widget.api.titleCards(
          items.map((WatchHistory h) => h.title).toList(),
          asSeriesPoster: true,
        );
        for (final CatalogCard c in cards) {
          byRef[_key(c.ref)] = c;
        }
      } catch (_) {}
    }
    return (items, byRef);
  }

  String _key(TitleRef r) => '${r.type.wire}|${r.id}';

  String _caption(WatchHistory h) {
    final String? when = dateText(h.lastWatchedAt);
    return when == null ? '' : Strings.watchedAt(when);
  }

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.accountHistoryHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.delete_sweep_outlined,
          label: Strings.clearHistory,
          danger: true,
          onPressed: busy() ? null : _clear,
        ),
      ],
      child: buildBlock<(List<WatchHistory>, Map<String, CatalogCard>)>(
        future: _future,
        errorText: Strings.couldNotLoadAccount,
        loading: const SkeletonRows(rows: 3),
        builder:
            (
              BuildContext context,
              (List<WatchHistory>, Map<String, CatalogCard>) data,
            ) {
              final List<WatchHistory> items = data.$1;
              final Map<String, CatalogCard> byRef = data.$2;
              final List<(WatchHistory, CatalogCard)> entries =
                  <(WatchHistory, CatalogCard)>[
                    for (final WatchHistory h in items)
                      if (byRef[_key(h.title)] != null)
                        (h, byRef[_key(h.title)]!),
                  ];
              if (entries.isEmpty) {
                return Text(
                  Strings.emptyHistory,
                  style: TextStyle(color: context.tokens.muted),
                );
              }
              return LayoutBuilder(
                builder: (BuildContext context, BoxConstraints constraints) {
                  final double width = fittedCardWidth(
                    constraints.maxWidth,
                    CardAspect.poster,
                  );
                  return Wrap(
                    spacing: Space.s4,
                    runSpacing: Space.s4,
                    children: <Widget>[
                      for (final (WatchHistory h, CatalogCard c) in entries)
                        CatalogCardTile(
                          card: c,
                          imageBase: widget.api.imageBase,
                          width: width,
                          aspect: CardAspect.poster,
                          caption: _caption(h),
                          badge: h.playCount > 1
                              ? Strings.timesWatched(h.playCount)
                              : null,
                          badgeTooltip: h.playCount > 1
                              ? Strings.timesWatchedTooltip(h.playCount)
                              : null,
                          dismissTooltip: Strings.removeFromHistory,
                          onDismiss: _remove,
                          dismissBusy: busy(c.ref.id),
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
