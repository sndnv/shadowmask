import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/nav/open_section.dart';

Route<dynamic> _route(RouteSettings settings) => MaterialPageRoute<void>(
  settings: settings,
  builder: (BuildContext _) => Scaffold(body: Text(settings.name!)),
);

Future<NavigatorState> _trail(
  WidgetTester tester,
  String first,
  List<String> pages,
) async {
  final GlobalKey<NavigatorState> navigator = GlobalKey<NavigatorState>();
  await tester.pumpWidget(
    MaterialApp(
      navigatorKey: navigator,
      initialRoute: first,
      onGenerateRoute: _route,
      onGenerateInitialRoutes: (String route) => <Route<dynamic>>[
        _route(RouteSettings(name: route)),
      ],
    ),
  );
  for (final String page in pages) {
    navigator.currentState!.pushNamed(page);
    await tester.pumpAndSettle();
  }
  return navigator.currentState!;
}

List<String> _stack(WidgetTester tester) => <String>[
  for (final Text text in tester.widgetList<Text>(
    find.byType(Text, skipOffstage: false),
  ))
    text.data!,
];

Future<void> _open(WidgetTester tester, String from, String section) async {
  openSection(tester.element(find.text(from)), section);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a section opened over a trail keeps only Home beneath it', (
    WidgetTester tester,
  ) async {
    final NavigatorState nav = await _trail(tester, '/home', <String>[
      '/title?id=7',
      '/season?id=3',
    ]);

    await _open(tester, '/season?id=3', '/series');

    expect(_stack(tester), <String>['/home', '/series']);
    await nav.maybePop();
    await tester.pumpAndSettle();
    expect(_stack(tester), <String>['/home']);
  });

  testWidgets('pages opened from a section stack on it', (
    WidgetTester tester,
  ) async {
    final NavigatorState nav = await _trail(tester, '/home', <String>[]);
    await _open(tester, '/home', '/series');
    nav.pushNamed('/title?id=7');
    await tester.pumpAndSettle();
    nav.pushNamed('/season?id=3');
    await tester.pumpAndSettle();

    expect(_stack(tester), <String>[
      '/home',
      '/series',
      '/title?id=7',
      '/season?id=3',
    ]);
  });

  testWidgets('Home clears the stack', (WidgetTester tester) async {
    await _trail(tester, '/home', <String>['/movies', '/title?id=7']);

    await _open(tester, '/title?id=7', '/home');

    expect(_stack(tester), <String>['/home']);
  });

  testWidgets('with no Home beneath, a section stands alone', (
    WidgetTester tester,
  ) async {
    await _trail(tester, '/movies', <String>['/title?id=7']);

    await _open(tester, '/title?id=7', '/series');

    expect(_stack(tester), <String>['/series']);
  });
}
