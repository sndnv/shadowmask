import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';

class LibraryToggles extends StatefulWidget {
  const LibraryToggles({
    super.key,
    required this.catalog,
    required this.userId,
    required this.ref,
    required this.title,
    this.initialWatched = false,
    this.initialWatchlisted = false,
    this.initialFavorite = false,
    this.showWatchlistFavorite = true,
    this.compact = false,
    this.leading,
    this.episodeCount = 0,
    this.onWatchedChanged,
  });

  final CatalogApi catalog;
  final String userId;
  final TitleRef ref;
  final String title;
  final bool initialWatched;
  final bool initialWatchlisted;
  final bool initialFavorite;
  final bool showWatchlistFavorite;
  final bool compact;
  final Widget? leading;
  final int episodeCount;
  final ValueChanged<bool>? onWatchedChanged;

  @override
  State<LibraryToggles> createState() => _LibraryTogglesState();
}

class _LibraryTogglesState extends State<LibraryToggles> {
  late bool _watched = widget.initialWatched;
  late bool _watchlisted = widget.initialWatchlisted;
  late bool _favorite = widget.initialFavorite;
  bool _busyWatched = false;
  bool _busyWatchlist = false;
  bool _busyFavorite = false;

  @override
  void didUpdateWidget(LibraryToggles oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.initialWatched != oldWidget.initialWatched) {
      _watched = widget.initialWatched;
    }
    if (widget.initialWatchlisted != oldWidget.initialWatchlisted) {
      _watchlisted = widget.initialWatchlisted;
    }
    if (widget.initialFavorite != oldWidget.initialFavorite) {
      _favorite = widget.initialFavorite;
    }
  }

  Future<bool> _confirmFanOut(bool next) async {
    if (widget.episodeCount < 2) {
      return true;
    }
    return confirmDialog(
      context,
      title: next ? Strings.markWatched : Strings.markUnwatched,
      message: next
          ? Strings.confirmWatchedBody(widget.episodeCount)
          : Strings.confirmUnwatchedBody(widget.episodeCount),
      confirmLabel: next ? Strings.markWatched : Strings.markUnwatched,
      danger: false,
    );
  }

  Future<void> _toggleWatched() async {
    final bool next = !_watched;
    if (!await _confirmFanOut(next)) {
      return;
    }
    if (!mounted) {
      return;
    }
    setState(() => _busyWatched = true);
    try {
      await widget.catalog.setWatched(widget.userId, widget.ref, next);
      if (!mounted) {
        return;
      }
      setState(() {
        _watched = next;
        if (next) {
          _watchlisted = false;
        }
      });
      Toasts.of(context).success(
        next
            ? Strings.toastMarkedWatched(widget.title)
            : Strings.toastMarkedUnwatched(widget.title),
        emphasis: widget.title,
      );
      widget.onWatchedChanged?.call(next);
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busyWatched = false);
      }
    }
  }

  Future<void> _toggleWatchlist() async {
    final bool next = !_watchlisted;
    setState(() => _busyWatchlist = true);
    try {
      if (next) {
        await widget.catalog.addToWatchlist(widget.userId, widget.ref);
      } else {
        await widget.catalog.removeFromWatchlist(widget.userId, widget.ref);
      }
      if (!mounted) {
        return;
      }
      setState(() => _watchlisted = next);
      Toasts.of(context).success(
        next
            ? Strings.toastAddedWatchlist(widget.title)
            : Strings.toastRemovedWatchlist(widget.title),
        emphasis: widget.title,
      );
    } catch (e) {
      if (mounted) {
        Toasts.of(
          context,
        ).error(failureText(next ? Strings.errorAdd : Strings.errorRemove, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busyWatchlist = false);
      }
    }
  }

  Future<void> _toggleFavorite() async {
    final bool next = !_favorite;
    setState(() => _busyFavorite = true);
    try {
      if (next) {
        await widget.catalog.addFavorite(widget.userId, widget.ref);
      } else {
        await widget.catalog.removeFavorite(widget.userId, widget.ref);
      }
      if (!mounted) {
        return;
      }
      setState(() => _favorite = next);
      Toasts.of(context).success(
        next
            ? Strings.toastAddedFavorite(widget.title)
            : Strings.toastRemovedFavorite(widget.title),
        emphasis: widget.title,
      );
    } catch (e) {
      if (mounted) {
        Toasts.of(
          context,
        ).error(failureText(next ? Strings.errorAdd : Strings.errorRemove, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busyFavorite = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final bool compact =
            widget.compact || constraints.maxWidth < Breakpoints.sm;
        return Wrap(
          spacing: Space.s2,
          runSpacing: Space.s2,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: <Widget>[
            if (widget.leading != null) widget.leading!,
            ToggleButton(
              icon: Icons.check,
              filledIcon: Icons.check,
              label: Strings.watchedLabel,
              tooltip: _watched ? Strings.markUnwatched : Strings.markWatched,
              pressed: _watched,
              busy: _busyWatched,
              compact: compact,
              onToggle: _toggleWatched,
            ),
            if (widget.showWatchlistFavorite) ...<Widget>[
              ToggleButton(
                icon: Icons.bookmark_border,
                filledIcon: Icons.bookmark,
                label: Strings.watchlistLabel,
                tooltip: _watchlisted
                    ? Strings.removeWatchlist
                    : Strings.addWatchlist,
                pressed: _watchlisted,
                busy: _busyWatchlist,
                compact: compact,
                onToggle: _toggleWatchlist,
              ),
              ToggleButton(
                icon: Icons.favorite_border,
                filledIcon: Icons.favorite,
                label: Strings.favoriteLabel,
                tooltip: _favorite
                    ? Strings.removeFavorite
                    : Strings.addFavorite,
                pressed: _favorite,
                busy: _busyFavorite,
                compact: compact,
                onToggle: _toggleFavorite,
              ),
            ],
          ],
        );
      },
    );
  }
}
