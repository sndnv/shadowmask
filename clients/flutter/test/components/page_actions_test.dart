import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/l10n/strings.dart';
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

  testWidgets('a phone shows the actions as icons on one row', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    for (final Size phone in <Size>[
      const Size(360, 800),
      const Size(800, 360),
    ]) {
      tester.view.physicalSize = phone;
      await tester.pumpWidget(
        _host(
          PageActions(<PageAction>[
            PageAction(icon: Icons.refresh, label: 'Refresh', onPressed: () {}),
            PageAction(
              icon: Icons.cleaning_services_outlined,
              label: 'Clean all',
              danger: true,
              onPressed: () {},
            ),
            PageAction(
              icon: Icons.add,
              label: 'Create library',
              primary: true,
              onPressed: () {},
            ),
          ]),
        ),
      );

      expect(tester.takeException(), isNull);
      expect(find.byTooltip(Strings.moreActions), findsNothing);
      for (final String label in <String>[
        'Refresh',
        'Clean all',
        'Create library',
      ]) {
        expect(find.text(label), findsNothing, reason: '$label at $phone');
        expect(find.byTooltip(label), findsOneWidget, reason: '$phone');
      }
      expect(
        tester.getRect(find.byTooltip('Create library')).top,
        tester.getRect(find.byTooltip('Refresh')).top,
        reason: 'labels wrapped the last button onto a second line at $phone',
      );
      expect(
        find.descendant(
          of: find.byTooltip('Create library'),
          matching: find.byType(FilledButton),
        ),
        findsOneWidget,
      );
    }
  });

  testWidgets('more than three actions show two and a menu of the rest', (
    WidgetTester tester,
  ) async {
    final List<String> ran = <String>[];
    PageAction action(String label, {bool danger = false}) => PageAction(
      icon: Icons.circle_outlined,
      label: label,
      danger: danger,
      onPressed: () => ran.add(label),
    );
    await tester.pumpWidget(
      _host(
        PageActions(<PageAction>[
          action('Refresh'),
          action('Edit'),
          action('Cancel jobs'),
          action('Purge', danger: true),
        ]),
      ),
    );

    expect(find.widgetWithText(OutlinedButton, 'Refresh'), findsOneWidget);
    expect(find.widgetWithText(OutlinedButton, 'Edit'), findsOneWidget);
    expect(find.byTooltip(Strings.moreActions), findsOneWidget);
    expect(find.text('Cancel jobs'), findsNothing);
    expect(find.text('Purge'), findsNothing);

    await tester.tap(find.byTooltip(Strings.moreActions));
    await tester.pumpAndSettle();

    expect(
      tester.getRect(find.widgetWithText(MenuItemButton, 'Cancel jobs')).top,
      lessThan(
        tester.getRect(find.widgetWithText(MenuItemButton, 'Purge')).top,
      ),
    );
    final ButtonStyle? purge = tester
        .widget<MenuItemButton>(find.widgetWithText(MenuItemButton, 'Purge'))
        .style;
    expect(
      purge?.foregroundColor?.resolve(<WidgetState>{}),
      Theme.of(tester.element(find.text('Purge'))).colorScheme.error,
    );
    expect(
      tester
          .widget<MenuItemButton>(
            find.widgetWithText(MenuItemButton, 'Cancel jobs'),
          )
          .style,
      isNull,
    );

    await tester.tap(find.widgetWithText(MenuItemButton, 'Purge'));
    await tester.pumpAndSettle();

    expect(ran, <String>['Purge']);
    expect(find.widgetWithText(MenuItemButton, 'Purge'), findsNothing);
  });

  testWidgets('an icon with a reason still names its action', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      _host(
        PageActions(const <PageAction>[
          PageAction(
            icon: Icons.refresh,
            label: 'Refresh metadata',
            tooltip: 'Nothing to refresh from here.',
            onPressed: null,
          ),
        ]),
      ),
    );

    expect(
      find.byTooltip('Refresh metadata: Nothing to refresh from here.'),
      findsOneWidget,
      reason: 'the reason alone left a bare icon nobody could put a name to',
    );
  });

  testWidgets('an action without a callback is disabled in the menu', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        PageActions(<PageAction>[
          PageAction(icon: Icons.refresh, label: 'Refresh', onPressed: () {}),
          PageAction(icon: Icons.edit, label: 'Edit', onPressed: () {}),
          PageAction(icon: Icons.cancel, label: 'Cancel', onPressed: () {}),
          const PageAction(
            icon: Icons.delete,
            label: 'Delete',
            danger: true,
            onPressed: null,
          ),
        ]),
      ),
    );

    await tester.tap(find.byTooltip(Strings.moreActions));
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<MenuItemButton>(find.widgetWithText(MenuItemButton, 'Delete'))
          .onPressed,
      isNull,
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
