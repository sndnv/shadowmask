import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/title_heading.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/paged_card_grid.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/collection.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';

class CollectionsPage extends StatelessWidget {
  const CollectionsPage({
    super.key,
    required this.api,
    this.id,
    this.offset = 0,
    this.limit,
  });

  final ApiClient api;
  final String? id;
  final int offset;
  final int? limit;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.collections,
      errorText: id == null
          ? Strings.couldNotLoadCollections
          : Strings.couldNotLoadCollection,
      loading: const SkeletonPage(child: SkeletonCards()),
      bodyBuilder: (BuildContext context, SelfUser user) => id == null
          ? _CollectionsListBody(
              api: api,
              user: user,
              offset: offset,
              limit: limit,
            )
          : _CollectionDetailBody(api: api, id: id!, user: user),
    );
  }
}

class _CollectionsListBody extends StatefulWidget {
  const _CollectionsListBody({
    required this.api,
    required this.user,
    required this.offset,
    required this.limit,
  });

  final ApiClient api;
  final SelfUser user;
  final int offset;
  final int? limit;

  @override
  State<_CollectionsListBody> createState() => _CollectionsListBodyState();
}

class _CollectionsListBodyState extends State<_CollectionsListBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final int _offset = widget.offset;
  late final int? _limit = widget.limit;
  late final Future<Paged<Collection>> _future = _load();

  final List<CatalogCard> _more = <CatalogCard>[];
  int _nextOffset = 0;
  int _total = 0;
  bool _loadingMore = false;
  bool _moreFailed = false;
  EmptyState _empty = const EmptyState(Strings.noCollectionsFound);

  Future<Paged<Collection>> _load() async {
    final Paged<Collection> page = await _catalog.collections(
      offset: _offset,
      limit: _limit,
    );
    _total = page.total;
    _nextOffset = page.offset + page.items.length;
    if (page.items.isEmpty) {
      _empty = await emptyState(
        widget.api,
        isAdmin: widget.user.isAdmin,
        noun: Strings.noCollectionsFound,
      );
    }
    return page;
  }

  Future<void> _loadMore() async {
    if (_loadingMore || _nextOffset >= _total) {
      return;
    }
    setState(() {
      _loadingMore = true;
      _moreFailed = false;
    });
    try {
      final Paged<Collection> page = await _catalog.collections(
        offset: _nextOffset,
        limit: _limit,
      );
      final List<CatalogCard> cards = page.items
          .map(CatalogCard.fromCollection)
          .toList();
      if (!mounted) {
        return;
      }
      setState(() {
        _more.addAll(cards);
        _total = page.total;
        _nextOffset = cards.isEmpty ? _total : page.offset + cards.length;
        _loadingMore = false;
      });
    } catch (_) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loadingMore = false;
        _moreFailed = true;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<Paged<Collection>>(
      future: _future,
      errorText: Strings.couldNotLoadCollections,
      builder: (BuildContext context, Paged<Collection> page) {
        final List<CatalogCard> cards = <CatalogCard>[
          ...page.items.map(CatalogCard.fromCollection),
          ..._more,
        ];
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(
                Strings.countLabel(Strings.navigationCollections, page.total),
              ),
            ]),
            if (cards.isEmpty)
              EmptyNote(_empty)
            else
              PagedCardGrid(
                cards: cards,
                imageBase: _catalog.imageBase,
                remaining: page.total - (page.offset + cards.length),
                loading: _loadingMore,
                failed: _moreFailed,
                onLoad: _loadMore,
              ),
          ],
        );
      },
    );
  }
}

class _CollectionDetailBody extends StatefulWidget {
  const _CollectionDetailBody({
    required this.api,
    required this.id,
    required this.user,
  });

  final ApiClient api;
  final String id;
  final SelfUser user;

  @override
  State<_CollectionDetailBody> createState() => _CollectionDetailBodyState();
}

class _CollectionDetailBodyState extends State<_CollectionDetailBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final Future<(Collection, List<CatalogCard>)> _future = _load();

  Future<(Collection, List<CatalogCard>)> _load() async {
    final Collection c = await _catalog.collection(widget.id);
    final List<CatalogCard> cards = c.items.map(CatalogCard.fromMovie).toList();
    await tagWatched(_catalog, widget.user.id, cards);
    return (c, cards);
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<(Collection, List<CatalogCard>)>(
      future: _future,
      errorText: Strings.couldNotLoadCollection,
      builder: (BuildContext context, (Collection, List<CatalogCard>) data) {
        final Collection c = data.$1;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.navigationCollections, route: collectionsRoute()),
              Crumb(c.name),
            ]),
            PageBackdrop(artwork: c.artwork, imageBase: _catalog.imageBase),
            TitleHeading(
              title: c.name,
              trailing: data.$2.isEmpty
                  ? null
                  : RandomButton(
                      tooltip: Strings.randomInCollection,
                      pick: () => _catalog.randomInCollection(c.id),
                    ),
            ),
            if (c.overview != null) ...<Widget>[
              const SizedBox(height: Space.s3),
              Text(c.overview!, style: Theme.of(context).textTheme.bodyLarge),
            ],
            const SizedBox(height: Space.s5),
            if (data.$2.isEmpty)
              Text(
                Strings.noMoviesFound,
                style: TextStyle(color: context.tokens.muted),
              )
            else
              CardGrid(cards: data.$2, imageBase: _catalog.imageBase),
          ],
        );
      },
    );
  }
}
