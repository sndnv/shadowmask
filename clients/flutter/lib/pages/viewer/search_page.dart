import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/section_heading.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/viewer/catalog_support.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/empty_state.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';

const double kSearchControlHeight = 40;

class SearchPage extends StatelessWidget {
  const SearchPage({
    super.key,
    required this.api,
    this.query,
    this.type,
    this.offset = 0,
  });

  final ApiClient api;
  final String? query;
  final String? type;
  final int offset;

  @override
  Widget build(BuildContext context) => SectionPage(
    api: api,
    section: NavSection.search,
    errorText: Strings.couldNotLoadSearch,
    loading: const SkeletonPage(child: SkeletonCards()),
    bodyBuilder: (BuildContext context, SelfUser user) => _SearchBody(
      api: api,
      user: user,
      query: query,
      type: type,
      offset: offset,
    ),
  );
}

class _SearchBody extends StatefulWidget {
  const _SearchBody({
    required this.api,
    required this.user,
    required this.offset,
    this.query,
    this.type,
  });

  final ApiClient api;
  final SelfUser user;
  final String? query;
  final String? type;
  final int offset;

  @override
  State<_SearchBody> createState() => _SearchBodyState();
}

class _SearchBodyState extends State<_SearchBody> {
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final String _q = widget.query?.trim() ?? '';
  late final String? _type = widget.type;
  late final int _offset = widget.offset;
  late final TextEditingController _controller = TextEditingController(
    text: _q,
  );
  late final Future<Paged<CatalogCard>>? _future = _q.isEmpty ? null : _load();

  EmptyState _empty = const EmptyState(Strings.noResultsFound);

  Future<Paged<CatalogCard>> _load() async {
    final Paged<CatalogCard> page = await _catalog.search(
      q: _q,
      type: _type,
      offset: _offset,
    );
    await tagWatched(_catalog, widget.user.id, page.items);
    if (page.items.isEmpty) {
      _empty = await emptyState(
        widget.api,
        isAdmin: widget.user.isAdmin,
        noun: Strings.noResultsFound,
      );
    }
    return page;
  }

  static const List<(TitleKind, String)> _blocks = <(TitleKind, String)>[
    (TitleKind.movie, Strings.typeMovie),
    (TitleKind.series, Strings.typeSeries),
    (TitleKind.episode, Strings.typeEpisode),
    (TitleKind.person, Strings.typePerson),
  ];

  static bool _isRail(TitleKind kind) =>
      kind == TitleKind.episode || kind == TitleKind.person;

  List<(String, List<CatalogCard>, bool)> _groups(List<CatalogCard> cards) {
    final List<(String, List<CatalogCard>, bool)> out =
        <(String, List<CatalogCard>, bool)>[];
    for (final (TitleKind kind, String label) in _blocks) {
      final List<CatalogCard> group = cards
          .where((CatalogCard card) => card.ref.type == kind)
          .toList();
      if (group.isEmpty) {
        continue;
      }
      out.add((Strings.countLabel(label, group.length), group, _isRail(kind)));
    }
    final Set<TitleKind> known = _blocks
        .map(((TitleKind, String) block) => block.$1)
        .toSet();
    final List<CatalogCard> rest = cards
        .where((CatalogCard card) => !known.contains(card.ref.type))
        .toList();
    if (rest.isNotEmpty) {
      out.add(('', rest, false));
    }
    return out;
  }

  Widget _block(
    String title,
    List<CatalogCard> group,
    bool rail, {
    bool natural = false,
  }) {
    if (rail) {
      return CardRail(
        title: title,
        cards: group,
        imageBase: _catalog.imageBase,
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (title.isNotEmpty) ...<Widget>[
          SectionHeading(title: title),
          const SizedBox(height: Space.s3),
        ],
        CardGrid(
          cards: group,
          imageBase: _catalog.imageBase,
          cardWidth: natural ? cardWidthFor(aspectOf(group)) : null,
        ),
      ],
    );
  }

  double _naturalWidth(List<CatalogCard> group) {
    final double card = cardWidthFor(aspectOf(group));
    return group.length * card + (group.length - 1) * Space.s4;
  }

  Widget _results(List<CatalogCard> cards, double available) {
    final List<(String, List<CatalogCard>, bool)> groups = _groups(cards);
    if (groups.length < 2 || available < Breakpoints.md) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          for (int i = 0; i < groups.length; i++) ...<Widget>[
            if (i > 0) const SizedBox(height: Space.s5),
            _block(groups[i].$1, groups[i].$2, groups[i].$3),
          ],
        ],
      );
    }
    return Wrap(
      spacing: Space.s4,
      runSpacing: Space.s5,
      children: <Widget>[
        for (final (String title, List<CatalogCard> group, bool rail) in groups)
          if (_naturalWidth(group) < available)
            SizedBox(
              width: _naturalWidth(group),
              child: _block(title, group, rail, natural: true),
            )
          else
            SizedBox(width: available, child: _block(title, group, rail)),
      ],
    );
  }

  void _run({String? type}) {
    final String text = _controller.text.trim();
    Navigator.of(context).pushReplacementNamed(
      withQuery(searchRoute(), <String, String?>{
        'q': text,
        'type': type ?? _type,
      }),
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(const <Crumb>[Crumb(Strings.navigationSearch)]),
        LayoutBuilder(
          builder: (BuildContext context, BoxConstraints constraints) {
            final bool narrow = constraints.maxWidth < Breakpoints.sm;
            final Widget field = SizedBox(
              height: kSearchControlHeight,
              child: Semantics(
                label: Strings.searchFieldLabel,
                child: TextField(
                  controller: _controller,
                  autofocus: _q.isEmpty,
                  textInputAction: TextInputAction.search,
                  onSubmitted: (_) => _run(),
                  decoration: const InputDecoration(
                    isDense: true,
                    hintText: Strings.searchLabel,
                    contentPadding: EdgeInsets.symmetric(vertical: Space.s2),
                    prefixIcon: Icon(Icons.search, size: 18),
                    prefixIconConstraints: BoxConstraints(
                      minWidth: 34,
                      minHeight: kSearchControlHeight,
                    ),
                  ),
                ),
              ),
            );
            final Widget type = AppDropdown<String>(
              value: _type ?? 'all',
              label: Strings.searchTypeLabel,
              width: narrow ? null : 150,
              height: kSearchControlHeight,
              items: const <(String, String)>[
                ('all', Strings.typeAll),
                ('movie', Strings.typeMovie),
                ('series', Strings.typeSeries),
                ('episode', Strings.typeEpisode),
                ('person', Strings.typePerson),
              ],
              onChanged: (String v) => _run(type: v == 'all' ? '' : v),
            );
            if (narrow) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  field,
                  const SizedBox(height: Space.s3),
                  type,
                ],
              );
            }
            return Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                Expanded(child: field),
                const SizedBox(width: Space.s3),
                type,
              ],
            );
          },
        ),
        const SizedBox(height: Space.s5),
        if (_future == null)
          const EmptyNote(EmptyState(Strings.searchOpening))
        else
          buildBlock<Paged<CatalogCard>>(
            future: _future,
            errorText: Strings.couldNotLoadSearch,
            loading: const SkeletonCards(),
            builder: (BuildContext context, Paged<CatalogCard> page) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  if (page.items.isEmpty)
                    EmptyNote(_empty)
                  else
                    LayoutBuilder(
                      builder:
                          (BuildContext context, BoxConstraints constraints) =>
                              _results(page.items, constraints.maxWidth),
                    ),
                  Pagination(
                    basePath: searchRoute(),
                    params: <String, String?>{'q': _q, 'type': _type},
                    total: page.total,
                    offset: page.offset,
                    limit: page.limit,
                    count: page.items.length,
                  ),
                ],
              );
            },
          ),
      ],
    );
  }
}
