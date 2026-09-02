import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

void main() {
  testWidgets('the primary action is filled and the rest are outlined', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        PageActions(<PageAction>[
          PageAction(icon: Icons.refresh, label: 'Refresh', onPressed: () {}),
          PageAction(
            icon: Icons.add,
            label: 'Create library',
            primary: true,
            onPressed: () {},
          ),
        ]),
      ),
    );

    expect(find.widgetWithText(FilledButton, 'Create library'), findsOneWidget);
    expect(find.widgetWithText(OutlinedButton, 'Refresh'), findsOneWidget);
  });

  testWidgets('every action carries a visible label, never a bare icon', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        PageActions(<PageAction>[
          PageAction(
            icon: Icons.delete_outline,
            label: 'Delete library',
            danger: true,
            onPressed: () {},
          ),
        ]),
      ),
    );

    expect(find.byType(IconButton), findsNothing);
    expect(find.text('Delete library'), findsOneWidget);
    expect(find.byType(Tooltip), findsNothing);
  });

  testWidgets('three actions wrap onto a phone instead of overflowing', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(
        PageActions(<PageAction>[
          PageAction(icon: Icons.refresh, label: 'Refresh', onPressed: () {}),
          PageAction(
            icon: Icons.sync,
            label: 'Rescan library',
            onPressed: () {},
          ),
          PageAction(
            icon: Icons.delete_outline,
            label: 'Delete library',
            danger: true,
            onPressed: () {},
          ),
        ]),
      ),
    );

    expect(tester.takeException(), isNull);
    final Rect first = tester.getRect(find.text('Refresh'));
    final Rect last = tester.getRect(find.text('Delete library'));
    expect(
      last.top,
      greaterThan(first.top),
      reason: 'a single row of three would run off the right edge',
    );
  });

  testWidgets('a null callback disables the button', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        PageActions(const <PageAction>[
          PageAction(icon: Icons.add, label: 'Create user', onPressed: null),
        ]),
      ),
    );

    expect(
      tester
          .widget<OutlinedButton>(
            find.widgetWithText(OutlinedButton, 'Create user'),
          )
          .onPressed,
      isNull,
    );
  });
}
