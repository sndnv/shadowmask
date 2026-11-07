import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/playlist_dialogs.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user_library/item_state.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/failure_reason.dart';

enum CardAction { watched, watchlist, favorite, playlist, dequeue, dismiss }

class CardMenuHost extends InheritedWidget {
  const CardMenuHost({
    super.key,
    required this.catalog,
    required this.userId,
    this.playlists,
    required super.child,
  });

  final CatalogApi catalog;
  final String userId;
  final PlaylistApi? playlists;

  static CardMenuHost? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<CardMenuHost>();

  @override
  bool updateShouldNotify(CardMenuHost oldWidget) =>
      catalog != oldWidget.catalog ||
      userId != oldWidget.userId ||
      playlists != oldWidget.playlists;
}

class CardMenuAction {
  const CardMenuAction({
    required this.action,
    required this.label,
    required this.icon,
    this.danger = false,
    this.separated = false,
  });

  final CardAction action;
  final String label;
  final IconData icon;
  final bool danger;
  final bool separated;
}

bool hasCardMenu(TitleKind kind, {required bool dismissible}) =>
    dismissible || kind.isWatchTarget;

List<CardMenuAction> cardMenuActions(
  ItemState state, {
  required bool dismissible,
  bool playlists = false,
  bool queued = false,
}) => <CardMenuAction>[
  if (state.title.type.isWatchTarget)
    CardMenuAction(
      action: CardAction.watched,
      label: state.watched ? Strings.markUnwatched : Strings.markWatched,
      icon: Icons.check,
    ),
  if (state.title.type.isLeaf) ...<CardMenuAction>[
    if (state.watchlisted)
      const CardMenuAction(
        action: CardAction.watchlist,
        label: Strings.removeWatchlist,
        icon: Icons.close,
        danger: true,
      )
    else
      const CardMenuAction(
        action: CardAction.watchlist,
        label: Strings.addWatchlist,
        icon: Icons.bookmark_border,
      ),
    if (state.favorite)
      const CardMenuAction(
        action: CardAction.favorite,
        label: Strings.removeFavorite,
        icon: Icons.close,
        danger: true,
      )
    else
      const CardMenuAction(
        action: CardAction.favorite,
        label: Strings.addFavorite,
        icon: Icons.favorite_border,
      ),
  ],
  if (playlists && state.title.type.isWatchTarget)
    const CardMenuAction(
      action: CardAction.playlist,
      label: Strings.addToPlaylistEllipsis,
      icon: Icons.playlist_add,
    ),
  if (queued)
    const CardMenuAction(
      action: CardAction.dequeue,
      label: Strings.removeFromQueue,
      icon: Icons.playlist_remove,
      danger: true,
      separated: true,
    ),
  if (dismissible)
    const CardMenuAction(
      action: CardAction.dismiss,
      label: Strings.dismissResume,
      icon: Icons.close,
      danger: true,
      separated: true,
    ),
];

List<PopupMenuEntry<CardAction>> cardMenuEntries(
  ItemState state, {
  required bool dismissible,
  bool playlists = false,
  bool queued = false,
}) => <PopupMenuEntry<CardAction>>[
  for (final CardMenuAction a in cardMenuActions(
    state,
    dismissible: dismissible,
    playlists: playlists,
    queued: queued,
  )) ...<PopupMenuEntry<CardAction>>[
    if (a.separated)
      const PopupMenuDivider(
        indent: kCardMenuDividerInset,
        endIndent: kCardMenuDividerInset,
      ),
    PopupMenuItem<CardAction>(
      value: a.action,
      padding: const EdgeInsets.symmetric(horizontal: Space.s3),
      child: MenuRow(icon: a.icon, label: a.label, danger: a.danger),
    ),
  ],
];

class MenuRow extends StatelessWidget {
  const MenuRow({
    super.key,
    required this.icon,
    required this.label,
    this.danger = false,
  });

  final IconData icon;
  final String label;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Color tint = danger ? t.danger : t.muted;
    return Row(
      children: <Widget>[
        SizedBox(
          width: 24,
          height: 24,
          child: Center(child: Icon(icon, size: 16, color: tint)),
        ),
        const SizedBox(width: Space.s2),
        Expanded(
          child: Text(
            label,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: danger ? t.danger : t.text, fontSize: 14),
          ),
        ),
      ],
    );
  }
}

ItemState knownState(CatalogCard card) => ItemState(
  title: card.ref,
  watched: card.watched,
  watchlisted: card.watchlisted,
  favorite: card.favorite,
  progressPercent: card.progressPercent ?? 0,
);

void rememberState(CatalogCard card, ItemState state) {
  card.watched = state.watched;
  card.watchlisted = state.watchlisted;
  card.favorite = state.favorite;
  card.stateKnown = true;
}

RelativeRect menuPositionAt(Offset globalPosition, Size overlay) =>
    RelativeRect.fromLTRB(
      globalPosition.dx,
      globalPosition.dy,
      overlay.width - globalPosition.dx,
      overlay.height - globalPosition.dy,
    );

Future<void> showCardMenu(
  BuildContext context, {
  required CatalogApi catalog,
  required String userId,
  required CatalogCard card,
  required Offset at,
  PlaylistApi? playlists,
  VoidCallback? onDismiss,
  VoidCallback? onDequeue,
  void Function(CardAction action)? onChanged,
}) async {
  final TitleKind kind = card.ref.type;
  final bool dismissible = onDismiss != null;
  final String? entryId = card.queueEntryId;
  if (!hasCardMenu(kind, dismissible: dismissible)) {
    return;
  }

  ItemState state = knownState(card);
  if (!card.stateKnown && kind.isLeaf) {
    try {
      state = await catalog.stateOne(userId, card.ref);
      rememberState(card, state);
    } catch (_) {
      state = knownState(card);
    }
    if (!context.mounted) {
      return;
    }
  }

  final CardAction? chosen = await showMenu<CardAction>(
    context: context,
    position: menuPositionAt(at, MediaQuery.sizeOf(context)),
    items: cardMenuEntries(
      state,
      dismissible: dismissible,
      playlists: playlists != null,
      queued: entryId != null && (onDequeue != null || playlists != null),
    ),
    constraints: const BoxConstraints.tightFor(width: kCardMenuWidth),
    popUpAnimationStyle: AnimationStyle.noAnimation,
  );
  if (chosen == null || !context.mounted) {
    return;
  }

  if (chosen == CardAction.dismiss) {
    onDismiss?.call();
    return;
  }

  if (chosen == CardAction.dequeue && onDequeue != null) {
    onDequeue();
    return;
  }

  if (playlists != null) {
    if (chosen == CardAction.playlist) {
      if (await addToPlaylist(
        context,
        playlists: playlists,
        userId: userId,
        title: card.ref,
      )) {
        onChanged?.call(CardAction.playlist);
      }
      return;
    }
    if (chosen == CardAction.dequeue && entryId != null) {
      await _dequeue(
        context,
        playlists: playlists,
        userId: userId,
        card: card,
        entryId: entryId,
        onChanged: onChanged,
      );
      return;
    }
  }

  await _run(
    context,
    catalog: catalog,
    userId: userId,
    card: card,
    state: state,
    action: chosen,
    onChanged: onChanged,
  );
}

Future<void> _dequeue(
  BuildContext context, {
  required PlaylistApi playlists,
  required String userId,
  required CatalogCard card,
  required String entryId,
  void Function(CardAction action)? onChanged,
}) async {
  try {
    await playlists.dequeue(userId, entryId);
    if (!context.mounted) {
      return;
    }
    Toasts.of(
      context,
    ).success(Strings.toastRemovedFromQueue(card.title), emphasis: card.title);
    onChanged?.call(CardAction.dequeue);
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorRemove, e));
    }
  }
}

Future<void> _run(
  BuildContext context, {
  required CatalogApi catalog,
  required String userId,
  required CatalogCard card,
  required ItemState state,
  required CardAction action,
  void Function(CardAction action)? onChanged,
}) async {
  final TitleRef ref = card.ref;
  try {
    final String message;
    switch (action) {
      case CardAction.watched:
        final bool next = !state.watched;
        if (!await _confirmFanOut(context, next, ref.type)) {
          return;
        }
        await catalog.setWatched(userId, ref, next);
        card.watched = next;
        if (next) {
          card.watchlisted = false;
          card.progressPercent = null;
        }
        message = next
            ? Strings.toastMarkedWatched(card.title)
            : Strings.toastMarkedUnwatched(card.title);
      case CardAction.watchlist:
        final bool next = !state.watchlisted;
        if (next) {
          await catalog.addToWatchlist(userId, ref);
        } else {
          await catalog.removeFromWatchlist(userId, ref);
        }
        card.watchlisted = next;
        message = next
            ? Strings.toastAddedWatchlist(card.title)
            : Strings.toastRemovedWatchlist(card.title);
      case CardAction.favorite:
        final bool next = !state.favorite;
        if (next) {
          await catalog.addFavorite(userId, ref);
        } else {
          await catalog.removeFavorite(userId, ref);
        }
        card.favorite = next;
        message = next
            ? Strings.toastAddedFavorite(card.title)
            : Strings.toastRemovedFavorite(card.title);
      case CardAction.playlist:
      case CardAction.dequeue:
      case CardAction.dismiss:
        return;
    }
    if (!context.mounted) {
      return;
    }
    Toasts.of(context).success(message, emphasis: card.title);
    onChanged?.call(action);
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    Toasts.of(context).error(failureText(Strings.errorAction, e));
  }
}

String? watchedConfirmBody(TitleKind kind, {required bool watched}) =>
    switch (kind) {
      TitleKind.series =>
        watched ? Strings.confirmWatchedSeries : Strings.confirmUnwatchedSeries,
      TitleKind.season =>
        watched ? Strings.confirmWatchedSeason : Strings.confirmUnwatchedSeason,
      _ => null,
    };

Future<bool> _confirmFanOut(
  BuildContext context,
  bool next,
  TitleKind kind,
) async {
  final String? body = watchedConfirmBody(kind, watched: next);
  if (body == null) {
    return true;
  }
  final bool ok = await confirmDialog(
    context,
    title: next ? Strings.markWatched : Strings.markUnwatched,
    message: body,
    confirmLabel: next ? Strings.markWatched : Strings.markUnwatched,
    danger: false,
  );
  return ok && context.mounted;
}
