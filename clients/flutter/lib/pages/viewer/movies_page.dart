import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/viewer/catalog_list_page.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/page.dart';

final CatalogListSpec kMoviesList = CatalogListSpec(
  section: NavSection.movies,
  errorText: Strings.couldNotLoadMovies,
  sortScope: 'movies',
  genreKind: 'movie',
  basePath: moviesRoute(),
  navigationLabel: Strings.navigationMovies,
  emptyNoun: Strings.noMoviesFound,
  randomTooltip: Strings.randomMovie,
  fetch: (CatalogApi api, ListQuery query, int offset) async {
    final Paged<Movie> page = await api.movies(
      offset: offset,
      limit: query.limit,
      sort: query.sort,
      order: query.order,
      genres: query.genres,
      library: query.library,
    );
    return (
      cards: page.items.map(CatalogCard.fromMovie).toList(),
      total: page.total,
      offset: page.offset,
    );
  },
  random: (CatalogApi api, ListQuery query) =>
      api.randomMovie(genres: query.genres, library: query.library),
);

class MoviesPage extends StatelessWidget {
  const MoviesPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) =>
      CatalogListPage(api: api, spec: kMoviesList);
}
