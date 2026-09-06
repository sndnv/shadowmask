import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/random_button.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/model/catalog/random_pick.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/viewer/catalog_list_data.dart';
import 'package:shadowmask/pages/viewer/catalog_list_view.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/pages/viewer/list_prefs_store.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';

typedef CardPage = ({List<CatalogCard> cards, int total, int offset});

class CatalogListSpec {
  const CatalogListSpec({
    required this.section,
    required this.errorText,
    required this.sortScope,
    required this.genreKind,
    required this.basePath,
    required this.navigationLabel,
    required this.emptyNoun,
    required this.randomTooltip,
    required this.fetch,
    required this.random,
  });

  final NavSection section;
  final String errorText;
  final String sortScope;
  final String genreKind;
  final String basePath;
  final String navigationLabel;
  final String emptyNoun;
  final String randomTooltip;
  final Future<CardPage> Function(CatalogApi api, ListQuery query, int offset)
  fetch;
  final Future<RandomPick> Function(CatalogApi api, ListQuery query) random;
}

class CatalogListPage extends StatelessWidget {
  const CatalogListPage({
    super.key,
    required this.api,
    required this.spec,
    required this.query,
  });

  final ApiClient api;
  final CatalogListSpec spec;
  final ListQuery query;

  @override
  Widget build(BuildContext context) => SectionPage(
    api: api,
    section: spec.section,
    errorText: spec.errorText,
    loading: const SkeletonPage(toolbar: true, child: SkeletonCards(count: 12)),
    bodyBuilder: (BuildContext context, SelfUser user) =>
        _Body(api: api, user: user, spec: spec, query: query),
  );
}

class _Body extends StatefulWidget {
  const _Body({
    required this.api,
    required this.user,
    required this.spec,
    required this.query,
  });

  final ApiClient api;
  final SelfUser user;
  final CatalogListSpec spec;
  final ListQuery query;

  @override
  State<_Body> createState() => _BodyState();
}

class _BodyState extends State<_Body> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late ListQuery _query = widget.query;
  late final Future<CatalogListData> _future = _load();

  final List<CatalogCard> _more = <CatalogCard>[];
  int _nextOffset = 0;
  int _total = 0;
  bool _loadingMore = false;
  bool _moreFailed = false;

  CatalogListSpec get _spec => widget.spec;

  Future<CardPage> _page(int offset) async {
    final CardPage page = await _spec.fetch(_catalog, _query, offset);
    await tagWatched(_catalog, widget.user.id, page.cards, withProgress: true);
    return page;
  }

  Future<CatalogListData> _load() async {
    _query = _query.withStored(await ListPrefsStore(_spec.sortScope).load());
    final Future<List<Genre>> pendingGenres = _catalog
        .genres(kind: _spec.genreKind)
        .catchError((Object _) => <Genre>[]);
    final CardPage page = await _page(_query.offset);
    final List<Genre> genres = await pendingGenres;
    _total = page.total;
    _nextOffset = page.offset + page.cards.length;
    final EmptyState? empty = page.cards.isEmpty
        ? await emptyState(
            widget.api,
            isAdmin: widget.user.isAdmin,
            noun: _spec.emptyNoun,
          )
        : null;
    return CatalogListData(
      total: page.total,
      offset: page.offset,
      genres: genres,
      cards: page.cards,
      empty: empty,
    );
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
      final CardPage page = await _page(_nextOffset);
      if (!mounted) {
        return;
      }
      setState(() {
        _more.addAll(page.cards);
        _total = page.total;
        _nextOffset = page.cards.isEmpty
            ? _total
            : page.offset + page.cards.length;
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
    return buildBlock<CatalogListData>(
      future: _future,
      errorText: _spec.errorText,
      builder: (BuildContext context, CatalogListData data) => CatalogListView(
        basePath: _spec.basePath,
        crumbs: <Crumb>[
          Crumb(Strings.countLabel(_spec.navigationLabel, data.total)),
        ],
        imageBase: _catalog.imageBase,
        query: _query,
        data: _more.isEmpty
            ? data
            : data.withCards(<CatalogCard>[...data.cards, ..._more]),
        sortScope: _spec.sortScope,
        onLoadMore: _loadMore,
        loadingMore: _loadingMore,
        moreFailed: _moreFailed,
        toolbarTrailing: RandomButton(
          tooltip: _spec.randomTooltip,
          pick: () => _spec.random(_catalog, _query),
        ),
      ),
    );
  }
}
