import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/nav/browser_history.dart';

class _Browser {
  _Browser(this.entries, this.index);

  final List<RouteInformation> entries;
  int index;

  List<String> get urls => <String>[
    for (final RouteInformation entry in entries) entry.uri.toString(),
  ];

  Future<Object?> record(MethodCall call) async {
    if (call.method != 'routeInformationUpdated') {
      return null;
    }
    final Map<Object?, Object?> args = call.arguments as Map<Object?, Object?>;
    final RouteInformation entry = RouteInformation(
      uri: Uri.parse(args['uri']! as String),
      state: args['state'],
    );
    if (args['replace'] == true) {
      entries[index] = entry;
    } else {
      entries
        ..removeRange(index + 1, entries.length)
        ..add(entry);
      index++;
    }
    return null;
  }
}

class _Harness {
  _Harness(this.browser, this.provider, this.navigator);

  final _Browser browser;
  final BrowserHistoryProvider provider;
  final GlobalKey<NavigatorState> navigator;

  NavigatorState get nav => navigator.currentState!;
}

Route<dynamic> _route(RouteSettings settings) => MaterialPageRoute<void>(
  settings: settings,
  builder: (BuildContext context) => Scaffold(body: Text(settings.name!)),
);

RouteInformation _entry(String url, [num? stamp]) => RouteInformation(
  uri: Uri.parse(url),
  state: stamp == null ? null : <String, Object?>{'stamp': stamp},
);

Future<_Harness> _pump(
  WidgetTester tester, {
  String initial = '/home',
  List<RouteInformation>? history,
  int at = 0,
  List<NavigatorObserver> observers = const <NavigatorObserver>[],
}) async {
  final _Browser browser = _Browser(
    history ?? <RouteInformation>[_entry(initial)],
    at,
  );
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.navigation,
    browser.record,
  );
  addTearDown(
    () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.navigation,
      null,
    ),
  );
  final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
  final BrowserHistoryProvider provider = BrowserHistoryProvider(
    initialRoute: initial,
  );
  await tester.pumpWidget(
    MaterialApp.router(
      routerDelegate: BrowserHistory(
        navigatorKey: navigator,
        onGenerateRoute: _route,
        onGenerateInitialRoutes: (String route) => <Route<dynamic>>[
          _route(RouteSettings(name: route)),
        ],
        observers: observers,
      ),
      routeInformationParser: const BrowserHistoryParser(),
      routeInformationProvider: provider,
    ),
  );
  await tester.pumpAndSettle();
  return _Harness(browser, provider, navigator);
}

Future<void> _go(WidgetTester tester, Future<void> Function() move) async {
  move();
  await tester.pumpAndSettle();
}

Future<void> _jump(WidgetTester tester, _Harness h, int index) async {
  h.browser.index = index;
  await h.provider.didPushRouteInformation(h.browser.entries[index]);
  await tester.pumpAndSettle();
}

Future<void> _back(WidgetTester tester, _Harness h) =>
    _jump(tester, h, h.browser.index - 1);

Future<void> _forward(WidgetTester tester, _Harness h) =>
    _jump(tester, h, h.browser.index + 1);

int _depth(WidgetTester tester) =>
    tester.widgetList(find.byType(Scaffold, skipOffstage: false)).length;

class _Heard extends NavigatorObserver {
  final List<String> pushed = <String>[];

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    pushed.add(route.settings.name ?? '');
  }
}

void main() {
  testWidgets('a deep link opens its page with no extra entry', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester, initial: '/title?id=7');

    expect(find.text('/title?id=7'), findsOneWidget);
    expect(h.browser.urls, <String>['/title?id=7']);
  });

  testWidgets('a new page opens an entry, a query change on it replaces it', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/movies'));
    await _go(tester, () => h.nav.pushReplacementNamed('/movies?sort=year'));

    expect(h.browser.urls, <String>['/home', '/movies?sort=year']);
    expect(h.browser.index, 1);
  });

  testWidgets('a section switch opens an entry though it replaces the page', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushReplacementNamed('/movies'));
    await _go(tester, () => h.nav.pushReplacementNamed('/series'));

    expect(h.browser.urls, <String>['/home', '/movies', '/series']);
  });

  testWidgets('another id on the same page opens an entry', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester, initial: '/title?id=1');
    await _go(tester, () => h.nav.pushReplacementNamed('/title?id=2'));

    expect(h.browser.urls, <String>['/title?id=1', '/title?id=2']);
  });

  testWidgets('Back pops to the page underneath', (WidgetTester tester) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));

    await _back(tester, h);

    expect(find.text('/home'), findsOneWidget);
    expect(find.text('/title?id=1', skipOffstage: false), findsNothing);
    expect(h.nav.canPop(), isFalse);
    expect(h.browser.urls, <String>['/home', '/title?id=1']);
  });

  testWidgets('Back into a replaced page brings it back', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushReplacementNamed('/movies'));

    await _back(tester, h);

    expect(find.text('/home'), findsOneWidget);
    expect(find.text('/movies', skipOffstage: false), findsNothing);
    expect(h.browser.index, 0);
  });

  testWidgets('Forward after Back opens the page again', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));
    await _back(tester, h);

    await _forward(tester, h);

    expect(find.text('/title?id=1'), findsOneWidget);
    expect(h.nav.canPop(), isTrue);
    expect(h.browser.urls, <String>['/home', '/title?id=1']);

    await _back(tester, h);
    expect(find.text('/home'), findsOneWidget);
  });

  testWidgets('Back closes an open dialog and goes back', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));
    showDialog<void>(
      context: h.navigator.currentContext!,
      builder: (BuildContext context) => const Text('dialog'),
    );
    await tester.pumpAndSettle();
    expect(h.browser.urls, <String>['/home', '/title?id=1']);

    await _back(tester, h);

    expect(find.text('dialog'), findsNothing);
    expect(find.text('/home'), findsOneWidget);
  });

  testWidgets('signing in and out replace the entry', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester, initial: '/');
    await _go(tester, () => h.nav.pushReplacementNamed('/home'));
    expect(h.browser.urls, <String>['/home']);

    await _go(tester, () => h.nav.pushNamed('/movies'));
    await _go(
      tester,
      () => h.nav.pushNamedAndRemoveUntil('/', (Route<dynamic> _) => false),
    );

    expect(h.browser.urls, <String>['/home', '/']);
  });

  testWidgets('an entry with no stamp is shown in place of the page', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/movies'));
    h.browser.entries[0] = RouteInformation(uri: Uri.parse('/series'));

    await _back(tester, h);

    expect(find.text('/series'), findsOneWidget);
    expect(find.text('/movies', skipOffstage: false), findsNothing);
  });

  testWidgets('the device back button pops the navigator', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();

    expect(find.text('/home'), findsOneWidget);
    expect(h.nav.canPop(), isFalse);
  });

  testWidgets('Back over two entries pops to that page', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/movies'));
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));

    await _jump(tester, h, 0);

    expect(find.text('/home'), findsOneWidget);
    expect(_depth(tester), 1);
  });

  testWidgets('Forward over a replaced entry opens it on top', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushReplacementNamed('/movies'));
    await _back(tester, h);

    await _forward(tester, h);

    expect(find.text('/movies'), findsOneWidget);
    expect(_depth(tester), 2);
  });

  testWidgets('after a refresh Back and Forward swap the page, never stack', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(
      tester,
      initial: '/movies',
      history: <RouteInformation>[
        _entry('/home', 1),
        _entry('/movies'),
        _entry('/title?id=1', 3),
      ],
      at: 1,
    );

    for (final (Future<void> Function(WidgetTester, _Harness), String) step
        in <(Future<void> Function(WidgetTester, _Harness), String)>[
          (_back, '/home'),
          (_forward, '/movies'),
          (_forward, '/title?id=1'),
          (_back, '/movies'),
          (_back, '/home'),
          (_forward, '/movies'),
        ]) {
      await step.$1(tester, h);
      expect(find.text(step.$2), findsOneWidget);
      expect(_depth(tester), 1);
    }

    await _go(tester, () => h.nav.pushNamed('/title?id=9'));
    expect(h.browser.urls, <String>['/home', '/movies', '/title?id=9']);
    await _back(tester, h);
    await _forward(tester, h);
    expect(find.text('/title?id=9'), findsOneWidget);
    expect(_depth(tester), 2, reason: 'a new entry puts the order back');
  });

  testWidgets('the in-app back arrow opens an entry for the page it shows', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));

    h.nav.pop();
    await tester.pumpAndSettle();

    expect(h.browser.urls, <String>['/home', '/title?id=1', '/home']);
  });

  testWidgets('a new version in the player replaces its entry', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester, initial: '/title?id=1');
    await _go(tester, () => h.nav.pushNamed('/watch?version=a'));
    await _go(tester, () => h.nav.pushReplacementNamed('/watch?version=b'));

    expect(h.browser.urls, <String>['/title?id=1', '/watch?version=b']);
    await _back(tester, h);
    expect(find.text('/title?id=1'), findsOneWidget);
  });

  testWidgets('observers given to the history hear the navigator', (
    WidgetTester tester,
  ) async {
    final _Heard heard = _Heard();
    final _Harness h = await _pump(
      tester,
      observers: <NavigatorObserver>[heard],
    );
    await _go(tester, () => h.nav.pushNamed('/movies'));

    expect(heard.pushed, <String>['/home', '/movies']);
  });

  testWidgets('a stamp read back as a fraction still orders entries', (
    WidgetTester tester,
  ) async {
    final _Harness h = await _pump(tester);
    await _go(tester, () => h.nav.pushNamed('/title?id=1'));
    await _back(tester, h);
    final RouteInformation ahead = h.browser.entries[1];
    final Map<Object?, Object?> state = ahead.state! as Map<Object?, Object?>;
    h.browser.entries[1] = _entry(
      ahead.uri.toString(),
      (state['stamp']! as int).toDouble(),
    );

    await _forward(tester, h);

    expect(find.text('/title?id=1'), findsOneWidget);
    expect(_depth(tester), 2);
  });
}
