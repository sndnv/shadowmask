import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

void main() {
  testWidgets('tapping the help icon opens the explanation', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const FieldHelp(title: 'Watcher', body: 'How new files are noticed.'),
      ),
    );

    expect(find.text('How new files are noticed.'), findsNothing);

    await tester.tap(find.byIcon(Icons.help_outline));
    await tester.pumpAndSettle();

    expect(find.text('How new files are noticed.'), findsOneWidget);
    expect(find.text('Watcher'), findsOneWidget);
  });

  testWidgets('the explanation sits inside a selectable region', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const FieldHelp(title: 'Watcher', body: 'How new files are noticed.'),
      ),
    );

    await tester.tap(find.byIcon(Icons.help_outline));
    await tester.pumpAndSettle();

    expect(
      find.ancestor(
        of: find.text('How new files are noticed.'),
        matching: find.byType(SelectionArea),
      ),
      findsOneWidget,
    );
  });

  testWidgets('the help icon tooltips What is this', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(const FieldHelp(title: 'Roots', body: 'Folder list.')),
    );

    expect(
      tester.widget<Tooltip>(find.byType(Tooltip)).message,
      Strings.whatIsThis,
    );
  });

  testWidgets('a FieldLabel without help shows no icon', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(const FieldLabel(label: 'Kind', child: Text('body'))),
    );

    expect(find.text('Kind'), findsOneWidget);
    expect(find.text('body'), findsOneWidget);
    expect(find.byIcon(Icons.help_outline), findsNothing);
  });

  testWidgets('a FieldLabel with help titles the dialog with its own label', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const FieldLabel(
          label: 'Scan schedule',
          help: 'How often it rescans.',
          child: Text('body'),
        ),
      ),
    );

    await tester.tap(find.byIcon(Icons.help_outline));
    await tester.pumpAndSettle();

    expect(find.text('How often it rescans.'), findsOneWidget);
    expect(find.text('Scan schedule'), findsNWidgets(2));
  });
}
