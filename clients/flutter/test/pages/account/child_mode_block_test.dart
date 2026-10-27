import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/child_mode_scope.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/child_mode_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

Widget _scoped(int? age, ValueChanged<int?> setAge) => _host(
  ChildModeScope(age: age, setAge: setAge, child: const ChildModeBlock()),
);

void main() {
  testWidgets('off says so and offers every age range', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_scoped(null, (_) {}));

    expect(find.text(Strings.childModeHeading), findsOneWidget);
    expect(find.text(Strings.childModeViewerAge), findsOneWidget);
    expect(find.text(Strings.childModeStatus(null)), findsOneWidget);

    await tester.tap(find.text(Strings.childModeOff));
    await tester.pumpAndSettle();

    expect(find.byType(MenuOption), findsNWidgets(5));
    for (final String label in <String>[
      'Under 8',
      '8 to 12',
      '13 to 16',
      '17 and over',
    ]) {
      expect(find.text(label), findsOneWidget);
    }
  });

  testWidgets('picking a range hands its youngest age to the app', (
    WidgetTester tester,
  ) async {
    final List<int?> picked = <int?>[];
    await tester.pumpWidget(_scoped(null, picked.add));

    await tester.tap(find.text(Strings.childModeOff));
    await tester.pumpAndSettle();
    await tester.tap(find.text('13 to 16'));
    await tester.pumpAndSettle();

    expect(picked, <int?>[13]);
  });

  testWidgets('on shows what that age may watch and can be turned off', (
    WidgetTester tester,
  ) async {
    final List<int?> picked = <int?>[];
    await tester.pumpWidget(_scoped(13, picked.add));

    expect(
      find.text(
        'Shows what a 13-year-old may watch: titles rated 13+ or lower. '
        'Unrated titles still show.',
      ),
      findsOneWidget,
    );

    await tester.tap(find.text('13 to 16'));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.childModeOff).last);
    await tester.pumpAndSettle();

    expect(picked, <int?>[null]);
  });

  testWidgets('the youngest range shows only all-ages titles', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_scoped(0, (_) {}));

    expect(find.text('Under 8'), findsOneWidget);
    expect(
      find.text(
        'Shows only titles rated for all ages. Unrated titles still show.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('an 8-year-old reads as one', (WidgetTester tester) async {
    await tester.pumpWidget(_scoped(8, (_) {}));

    expect(find.text('8 to 12'), findsOneWidget);
    expect(
      find.text(
        'Shows what an 8-year-old may watch: titles rated 8+ or lower. '
        'Unrated titles still show.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('the oldest range has no upper bound', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_scoped(17, (_) {}));

    expect(find.text('17 and over'), findsOneWidget);
    expect(childModeToast(17), 'Child mode: 17 and over.');
    expect(childModeToast(0), 'Child mode: under 8.');
    expect(childModeToast(null), 'Child mode off.');
  });

  testWidgets('without the app scope the block shows nothing', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(const ChildModeBlock()));

    expect(find.text(Strings.childModeHeading), findsNothing);
  });
}
