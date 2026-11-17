import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_link_card.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

void _noop() {}

Widget _host(double width, {AppThemeVariant variant = AppThemeVariant.dark}) =>
    MaterialApp(
      theme: buildTheme(variant),
      home: Scaffold(
        body: SizedBox(
          width: width,
          child: const Wrap(
            spacing: Space.s4,
            runSpacing: Space.s4,
            children: <Widget>[
              AdminLinkCard(
                title: 'Libraries',
                description: 'Add and scan libraries.',
                icon: Icons.folder_outlined,
                onTap: _noop,
              ),
              AdminLinkCard(
                title: 'Users',
                description: 'Manage accounts.',
                icon: Icons.person_outline,
                onTap: _noop,
              ),
            ],
          ),
        ),
      ),
    );

void main() {
  testWidgets('cards keep their fixed width on a wide screen', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(760));

    expect(
      tester.getSize(find.byType(AdminLinkCard).first).width,
      kAdminLinkCardWidth,
    );
  });

  testWidgets('a card takes the whole row on a phone', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304));

    // At 260 of a 304px row the card looked like it had stopped short.
    expect(tester.getSize(find.byType(AdminLinkCard).first).width, 304);
    final Rect first = tester.getRect(find.byType(AdminLinkCard).first);
    final Rect second = tester.getRect(find.byType(AdminLinkCard).last);
    expect(second.top, greaterThan(first.bottom - 1));
  });

  testWidgets('hologram keeps the icon beside the name in a card as tall as '
      'the others', (WidgetTester tester) async {
    await tester.pumpWidget(_host(760));
    final double rounded = tester
        .getSize(find.byType(AdminLinkCard).first)
        .height;

    await tester.pumpWidget(_host(760, variant: AppThemeVariant.hologram));
    await tester.pumpAndSettle();

    final Rect icon = tester.getRect(find.byIcon(Icons.folder_outlined));
    final Rect title = tester.getRect(find.text('Libraries'));
    expect(icon.right, lessThan(title.left));
    expect(icon.top, lessThan(title.bottom));
    expect(tester.getSize(find.byType(AdminLinkCard).first).height, rounded);
  });

  testWidgets('a hovered hologram card draws its edge on the line', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(760, variant: AppThemeVariant.hologram));
    const Tokens t = Tokens.hologram;
    expect(_edge(tester), t.accent.withValues(alpha: 0.26));

    await _hover(tester);

    expect(_edge(tester), t.accent.withValues(alpha: 0.62));
  });

  testWidgets('a hologram card reached by the keyboard draws its edge too', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(760, variant: AppThemeVariant.hologram));

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();

    expect(_edge(tester), Tokens.hologram.accent.withValues(alpha: 0.62));
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} keeps the icon beside the name', (
      WidgetTester tester,
    ) async {
      await tester.pumpWidget(_host(760, variant: variant));

      final Rect icon = tester.getRect(find.byIcon(Icons.folder_outlined));
      final Rect title = tester.getRect(find.text('Libraries'));
      expect(icon.right, lessThan(title.left));
      expect(icon.top, lessThan(title.bottom));

      await _hover(tester);

      expect(_edge(tester), Tokens.of(variant).border);
    });
  }
}

Future<void> _hover(WidgetTester tester) async {
  final TestGesture pointer = await tester.createGesture(
    kind: PointerDeviceKind.mouse,
  );
  addTearDown(pointer.removePointer);
  await pointer.addPointer(location: Offset.zero);
  await pointer.moveTo(tester.getCenter(find.byType(AdminLinkCard).first));
  await tester.pumpAndSettle();
}

Color _edge(WidgetTester tester) {
  final Decoration? decoration = tester
      .widget<Container>(
        find
            .descendant(
              of: find.byType(AdminLinkCard).first,
              matching: find.byType(Container),
            )
            .first,
      )
      .decoration;
  return switch (decoration) {
    ShapeDecoration(:final OutlinedBorder shape) => shape.side.color,
    BoxDecoration(:final Border border) => border.top.color,
    _ => throw StateError('no edge'),
  };
}
