import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/shell_scaffold.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request _) async => http.Response('{}', 200)),
);

late List<String> _titles;

Future<void> _pump(
  WidgetTester tester, {
  required Widget body,
  NavSection section = NavSection.movies,
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

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        title: Strings.appTitle,
        theme: buildTheme(AppThemeVariant.dark),
        home: ShellScaffold(api: _api(), current: section, body: body),
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
}
