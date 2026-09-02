import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/series.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/catalog_list_page.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/page.dart';

final CatalogListSpec kSeriesList = CatalogListSpec(
  section: NavSection.series,
  errorText: Strings.couldNotLoadSeries,
  sortScope: 'series',
  genreKind: 'series',
  basePath: seriesListRoute(),
  navigationLabel: Strings.navigationSeries,
  emptyNoun: Strings.noSeriesFound,
  randomTooltip: Strings.randomEpisode,
  fetch: (CatalogApi api, ListQuery query, int offset) async {
    final Paged<Series> page = await api.series(
      offset: offset,
      limit: query.limit,
      sort: query.sort,
      order: query.order,
      genres: query.genres,
      library: query.library,
    );
    return (
      cards: page.items.map(CatalogCard.fromSeries).toList(),
      total: page.total,
      offset: page.offset,
    );
  },
  random: (CatalogApi api, ListQuery query) =>
      api.randomEpisode(genres: query.genres, library: query.library),
);

class SeriesPage extends StatelessWidget {
  const SeriesPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) =>
      CatalogListPage(api: api, spec: kSeriesList);
}
