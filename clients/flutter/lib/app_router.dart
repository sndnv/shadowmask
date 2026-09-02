import 'package:fluro/fluro.dart';
import 'package:flutter/widgets.dart';

import 'package:shadowmask/api/api_client.dart';
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
import 'package:shadowmask/pages/entry/not_found_page.dart';
import 'package:shadowmask/pages/entry/sign_in_page.dart';
import 'package:shadowmask/pages/player/watch_page.dart';
import 'package:shadowmask/pages/viewer/collections_page.dart';
import 'package:shadowmask/pages/viewer/episode_page.dart';
import 'package:shadowmask/pages/viewer/home_page.dart';
import 'package:shadowmask/pages/viewer/movies_page.dart';
import 'package:shadowmask/pages/viewer/person_page.dart';
import 'package:shadowmask/pages/viewer/search_page.dart';
import 'package:shadowmask/pages/viewer/season_page.dart';
import 'package:shadowmask/pages/viewer/series_page.dart';
import 'package:shadowmask/pages/viewer/title_page.dart';
import 'package:shadowmask/pages/admin/version_page.dart';

class AppRouter {
  AppRouter(this.api) {
    _configure();
  }

  final ApiClient api;
  final FluroRouter router = FluroRouter();

  Handler _page(Widget Function() build) => Handler(
    handlerFunc: (BuildContext? context, Map<String, List<String>> params) =>
        build(),
  );

  void _define(String route, Widget Function() build) {
    router.define(
      route,
      handler: _page(build),
      transitionType: TransitionType.none,
    );
  }

  void _configure() {
    _define('/', () => SignInPage(api: api));
    _define('/home', () => HomePage(api: api));
    _define('/movies', () => MoviesPage(api: api));
    _define('/series', () => SeriesPage(api: api));
    _define('/collections', () => CollectionsPage(api: api));
    _define('/search', () => SearchPage(api: api));
    _define('/title', () => TitlePage(api: api));
    _define('/season', () => SeasonPage(api: api));
    _define('/episode', () => EpisodePage(api: api));
    _define('/person', () => PersonPage(api: api));
    _define('/version', () => VersionPage(api: api));
    _define('/watch', () => WatchPage(api: api));
    _define('/account', () => AccountPage(api: api));
    _define('/admin', () => DashboardPage(api: api));
    _define('/admin/jobs', () => JobsPage(api: api));
    _define('/admin/job', () => JobPage(api: api));
    _define('/admin/activity', () => ActivityPage(api: api));
    _define('/admin/libraries', () => LibrariesPage(api: api));
    _define('/admin/library', () => LibraryPage(api: api));
    _define('/admin/users', () => UsersPage(api: api));
    _define('/admin/user', () => UserPage(api: api));
    _define('/admin/versions', () => VersionsPage(api: api));
    _define('/admin/collections', () => CollectionsAdminPage(api: api));
    _define('/admin/fetch', () => FetchPage(api: api));
    router.notFoundHandler = _page(() => NotFoundPage(api: api));
  }
}
