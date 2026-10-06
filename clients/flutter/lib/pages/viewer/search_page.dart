import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/link_heading.dart';
import 'package:shadowmask/components/pagination.dart';
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
const int kSearchPreviewLimit = 24;

typedef _Section = ({TitleKind kind, String label, Paged<CatalogCard> page});

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
  late final Future<List<_Section>>? _future = _q.isEmpty ? null : _load();

  EmptyState _empty = const EmptyState(Strings.noResultsFound);

  bool get _all => _type == null;

  Future<List<_Section>> _load() async {
    final List<(TitleKind, String)> kinds = _blocks
        .where(((TitleKind, String) b) => _all || b.$1.name == _type)
        .toList();
    final List<Paged<CatalogCard>> pages =
        await Future.wait(<Future<Paged<CatalogCard>>>[
          for (final (TitleKind kind, String _) in kinds)
            _catalog.search(
              q: _q,
              type: kind.name,
              offset: _all ? 0 : _offset,
              limit: _all ? kSearchPreviewLimit : null,
            ),
        ]);
    final List<_Section> sections = <_Section>[
      for (int i = 0; i < kinds.length; i++)
        (kind: kinds[i].$1, label: kinds[i].$2, page: pages[i]),
    ];
    final List<CatalogCard> cards = <CatalogCard>[
      for (final _Section s in sections) ...s.page.items,
    ];
    await tagWatched(_catalog, widget.user.id, cards);
    if (cards.isEmpty) {
      _empty = await emptyState(
        widget.api,
        isAdmin: widget.user.isAdmin,
        noun: Strings.noResultsFound,
      );
    }
    return sections;
  }

  static const List<(TitleKind, String)> _blocks = <(TitleKind, String)>[
    (TitleKind.movie, Strings.typeMovie),
    (TitleKind.series, Strings.typeSeries),
    (TitleKind.episode, Strings.typeEpisode),
    (TitleKind.person, Strings.typePerson),
  ];

  static bool _isRail(TitleKind kind) =>
      kind == TitleKind.episode || kind == TitleKind.person;

  Widget _block(_Section section, {bool natural = false}) {
    final String count = Strings.countLabel(section.label, section.page.total);
    final bool more = _all && section.page.hasNext;
    final String title = more ? Strings.seeAllAfter(count) : count;
    final String? link = more ? Strings.seeAll : null;
    final String? route = more
        ? withQuery(searchRoute(), <String, String?>{
            'q': _q,
            'type': section.kind.name,
          })
        : null;
    final List<CatalogCard> cards = section.page.items;
    if (_isRail(section.kind)) {
      return CardRail(
        title: title,
        titleLink: link,
        titleRoute: route,
        cards: cards,
        imageBase: _catalog.imageBase,
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        LinkHeading(title: title, linkLabel: link, route: route),
        const SizedBox(height: Space.s3),
        CardGrid(
          cards: cards,
          imageBase: _catalog.imageBase,
          cardWidth: natural ? cardWidthFor(aspectOf(cards)) : null,
        ),
      ],
    );
  }

  double _naturalWidth(List<CatalogCard> group) {
    final double card = cardWidthFor(aspectOf(group));
    return group.length * card + (group.length - 1) * Space.s4;
  }

  Widget _results(List<_Section> sections, double available) {
    if (sections.length < 2 || available < Breakpoints.md) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          for (int i = 0; i < sections.length; i++) ...<Widget>[
            if (i > 0) const SizedBox(height: Space.s5),
            _block(sections[i]),
          ],
        ],
      );
    }
    return Wrap(
      spacing: Space.s4,
      runSpacing: Space.s5,
      children: <Widget>[
        for (final _Section section in sections)
          if (_naturalWidth(section.page.items) < available)
            SizedBox(
              width: _naturalWidth(section.page.items),
              child: _block(section, natural: true),
            )
          else
            SizedBox(width: available, child: _block(section)),
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
          buildBlock<List<_Section>>(
            future: _future,
            errorText: Strings.couldNotLoadSearch,
            loading: const SkeletonCards(),
            builder: (BuildContext context, List<_Section> sections) {
              final List<_Section> found = sections
                  .where((_Section s) => s.page.items.isNotEmpty)
                  .toList();
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  if (found.isEmpty)
                    EmptyNote(_empty)
                  else
                    LayoutBuilder(
                      builder:
                          (BuildContext context, BoxConstraints constraints) =>
                              _results(found, constraints.maxWidth),
                    ),
                  if (!_all && sections.length == 1)
                    Pagination(
                      basePath: searchRoute(),
                      params: <String, String?>{'q': _q, 'type': _type},
                      total: sections.single.page.total,
                      offset: sections.single.page.offset,
                      limit: sections.single.page.limit,
                      count: sections.single.page.items.length,
                    ),
                ],
              );
            },
          ),
      ],
    );
  }
}
