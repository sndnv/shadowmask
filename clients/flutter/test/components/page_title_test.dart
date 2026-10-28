import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_title.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/pages/default/shell_scaffold.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/util/page_director.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../support/backdrops.dart';

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request _) async => http.Response('{}', 200)),
);

late List<String> _titles;

Route<void> _page(Widget body) => PageRouteBuilder<void>(
  transitionDuration: Duration.zero,
  reverseTransitionDuration: Duration.zero,
  pageBuilder: (BuildContext _, Animation<double> _, Animation<double> _) =>
      ShellScaffold(api: _api(), current: NavSection.movies, body: body),
);

Future<void> _pump(
  WidgetTester tester, {
  required Widget body,
  NavSection section = NavSection.movies,
  GlobalKey<NavigatorState>? navigator,
}) async {
  _titles = <String>[];
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.platform,
    (MethodCall call) async {
      if (call.method == 'SystemChrome.setApplicationSwitcherDescription') {
        final Map<dynamic, dynamic> args =
            call.arguments as Map<dynamic, dynamic>;
        _titles.add(args['label'] as String);
      }
      return null;
    },
  );
  addTearDown(
    () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      null,
    ),
  );

  final PageDirector<String?> titles = pageTitles();
  await tester.pumpWidget(
    PageTitleScope(
      titles: titles,
      child: ThemeScope(
        variant: AppThemeVariant.dark,
        setVariant: (_) {},
        child: MaterialApp(
          navigatorKey: navigator,
          title: Strings.appTitle,
          theme: buildTheme(AppThemeVariant.dark),
          navigatorObservers: <NavigatorObserver>[titles.observer],
          builder: (BuildContext _, Widget? child) =>
              TabTitle(titles: titles, child: child!),
          home: ShellScaffold(api: _api(), current: section, body: body),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the tab and the history entry name the page', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      body: Breadcrumbs(<Crumb>[
        const Crumb(Strings.navigationMovies, route: '/movies'),
        const Crumb('Blade Runner'),
      ]),
    );

    // Every entry read "Shadowmask" before, which is no help in a history
    // list. The deepest crumb is the page's own name.
    expect(_titles.last, Strings.documentTitle('Blade Runner'));
  });

  testWidgets('a page with no trail falls back to its section', (
    WidgetTester tester,
  ) async {
    await _pump(tester, body: const SizedBox.shrink());

    expect(_titles.last, Strings.documentTitle(Strings.navigationMovies));
  });

  testWidgets('the page title wins over the app title', (
    WidgetTester tester,
  ) async {
    await _pump(tester, body: Breadcrumbs(<Crumb>[const Crumb('Villeneuve')]));

    expect(_titles, contains(Strings.appTitle));
    expect(
      _titles.last,
      Strings.documentTitle('Villeneuve'),
      reason:
          'MaterialApp.title is an ancestor, so it must not have the last '
          'word',
    );
  });

  testWidgets('a page that loads under another page leaves the tab alone', (
    WidgetTester tester,
  ) async {
    final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
    final Completer<void> loaded = Completer<void>();
    await _pump(tester, body: const SizedBox.shrink(), navigator: navigator);

    navigator.currentState!.push(
      _page(
        FutureBuilder<void>(
          future: loaded.future,
          builder: (BuildContext _, AsyncSnapshot<void> snapshot) =>
              snapshot.connectionState == ConnectionState.done
              ? Breadcrumbs(<Crumb>[const Crumb('Blade Runner')])
              : const SizedBox.shrink(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    navigator.currentState!.push(
      _page(Breadcrumbs(<Crumb>[const Crumb('Villeneuve')])),
    );
    await tester.pumpAndSettle();
    loaded.complete();
    await tester.pumpAndSettle();

    expect(_titles.last, Strings.documentTitle('Villeneuve'));

    navigator.currentState!.pop();
    await tester.pumpAndSettle();

    expect(_titles.last, Strings.documentTitle('Blade Runner'));
  });

  testWidgets('after Retry on an unreachable server the tab names the page', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
    int asked = 0;
    final ApiClient api = ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request r) async {
        if (r.url.path != '/api/v1/users/self') {
          return http.Response('{}', 200);
        }
        if (asked++ == 0) {
          throw http.ClientException('down');
        }
        return http.Response(
          jsonEncode(<String, dynamic>{
            'id': 'u1',
            'username': 'pat',
            'role': 'user',
          }),
          200,
        );
      }),
    );
    await _pump(tester, body: const SizedBox.shrink(), navigator: navigator);

    navigator.currentState!.push(
      PageRouteBuilder<void>(
        transitionDuration: Duration.zero,
        reverseTransitionDuration: Duration.zero,
        pageBuilder:
            (BuildContext _, Animation<double> _, Animation<double> _) =>
                SectionPage(
                  api: api,
                  section: NavSection.movies,
                  bodyBuilder: (BuildContext _, SelfUser _) =>
                      const SizedBox.shrink(),
                ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      _titles.last,
      Strings.documentTitle(Strings.serverUnreachableHeading),
    );

    await tester.tap(find.text(Strings.retry));
    await tester.pumpAndSettle();

    expect(
      _titles.last,
      Strings.documentTitle(Strings.navigationMovies),
      reason: 'the logo reloads the page while the server is down',
    );
  });

  testWidgets('a page renamed as another opens over it leaves the tab alone', (
    WidgetTester tester,
  ) async {
    final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
    await _pump(
      tester,
      body: const SizedBox.shrink(key: Key('under')),
      section: NavSection.home,
      navigator: navigator,
    );

    final BuildContext under = tester.element(find.byKey(const Key('under')));
    navigator.currentState!.push(_page(const SizedBox.shrink()));
    PageTitleScope.maybeOf(
      under,
    )!.request(ModalRoute.of(under)!, Strings.documentTitle('Blade Runner'));
    await tester.pumpAndSettle();

    expect(_titles, isNot(contains(Strings.documentTitle('Blade Runner'))));
    expect(_titles.last, Strings.documentTitle(Strings.navigationMovies));
  });

  testWidgets('a page renamed under its dialog names the tab', (
    WidgetTester tester,
  ) async {
    final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
    final ValueNotifier<String> name = ValueNotifier<String>('Arrival');
    addTearDown(name.dispose);
    await _pump(
      tester,
      body: ValueListenableBuilder<String>(
        valueListenable: name,
        builder: (BuildContext _, String label, Widget? _) =>
            Breadcrumbs(<Crumb>[Crumb(label)]),
      ),
      navigator: navigator,
    );
    navigator.currentState!.push(
      RawDialogRoute<void>(
        pageBuilder:
            (BuildContext _, Animation<double> _, Animation<double> _) =>
                const SizedBox.shrink(),
      ),
    );
    await tester.pumpAndSettle();

    name.value = 'Blade Runner';
    await tester.pumpAndSettle();

    expect(
      _titles.last,
      Strings.documentTitle('Blade Runner'),
      reason: 'a title is renamed in a dialog over its page',
    );
  });

  testWidgets('back on a page, the tab names it again', (
    WidgetTester tester,
  ) async {
    final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
    await _pump(tester, body: const SizedBox.shrink(), navigator: navigator);

    navigator.currentState!.push(
      _page(Breadcrumbs(<Crumb>[const Crumb('Blade Runner')])),
    );
    await tester.pumpAndSettle();
    expect(_titles.last, Strings.documentTitle('Blade Runner'));

    navigator.currentState!.pop();
    await tester.pumpAndSettle();

    expect(_titles.last, Strings.documentTitle(Strings.navigationMovies));
  });
}
