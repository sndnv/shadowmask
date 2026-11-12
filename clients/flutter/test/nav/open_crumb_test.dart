import 'package:material_ui/material_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/nav/browser_history.dart';
import 'package:shadowmask/nav/open_section.dart';

class _Pops extends NavigatorObserver {
  int count = 0;

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    count++;
  }
}

class _Trail {
  _Trail(this.navigator, this.built, this.pops);

  final NavigatorState navigator;
  final List<String> built;
  final _Pops pops;
}

Future<_Trail> _trail(WidgetTester tester, List<String> pages) async {
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.navigation,
    (MethodCall _) async => null,
  );
  addTearDown(
    () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.navigation,
      null,
    ),
  );
  final List<String> built = <String>[];
  Route<dynamic> route(RouteSettings settings) {
    built.add(settings.name!);
    return MaterialPageRoute<void>(
      settings: settings,
      builder: (BuildContext _) => Scaffold(body: Text(settings.name!)),
    );
  }

  final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
  final _Pops pops = _Pops();
  await tester.pumpWidget(
    MaterialApp.router(
      routerDelegate: BrowserHistory(
        navigatorKey: navigator,
        onGenerateRoute: route,
        onGenerateInitialRoutes: (String name) => <Route<dynamic>>[
          route(RouteSettings(name: name)),
        ],
        observers: <NavigatorObserver>[pops],
      ),
      routeInformationParser: const BrowserHistoryParser(),
      routeInformationProvider: BrowserHistoryProvider(
        initialRoute: pages.first,
      ),
    ),
  );
  await tester.pumpAndSettle();
  for (final String page in pages.skip(1)) {
    navigator.currentState!.pushNamed(page);
    await tester.pumpAndSettle();
  }
  return _Trail(navigator.currentState!, built, pops);
}

List<String> _stack(WidgetTester tester) => <String>[
  for (final Text text in tester.widgetList<Text>(
    find.byType(Text, skipOffstage: false),
  ))
    text.data!,
];

Future<void> _crumb(WidgetTester tester, String from, String to) async {
  openCrumb(tester.element(find.text(from)), to);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a Home crumb clears the stack as the nav bar does', (
    WidgetTester tester,
  ) async {
    await _trail(tester, <String>[
      '/title?id=5',
      '/home',
      '/movies',
      '/title?id=7',
    ]);

    await _crumb(tester, '/title?id=7', '/home');

    expect(_stack(tester), <String>['/home']);
  });

  testWidgets('going round through a section crumb does not stack', (
    WidgetTester tester,
  ) async {
    final _Trail trail = await _trail(tester, <String>[
      '/home',
      '/admin',
      '/admin/users',
    ]);

    await _crumb(tester, '/admin/users', '/admin');
    trail.navigator.pushNamed('/admin/jobs');
    await tester.pumpAndSettle();
    await _crumb(tester, '/admin/jobs', '/admin');

    expect(_stack(tester), <String>['/home', '/admin']);
  });

  testWidgets('a crumb to a page below opens a fresh copy of it there', (
    WidgetTester tester,
  ) async {
    final _Trail trail = await _trail(tester, <String>[
      '/home',
      '/series',
      '/title?id=1',
      '/season?id=2',
      '/episode?id=3',
    ]);

    await _crumb(tester, '/episode?id=3', '/title?id=1');

    expect(_stack(tester), <String>['/home', '/series', '/title?id=1']);
    expect(
      trail.built.where((String name) => name == '/title?id=1'),
      hasLength(2),
      reason: 'the page is built again, so it shows what changed meanwhile',
    );
  });

  testWidgets('pages passed on the way are removed, not popped', (
    WidgetTester tester,
  ) async {
    final _Trail trail = await _trail(tester, <String>[
      '/home',
      '/series',
      '/title?id=1',
      '/watch?v=9',
      '/episode?id=3',
    ]);

    await _crumb(tester, '/episode?id=3', '/title?id=1');

    expect(_stack(tester), <String>['/home', '/series', '/title?id=1']);
    expect(
      trail.pops.count,
      0,
      reason: 'a pop tells the page beneath it is showing again',
    );
  });

  testWidgets('a crumb finds its page by path and id', (
    WidgetTester tester,
  ) async {
    await _trail(tester, <String>[
      '/home',
      '/series',
      '/episode?id=3&season=2',
      '/watch?v=1',
    ]);

    await _crumb(tester, '/watch?v=1', '/episode?id=3&series=9');

    expect(_stack(tester), <String>[
      '/home',
      '/series',
      '/episode?id=3&season=2',
    ]);
  });

  testWidgets('a section crumb returns to the list as it was left', (
    WidgetTester tester,
  ) async {
    await _trail(tester, <String>[
      '/home',
      '/movies?genres=drama',
      '/title?id=7',
    ]);

    await _crumb(tester, '/title?id=7', '/movies');

    expect(_stack(tester), <String>['/home', '/movies?genres=drama']);
  });

  testWidgets('a crumb to a page not below opens it on top', (
    WidgetTester tester,
  ) async {
    await _trail(tester, <String>['/home', '/search', '/episode?id=3']);

    await _crumb(tester, '/episode?id=3', '/season?id=2');

    expect(_stack(tester), <String>[
      '/home',
      '/search',
      '/episode?id=3',
      '/season?id=2',
    ]);
  });
}
