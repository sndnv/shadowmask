import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/discovery/continue_feed.dart';
import 'package:shadowmask/model/discovery/hub.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';

const Set<String> _continueCovered = <String>{'continue_watching', 'on_deck'};
const String _watchlistHub = 'watchlist';

class HomePage extends StatelessWidget {
  const HomePage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) => SectionPage(
    api: api,
    section: NavSection.home,
    errorText: Strings.couldNotLoadHome,
    loading: const SkeletonPage(child: SkeletonCards()),
    bodyBuilder: (BuildContext context, SelfUser user) =>
        _HomeBody(api: api, user: user),
  );
}

class _HomeData {
  const _HomeData(this.feed, this.hubs, this.empty);

  final ContinueFeed feed;
  final List<Hub> hubs;
  final EmptyState? empty;
}

class _HomeBody extends StatefulWidget {
  const _HomeBody({required this.api, required this.user});

  final ApiClient api;
  final SelfUser user;

  @override
  State<_HomeBody> createState() => _HomeBodyState();
}

class _HomeBodyState extends State<_HomeBody>
    with RouteAware, Mutations<_HomeBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final PlaybackApi _playback = PlaybackApi(widget.api);
  late Future<_HomeData> _future = _load();
  final Set<String> _dismissed = <String>{};
  final Set<String> _unsaved = <String>{};

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  @override
  void didPopNext() {
    if (mounted) {
      _reload();
    }
  }

  void _reload() {
    setState(() {
      _dismissed.clear();
      _unsaved.clear();
      _future = _load();
    });
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    super.dispose();
  }

  Future<void> _removeFromWatchlist(CatalogCard card) => mutate(
    key: card.ref.key,
    () => _catalog.removeFromWatchlist(widget.user.id, card.ref),
    successText: Strings.toastRemovedTitle(card.title),
    emphasis: card.title,
    errorText: Strings.errorRemove,
    then: () => setState(() => _unsaved.add(card.ref.key)),
  );

  void _refreshAfterMenu(CatalogCard _, CardAction _) => unawaited(_refresh());

  Future<void> _refresh() async {
    final _HomeData data;
    try {
      data = await _load();
    } catch (_) {
      return;
    }
    if (mounted) {
      setState(() {
        _dismissed.clear();
        _unsaved.clear();
        _future = SynchronousFuture<_HomeData>(data);
      });
    }
  }

  List<CatalogCard> _railCards(Hub hub) => hub.id == _watchlistHub
      ? hub.items
            .where((CatalogCard c) => !_unsaved.contains(c.ref.key))
            .toList()
      : hub.items;

  Future<void> _dismiss(CatalogCard card) async {
    final String? versionId = card.dismissVersionId;
    if (versionId == null) {
      return;
    }
    await mutate(
      key: versionId,
      () => _playback.clearProgress(widget.user.id, versionId),
      successText: Strings.toastResumeDismissed,
      errorText: Strings.errorAction,
      then: () => setState(() => _dismissed.add(versionId)),
    );
  }

  Future<_HomeData> _load() async {
    final List<Object?> loaded = await Future.wait<Object?>(<Future<Object?>>[
      _catalog.hub(widget.user.id),
      _catalog.continueFeed(widget.user.id),
    ]);
    final List<Hub> hubs = loaded[0]! as List<Hub>;
    final ContinueFeed feed = loaded[1]! as ContinueFeed;
    final List<Hub> shown = hubs
        .where(
          (Hub h) => !_continueCovered.contains(h.id) && h.items.isNotEmpty,
        )
        .toList();
    final List<CatalogCard> all = <CatalogCard>[
      ...feed.continueWatching,
      ...feed.upNext,
      for (final Hub h in shown) ...h.items,
    ];
    await tagWatched(_catalog, widget.user.id, all);
    return _HomeData(
      feed,
      shown,
      feed.isEmpty && shown.isEmpty
          ? await emptyState(
              widget.api,
              isAdmin: widget.user.isAdmin,
              noun: Strings.nothingToShowYet,
            )
          : null,
    );
  }

  String _hubTitle(Hub h) => switch (h.id) {
    'watchlist' => Strings.onYourWatchlist,
    'recently_added_movies' => Strings.recentlyAddedMovies,
    'recently_added_shows' => Strings.recentlyAddedShows,
    _ => h.title,
  };

  @override
  Widget build(BuildContext context) {
    return buildBlock<_HomeData>(
      future: _future,
      errorText: Strings.couldNotLoadHome,
      onRetry: _reload,
      builder: (BuildContext context, _HomeData data) {
        final EmptyState? empty = data.empty;
        if (empty != null) {
          return EmptyNote(empty);
        }
        final List<CatalogCard> continueCards = data.feed.continueWatching
            .where(
              (CatalogCard c) =>
                  c.dismissVersionId == null ||
                  !_dismissed.contains(c.dismissVersionId),
            )
            .toList();
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(const <Crumb>[Crumb(Strings.navigationHome)]),
            if (continueCards.isNotEmpty)
              CardRail(
                title: Strings.continueWatching,
                cards: continueCards,
                imageBase: _catalog.imageBase,
                onDismiss: _dismiss,
                onMenuAction: _refreshAfterMenu,
                dismissBusy: (CatalogCard c) => busy(c.dismissVersionId),
              ),
            if (data.feed.upNext.isNotEmpty)
              CardRail(
                title: Strings.upNext,
                cards: data.feed.upNext,
                imageBase: _catalog.imageBase,
                onMenuAction: _refreshAfterMenu,
              ),
            for (final Hub h in data.hubs)
              if (_railCards(h).isNotEmpty)
                CardRail(
                  title: _hubTitle(h),
                  cards: _railCards(h),
                  imageBase: _catalog.imageBase,
                  onDismiss: h.id == _watchlistHub
                      ? _removeFromWatchlist
                      : null,
                  onMenuAction: _refreshAfterMenu,
                  dismissBusy: (CatalogCard c) => busy(c.ref.key),
                  dismissesByTitle: h.id == _watchlistHub,
                  dismissTooltip: Strings.removeWatchlist,
                ),
          ],
        );
      },
    );
  }
}
