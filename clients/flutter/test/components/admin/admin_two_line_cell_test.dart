import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_two_line_cell.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(WidgetTester tester, Widget cell) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: Center(child: SizedBox(width: 120, child: cell)),
        ),
      ),
    ),
  );
}

Finder _texts() => find.descendant(
  of: find.byType(AdminTwoLineCell),
  matching: find.byType(Text),
);

void main() {
  testWidgets('the second line sits under the first, muted', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTwoLineCell.text('Alpha', below: 'Movie · 2 versions'),
    );

    expect(
      tester.getTopLeft(find.text('Alpha')).dy,
      lessThan(tester.getTopLeft(find.text('Movie · 2 versions')).dy),
    );
    final Finder below = find.text('Movie · 2 versions');
    expect(
      tester.widget<Text>(below).style,
      Theme.of(
        tester.element(below),
      ).textTheme.bodySmall?.copyWith(color: Tokens.dark.muted),
    );
  });

  testWidgets('with nothing below, the cell is its first line alone', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AdminTwoLineCell.text('Alpha'));

    expect(_texts(), findsOneWidget);
    expect(find.text('Alpha'), findsOneWidget);
  });

  testWidgets('each line stays on one line and ends in an ellipsis', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AdminTwoLineCell.text(
        'A name far too long for the cell it sits in',
        below: 'A context line far too long for the cell as well',
      ),
    );

    for (final Text text in tester.widgetList<Text>(_texts())) {
      expect(text.maxLines, 1);
      expect(text.overflow, TextOverflow.ellipsis);
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('the first line can be any widget', (WidgetTester tester) async {
    await _pump(
      tester,
      const AdminTwoLineCell(first: Icon(Icons.link), below: 'sam · playing'),
    );

    expect(find.byIcon(Icons.link), findsOneWidget);
    expect(find.text('sam · playing'), findsOneWidget);
  });
}
