import 'package:shadowmask/model/common/title_ref.dart';

String _enc(String s) => Uri.encodeComponent(s);

String homeRoute() => '/home';
String moviesRoute() => '/movies';
String seriesListRoute() => '/series';
String collectionsRoute() => '/collections';
String searchRoute() => '/search';

String adminRoute() => '/admin';
String adminJobsRoute() => '/admin/jobs';
String adminLibrariesRoute() => '/admin/libraries';
String adminUsersRoute() => '/admin/users';
String adminVersionsRoute() => '/admin/versions';
String adminCollectionsRoute() => '/admin/collections';
String adminActivityRoute() => '/admin/activity';
String adminFetchRoute() => '/admin/fetch';

String jobRoute(String id) => '/admin/job?id=${_enc(id)}';
String adminUserRoute(String id) => '/admin/user?id=${_enc(id)}';
String adminLibraryRoute(String id) => '/admin/library?id=${_enc(id)}';

String movieRoute(String id) => '/title?type=movie&id=${_enc(id)}';
String seriesRoute(String id) => '/title?type=series&id=${_enc(id)}';
String personRoute(String id) => '/person?id=${_enc(id)}';
String versionRoute(String id) => '/version?id=${_enc(id)}';
String collectionRoute(String id) => '/collections?id=${_enc(id)}';
String watchRoute(String versionId) => '/watch?version=${_enc(versionId)}';

String seasonRoute(String id, {String? series}) =>
    '/season?id=${_enc(id)}${series != null ? '&series=${_enc(series)}' : ''}';

String episodeRoute(String id, {String? series, String? season}) =>
    '/episode?id=${_enc(id)}'
    '${season != null ? '&season=${_enc(season)}' : ''}'
    '${series != null ? '&series=${_enc(series)}' : ''}';

String titleRoute(
  TitleKind kind,
  String id, {
  String? series,
  String? season,
}) => switch (kind) {
  TitleKind.movie => movieRoute(id),
  TitleKind.series => seriesRoute(id),
  TitleKind.season => seasonRoute(id, series: series),
  TitleKind.episode => episodeRoute(id, series: series, season: season),
  TitleKind.person => personRoute(id),
  TitleKind.collection => collectionRoute(id),
};

const Set<String> rootPaths = <String>{
  '/',
  '/home',
  '/movies',
  '/series',
  '/collections',
  '/search',
  '/admin',
  '/account',
};

bool isRootRoute(String? name) {
  if (name == null) {
    return false;
  }
  final Uri uri = Uri.parse(name);
  return rootPaths.contains(uri.path) && !uri.queryParameters.containsKey('id');
}

String withQuery(String path, Map<String, String?> params) {
  final String query = params.entries
      .where(
        (MapEntry<String, String?> e) => e.value != null && e.value!.isNotEmpty,
      )
      .map(
        (MapEntry<String, String?> e) =>
            '${Uri.encodeQueryComponent(e.key)}='
            '${Uri.encodeQueryComponent(e.value!)}',
      )
      .join('&');
  return query.isEmpty ? path : '$path?$query';
}
