import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/toggle_button.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/tokens.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: child),
);

void main() {
  testWidgets('shows the label and the outline icon when unpressed', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.bookmark_border,
          filledIcon: Icons.bookmark,
          label: 'Add to watchlist',
          pressed: false,
          onToggle: () {},
        ),
      ),
    );

    expect(find.text('Add to watchlist'), findsOneWidget);
    expect(find.byIcon(Icons.bookmark_border), findsOneWidget);
    expect(find.byIcon(Icons.bookmark), findsNothing);
  });

  testWidgets('uses the filled icon when pressed', (WidgetTester tester) async {
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.bookmark_border,
          filledIcon: Icons.bookmark,
          label: 'Remove from watchlist',
          pressed: true,
          onToggle: () {},
        ),
      ),
    );

    expect(find.byIcon(Icons.bookmark), findsOneWidget);
    expect(find.byIcon(Icons.bookmark_border), findsNothing);
  });

  testWidgets('a labelled toggle does not tooltip its own label', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.check,
          filledIcon: Icons.check_circle,
          label: 'Mark watched',
          pressed: false,
          onToggle: () {},
        ),
      ),
    );

    expect(find.byType(Tooltip), findsNothing);
  });

  testWidgets('a compact toggle falls back to its label as the tooltip', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.check,
          filledIcon: Icons.check_circle,
          label: 'Mark watched',
          pressed: false,
          compact: true,
          onToggle: () {},
        ),
      ),
    );

    expect(
      tester.widget<Tooltip>(find.byType(Tooltip)).message,
      'Mark watched',
    );
  });

  testWidgets('an explicit tooltip survives in either mode', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.check,
          filledIcon: Icons.check_circle,
          label: 'Watched',
          tooltip: 'Mark unwatched',
          pressed: true,
          onToggle: () {},
        ),
      ),
    );

    expect(
      tester.widget<Tooltip>(find.byType(Tooltip)).message,
      'Mark unwatched',
    );
  });

  testWidgets('a busy toggle does not fire its callback', (
    WidgetTester tester,
  ) async {
    int taps = 0;
    await tester.pumpWidget(
      _host(
        ToggleButton(
          icon: Icons.check,
          filledIcon: Icons.check_circle,
          label: 'Mark watched',
          pressed: false,
          busy: true,
          onToggle: () => taps++,
        ),
      ),
    );

    await tester.tap(find.byType(ToggleButton));
    await tester.pump();
    expect(taps, 0);
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    final Tokens t = Tokens.of(variant);
    final Shapes shapes = Shapes.of(variant, t);

    Future<Color> edge(
      WidgetTester tester, {
      required bool pressed,
      required bool highlight,
    }) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(variant),
          home: Scaffold(
            body: ToggleButton(
              icon: Icons.check,
              filledIcon: Icons.check_circle,
              label: 'Mark watched',
              pressed: pressed,
              highlight: highlight,
              onToggle: () {},
            ),
          ),
        ),
      );
      final Material material = tester.widget<Material>(
        find
            .descendant(
              of: find.byType(ToggleButton),
              matching: find.byType(Material),
            )
            .first,
      );
      return (material.shape! as OutlinedBorder).side.color;
    }

    testWidgets('a ${variant.name} toggle that only acts takes the button '
        'edge', (WidgetTester tester) async {
      expect(
        await edge(tester, pressed: false, highlight: false),
        shapes.line(t.border),
      );
      expect(
        await edge(tester, pressed: true, highlight: false),
        shapes.line(t.border),
      );
    });

    testWidgets('a ${variant.name} toggle with a state keeps the soft edge '
        'until pressed', (WidgetTester tester) async {
      expect(
        await edge(tester, pressed: false, highlight: true),
        shapes.lineSoft(t.border),
      );
      expect(
        await edge(tester, pressed: true, highlight: true),
        shapes.line(t.accent),
      );
    });
  }
}
