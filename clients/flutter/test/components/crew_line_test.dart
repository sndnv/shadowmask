import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/crew_line.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/detail_dimensions.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

Credit _credit(String id, String name, CreditRole role, int order) => Credit(
  person: PersonRef(id: id, name: name),
  role: role,
  order: order,
);

Future<void> _pump(
  WidgetTester tester,
  List<Credit> credits, {
  List<String>? pushed,
}) => tester.pumpWidget(
  ThemeScope(
    variant: AppThemeVariant.dark,
    setVariant: (_) {},
    child: MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      onGenerateRoute: (RouteSettings s) {
        final String? name = s.name;
        if (name != null && name != '/') {
          pushed?.add(name);
        }
        return MaterialPageRoute<void>(
          builder: (_) => Scaffold(
            body: name == null || name == '/'
                ? CrewLine(credits: credits)
                : const SizedBox.shrink(),
          ),
        );
      },
    ),
  ),
);

void main() {
  testWidgets('each name opens that person', (WidgetTester tester) async {
    final List<String> pushed = <String>[];
    await _pump(tester, <Credit>[
      _credit('d1', 'Dee', CreditRole.director, 0),
      _credit('w1', 'Wren', CreditRole.writer, 0),
    ], pushed: pushed);

    expect(find.text(Strings.directedBy), findsOneWidget);
    expect(find.text(Strings.writtenBy), findsOneWidget);

    await tester.tap(find.text('Wren'));
    await tester.pumpAndSettle();

    expect(pushed, <String>[personRoute('w1')]);
  });

  testWidgets('the names are accented and the role labels are not', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Credit>[_credit('d1', 'Dee', CreditRole.director, 0)]);

    const Tokens t = Tokens.dark;
    expect(tester.widget<Text>(find.text('Dee')).style?.color, t.accent);
    expect(
      tester.widget<Text>(find.text(Strings.directedBy)).style?.color,
      t.muted,
    );
  });

  testWidgets('several names in one role are separated by commas', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Credit>[
      _credit('w2', 'Wynn', CreditRole.writer, 1),
      _credit('w1', 'Wren', CreditRole.writer, 0),
    ]);

    // billing order decides the order, and only the last name loses its comma
    expect(find.text('Wren,'), findsOneWidget);
    expect(find.text('Wynn'), findsOneWidget);
  });

  testWidgets('a title with only actors shows no line', (
    WidgetTester tester,
  ) async {
    await _pump(tester, <Credit>[_credit('p1', 'Ada', CreditRole.actor, 0)]);

    expect(find.byType(Text), findsNothing);
  });

  test('a person credited twice in one role is listed once', () {
    final List<CrewGroup> groups = crewGroups(<Credit>[
      _credit('d1', 'Dee', CreditRole.director, 0),
      _credit('d1', 'Dee', CreditRole.director, 4),
    ]);

    expect(groups.length, 1);
    expect(groups.first.people.length, 1);
  });
}
