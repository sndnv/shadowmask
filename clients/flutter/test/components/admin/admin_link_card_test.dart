import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_link_card.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';

void _noop() {}

Widget _host(double width) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
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
}
