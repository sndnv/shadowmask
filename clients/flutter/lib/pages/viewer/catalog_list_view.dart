import 'package:flutter/material.dart';

import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/empty_note.dart';
import 'package:shadowmask/components/paged_card_grid.dart';
import 'package:shadowmask/components/section_toolbar.dart';
import 'package:shadowmask/pages/viewer/catalog_list_data.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/view/empty_state.dart';

class CatalogListView extends StatelessWidget {
  const CatalogListView({
    super.key,
    required this.basePath,
    required this.imageBase,
    required this.query,
    required this.data,
    required this.onLoadMore,
    this.crumbs,
    this.sortScope,
    this.loadingMore = false,
    this.moreFailed = false,
    this.toolbarTrailing,
  });

  final String basePath;
  final String imageBase;
  final ListQuery query;
  final CatalogListData data;
  final VoidCallback onLoadMore;
  final List<Crumb>? crumbs;
  final String? sortScope;
  final bool loadingMore;
  final bool moreFailed;
  final Widget? toolbarTrailing;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (crumbs != null) Breadcrumbs(crumbs!),
        SectionToolbar(
          basePath: basePath,
          genres: data.genres,
          sort: query.sort,
          order: query.order,
          selectedGenres: query.genres,
          library: query.library,
          libraries: data.libraries,
          sortScope: sortScope,
          limit: query.limit,
          trailing: toolbarTrailing,
        ),
        if (data.cards.isEmpty)
          EmptyNote(data.empty ?? const EmptyState(''))
        else
          PagedCardGrid(
            cards: data.cards,
            imageBase: imageBase,
            remaining: data.total - (data.offset + data.cards.length),
            loading: loadingMore,
            failed: moreFailed,
            onLoad: onLoadMore,
          ),
      ],
    );
  }
}
