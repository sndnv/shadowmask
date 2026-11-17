import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(
  WidgetTester tester,
  Size size, {
  AppThemeVariant variant = AppThemeVariant.dark,
  MaterialTapTargetSize tapTarget = MaterialTapTargetSize.padded,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: variant,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(variant).copyWith(materialTapTargetSize: tapTarget),
        home: Scaffold(
          body: SectionBlock(
            title: 'Versions',
            actions: <Widget>[
              OutlinedButton(onPressed: () {}, child: const Text('Rescan')),
              OutlinedButton(onPressed: () {}, child: const Text('Refresh')),
              OutlinedButton(onPressed: () {}, child: const Text('Delete')),
            ],
            child: const Text('body'),
          ),
        ),
      ),
    ),
  );
}

Future<void> _pumpItems(
  WidgetTester tester,
  Size size, {
  List<String> labels = const <String>['Purge', 'Clean', 'Edit'],
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: SectionBlock(
          title: 'Movies',
          actionItems: <PageAction>[
            for (final String label in labels)
              PageAction(
                icon: Icons.circle_outlined,
                label: label,
                onPressed: () {},
              ),
          ],
          child: const Text('body'),
        ),
      ),
    ),
  );
}

Finder _actions() =>
    find.ancestor(of: find.text('Rescan'), matching: find.byType(Wrap)).first;

void main() {
  testWidgets('hologram heads a block with a tab over a rule', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const Size(1200, 800),
      variant: AppThemeVariant.hologram,
    );

    expect(
      find.descendant(
        of: find.byType(TabRule),
        matching: find.byType(TabHeading),
      ),
      findsOneWidget,
    );
    expect(find.text('VERSIONS'), findsOneWidget);
    expect(find.text('Rescan'), findsOneWidget);
    expect(
      tester.getBottomLeft(find.byType(TabHeading)).dy,
      tester.getBottomLeft(find.byType(TabRule)).dy,
    );
  });

  for (final MaterialTapTargetSize tapTarget in MaterialTapTargetSize.values) {
    testWidgets('hologram lifts the buttons off the rule (${tapTarget.name})', (
      WidgetTester tester,
    ) async {
      await _pump(
        tester,
        const Size(1200, 800),
        variant: AppThemeVariant.hologram,
        tapTarget: tapTarget,
      );
      final Finder button = find
          .descendant(
            of: find.widgetWithText(OutlinedButton, 'Rescan'),
            matching: find.byType(Material),
          )
          .first;

      expect(
        tester.getBottomLeft(button).dy,
        tester.getBottomLeft(find.byType(TabRule)).dy - Space.s1,
      );
    });
  }

  testWidgets('a touch header spends no more height than its tap area', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const Size(1200, 800),
      variant: AppThemeVariant.hologram,
    );

    expect(
      tester.getSize(find.byType(TabRule)).height,
      tester.getSize(_actions()).height,
    );
  });

  testWidgets('the other themes centre the actions on the title', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const Size(1200, 800),
      tapTarget: MaterialTapTargetSize.shrinkWrap,
    );

    expect(
      tester.getCenter(_actions()).dy,
      tester.getCenter(find.byType(TabHeading)).dy,
    );
  });

  testWidgets('hologram keeps the tab on its rule in a taller header', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      ThemeScope(
        variant: AppThemeVariant.hologram,
        setVariant: (_) {},
        child: MaterialApp(
          theme: buildTheme(AppThemeVariant.hologram),
          home: const Scaffold(
            body: SectionHeaderHeight(
              minHeight: 48,
              child: SectionBlock(title: 'Profile', child: Text('body')),
            ),
          ),
        ),
      ),
    );

    expect(tester.getSize(find.byType(TabRule)).height, 48);
    expect(
      tester.getBottomLeft(find.byType(TabHeading)).dy,
      tester.getBottomLeft(find.byType(TabRule)).dy,
    );
  });

  testWidgets('a wide block keeps the title and actions on one line', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(1200, 800));

    expect(
      tester.getRect(find.text('Delete')).top,
      closeTo(tester.getRect(find.text('Versions')).top, 12),
    );
  });

  testWidgets('the actions are pinned to the right of their section', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(1200, 800));

    final Rect block = tester.getRect(find.byType(SectionBlock));
    final Rect first = tester.getRect(
      find.widgetWithText(OutlinedButton, 'Rescan'),
    );
    final Rect last = tester.getRect(
      find.widgetWithText(OutlinedButton, 'Delete'),
    );
    final Rect title = tester.getRect(find.text('Versions'));

    expect(
      last.right,
      moreOrLessEquals(block.right, epsilon: 1),
      reason:
          'a Wrap that shrink-wraps has no free space to spread, which '
          'left the actions sitting against the title',
    );
    expect(first.left, greaterThan(title.right + 100));
  });

  testWidgets('a phone drops the actions below the title', (
    WidgetTester tester,
  ) async {
    await _pump(tester, const Size(360, 800));

    expect(tester.takeException(), isNull);
    expect(
      tester.getRect(find.text('Rescan')).top,
      greaterThan(tester.getRect(find.text('Versions')).bottom),
      reason: 'three buttons beside a heading need more than 360px',
    );
  });

  testWidgets('action items turn to icons on a phone, either way up', (
    WidgetTester tester,
  ) async {
    for (final (Size size, bool icons) in <(Size, bool)>[
      (const Size(360, 800), true),
      (const Size(800, 360), true),
      (const Size(1200, 800), false),
    ]) {
      await _pumpItems(tester, size);

      expect(tester.takeException(), isNull);
      for (final String label in <String>['Purge', 'Clean', 'Edit']) {
        expect(
          find.text(label),
          icons ? findsNothing : findsOneWidget,
          reason: '$label at $size',
        );
        expect(
          find.byTooltip(label),
          icons ? findsOneWidget : findsNothing,
          reason: '$label at $size',
        );
      }
    }
  });

  testWidgets('six action items show as three buttons on one row', (
    WidgetTester tester,
  ) async {
    const List<String> labels = <String>[
      'Refresh',
      'Edit',
      'Cancel jobs',
      'Clean',
      'Purge',
      'Delete',
    ];
    for (final Size size in <Size>[
      const Size(360, 800),
      const Size(1200, 800),
    ]) {
      await _pumpItems(tester, size, labels: labels);

      expect(tester.takeException(), isNull);
      final Finder buttons = find.descendant(
        of: find.byType(SectionBlock),
        matching: find.byType(OutlinedButton),
      );
      expect(buttons, findsNWidgets(3), reason: 'at $size');
      final Set<double> tops = <double>{
        for (int i = 0; i < 3; i++) tester.getTopLeft(buttons.at(i)).dy,
      };
      expect(tops, hasLength(1), reason: 'at $size');
      expect(find.byTooltip(Strings.moreActions), findsOneWidget);
    }
  });

  testWidgets('a title too long for the row is ellipsised, not overflowed', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: SectionBlock(
            title: '0f8b2c1e-7a44-4d19-9c3f-2b6e8a05d731',
            child: Text('body'),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final Text title = tester.widget<Text>(
      find.text('0f8b2c1e-7a44-4d19-9c3f-2b6e8a05d731'),
    );
    // A job id is a section title, and it must not paint past its box.
    expect(title.maxLines, 1);
    expect(title.overflow, TextOverflow.ellipsis);
    expect(
      tester.getSize(find.byType(Text).first).width,
      lessThanOrEqualTo(360),
    );
  });
}
