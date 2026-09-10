import 'package:fluro/fluro.dart';
import 'package:flutter/widgets.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/nav/route_args.dart';
import 'package:shadowmask/pages/account/account_page.dart';
import 'package:shadowmask/pages/admin/activity_page.dart';
import 'package:shadowmask/pages/admin/collections_admin_page.dart';
import 'package:shadowmask/pages/admin/dashboard_page.dart';
import 'package:shadowmask/pages/admin/fetch_page.dart';
import 'package:shadowmask/pages/admin/job_page.dart';
import 'package:shadowmask/pages/admin/jobs_page.dart';
import 'package:shadowmask/pages/admin/libraries_page.dart';
import 'package:shadowmask/pages/admin/library_page.dart';
import 'package:shadowmask/pages/admin/user_page.dart';
import 'package:shadowmask/pages/admin/users_page.dart';
import 'package:shadowmask/pages/admin/versions_page.dart';
import 'package:shadowmask/pages/entry/link_code_page.dart';
import 'package:shadowmask/pages/entry/not_found_page.dart';
import 'package:shadowmask/pages/entry/sign_in_page.dart';
import 'package:shadowmask/pages/player/watch_page.dart';
import 'package:shadowmask/pages/viewer/collections_page.dart';
import 'package:shadowmask/pages/viewer/episode_page.dart';
import 'package:shadowmask/pages/viewer/home_page.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shadowmask/pages/viewer/movies_page.dart';
import 'package:shadowmask/pages/viewer/person_page.dart';
import 'package:shadowmask/pages/viewer/search_page.dart';
import 'package:shadowmask/pages/viewer/season_page.dart';
import 'package:shadowmask/pages/viewer/series_page.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/pages/admin/version_page.dart';
import 'package:shadowmask/view/playback_controls.dart';

class AppRouter {
  AppRouter(this.api) {
    _configure();
  }

  final ApiClient api;
  final FluroRouter router = FluroRouter();

  Handler _page(Widget Function(Map<String, String> args) build) => Handler(
    handlerFunc: (BuildContext? context, Map<String, List<String>> params) =>
        build(routeArgs(params)),
  );

  void _define(String route, Widget Function(Map<String, String> args) build) {
    router.define(
      route,
      handler: _page(build),
      transitionType: TransitionType.none,
    );
  }

  void _configure() {
    _define('/', (Map<String, String> a) => SignInPage(api: api));
    _define('/link', (Map<String, String> a) => LinkCodePage(api: api));
    _define('/home', (Map<String, String> a) => HomePage(api: api));
    _define(
      '/movies',
      (Map<String, String> a) =>
          MoviesPage(api: api, query: ListQuery.fromArgs(a)),
    );
    _define(
      '/series',
      (Map<String, String> a) =>
          SeriesPage(api: api, query: ListQuery.fromArgs(a)),
    );
    _define(
      '/collections',
      (Map<String, String> a) => CollectionsPage(
        api: api,
        id: a['id'],
        offset: offsetArg(a),
        limit: pageSizeArg(a),
      ),
    );
    _define(
      '/search',
      (Map<String, String> a) => SearchPage(
        api: api,
        query: a['q'],
        type: a['type'],
        offset: offsetArg(a),
      ),
    );
    _define(
      '/title',
      (Map<String, String> a) =>
          TitlePage(api: api, kind: a['type'] ?? 'movie', id: a['id'] ?? ''),
    );
    _define(
      '/season',
      (Map<String, String> a) =>
          SeasonPage(api: api, id: a['id'] ?? '', series: a['series']),
    );
    _define(
      '/episode',
      (Map<String, String> a) => EpisodePage(
        api: api,
        id: a['id'] ?? '',
        season: a['season'],
        series: a['series'],
      ),
    );
    _define(
      '/person',
      (Map<String, String> a) => PersonPage(api: api, id: a['id'] ?? ''),
    );
    _define(
      '/version',
      (Map<String, String> a) => VersionPage(api: api, id: a['id'] ?? ''),
    );
    _define(
      '/watch',
      (Map<String, String> a) => WatchPage(
        api: api,
        versionId: a['version'] ?? '',
        controls: PlaybackControls.fromQuery(a),
      ),
    );
    _define('/account', (Map<String, String> a) => AccountPage(api: api));
    _define('/admin', (Map<String, String> a) => DashboardPage(api: api));
    _define('/admin/jobs', (Map<String, String> a) => JobsPage(api: api));
    _define(
      '/admin/job',
      (Map<String, String> a) => JobPage(api: api, id: a['id']),
    );
    _define(
      '/admin/activity',
      (Map<String, String> a) => ActivityPage(api: api, offset: offsetArg(a)),
    );
    _define(
      '/admin/libraries',
      (Map<String, String> a) => LibrariesPage(api: api),
    );
    _define(
      '/admin/library',
      (Map<String, String> a) => LibraryPage(api: api, libraryId: a['id']),
    );
    _define(
      '/admin/users',
      (Map<String, String> a) => UsersPage(api: api, offset: offsetArg(a)),
    );
    _define(
      '/admin/user',
      (Map<String, String> a) => UserPage(api: api, id: a['id']),
    );
    _define(
      '/admin/versions',
      (Map<String, String> a) => VersionsPage(api: api, offset: offsetArg(a)),
    );
    _define(
      '/admin/collections',
      (Map<String, String> a) => CollectionsAdminPage(api: api),
    );
    _define('/admin/fetch', (Map<String, String> a) => FetchPage(api: api));
    router.notFoundHandler = _page(
      (Map<String, String> a) => NotFoundPage(api: api),
    );
  }
}
