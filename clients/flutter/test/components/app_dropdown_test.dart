import 'dart:ui' show Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/menu_option.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(body: Center(child: child)),
);

void _ignore(String _) {}

void main() {
  testWidgets('shows the current label in a bordered control', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        AppDropdown<String>(
          value: 'added_at',
          items: const <(String, String)>[
            ('added_at', 'Added'),
            ('title', 'Title'),
          ],
          onChanged: (_) {},
        ),
      ),
    );

    expect(find.byType(MenuField), findsOneWidget);
    expect(find.text('Added'), findsOneWidget);
    expect(tester.getSize(find.byType(MenuField)).height, kControlHeight);
  });

  testWidgets('opens a themed menu and reports the picked value', (
    WidgetTester tester,
  ) async {
    String? picked;
    await tester.pumpWidget(
      _host(
        AppDropdown<String>(
          value: 'added_at',
          items: const <(String, String)>[
            ('added_at', 'Added'),
            ('title', 'Title'),
          ],
          onChanged: (String v) => picked = v,
        ),
      ),
    );

    await tester.tap(find.text('Added'));
    await tester.pumpAndSettle();
    expect(find.byType(MenuOption), findsNWidgets(2));
    expect(find.byIcon(Icons.check), findsOneWidget);

    await tester.tap(find.text('Title').last);
    await tester.pumpAndSettle();
    expect(picked, 'title');
  });

  testWidgets('a control that fills its slot puts the chevron at the edge', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SizedBox(
          width: 320,
          child: AppDropdown<String>(
            value: 'added_at',
            items: <(String, String)>[
              ('added_at', 'Added'),
              ('title', 'Title'),
            ],
            onChanged: _ignore,
          ),
        ),
      ),
    );

    final Rect field = tester.getRect(find.byType(MenuField));
    final Rect chevron = tester.getRect(find.byIcon(Icons.expand_more));
    final Rect label = tester.getRect(find.text('Added'));
    expect(field.width, 320);
    expect(field.right - chevron.right, label.left - field.left);
    expect(chevron.left - label.right, Space.s2);
  });

  testWidgets('the menu is at least as wide as a control that fills', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const SizedBox(
          width: 320,
          child: AppDropdown<String>(
            value: 'added_at',
            items: <(String, String)>[
              ('added_at', 'Added'),
              ('title', 'Title'),
            ],
            onChanged: _ignore,
          ),
        ),
      ),
    );

    await tester.tap(find.text('Added'));
    await tester.pumpAndSettle();
    expect(
      tester.getSize(find.byType(MenuOption).first).width,
      greaterThanOrEqualTo(320 - 2),
    );
  });

  testWidgets('an intrinsic control lets the menu keep its own width', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const Wrap(
          children: <Widget>[
            AppDropdown<String>(
              value: 'a',
              items: <(String, String)>[
                ('a', 'A'),
                ('b', 'A rather long option label that needs the room'),
              ],
              onChanged: _ignore,
            ),
          ],
        ),
      ),
    );

    final double field = tester.getSize(find.byType(MenuField)).width;
    await tester.tap(find.text('A'));
    await tester.pumpAndSettle();
    expect(
      tester.getSize(find.byType(MenuOption).first).width,
      greaterThan(field),
    );
  });

  testWidgets('picking the current value does not fire a change', (
    WidgetTester tester,
  ) async {
    bool fired = false;
    await tester.pumpWidget(
      _host(
        AppDropdown<String>(
          value: 'added_at',
          items: const <(String, String)>[
            ('added_at', 'Added'),
            ('title', 'Title'),
          ],
          onChanged: (_) => fired = true,
        ),
      ),
    );

    await tester.tap(find.text('Added'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Added').last);
    await tester.pumpAndSettle();
    expect(fired, isFalse);
  });

  testWidgets('a label is announced and offered as a tooltip', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();

    await tester.pumpWidget(
      _host(
        AppDropdown<String>(
          value: 'added_at',
          label: 'Sort by',
          items: const <(String, String)>[
            ('added_at', 'Added'),
            ('title', 'Title'),
          ],
          onChanged: _ignore,
        ),
      ),
    );

    expect(find.byTooltip('Sort by'), findsOneWidget);

    final SemanticsNode node = tester.getSemantics(find.byType(MenuField));
    // The label and the current value are read as one control, not two.
    expect(node.label, contains('Sort by'));
    expect(node.label, contains('Added'));
    expect(
      node.flagsCollection.isExpanded,
      Tristate.isFalse,
      reason: 'a closed dropdown reports itself closed',
    );

    await tester.tap(find.text('Added'));
    await tester.pumpAndSettle();
    expect(
      tester.getSemantics(find.byType(MenuField)).flagsCollection.isExpanded,
      Tristate.isTrue,
    );
    semantics.dispose();
  });

  testWidgets('an unlabelled dropdown gains no tooltip', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        AppDropdown<String>(
          value: 'added_at',
          items: const <(String, String)>[('added_at', 'Added')],
          onChanged: _ignore,
        ),
      ),
    );

    expect(find.byType(Tooltip), findsNothing);
  });
}
