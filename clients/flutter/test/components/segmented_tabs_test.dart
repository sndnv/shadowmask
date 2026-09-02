import 'dart:ui' show SemanticsRole, Tristate;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

class _Host extends StatefulWidget {
  const _Host({required this.picked});

  final List<String> picked;

  @override
  State<_Host> createState() => _HostState();
}

class _HostState extends State<_Host> {
  String _current = 'a';

  @override
  Widget build(BuildContext context) => SegmentedTabs<String>(
    current: _current,
    tabs: const <(String, String)>[
      ('a', 'Active'),
      ('b', 'Failed'),
      ('c', 'All'),
    ],
    onChanged: (String v) {
      widget.picked.add(v);
      setState(() => _current = v);
    },
  );
}

Future<List<String>> _pump(WidgetTester tester) async {
  final List<String> picked = <String>[];
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(body: _Host(picked: picked)),
    ),
  );
  await tester.pumpAndSettle();
  return picked;
}

Future<void> _key(WidgetTester tester, LogicalKeyboardKey key) async {
  await tester.sendKeyEvent(key);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('only the selected tab is a tab stop', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    // Three tabs used to mean three stops on the way past them.
    final Iterable<InkWell> wells = tester.widgetList<InkWell>(
      find.byType(InkWell),
    );
    expect(wells.where((InkWell w) => w.canRequestFocus).length, 1);
    expect(
      tester
          .widget<InkWell>(
            find.ancestor(
              of: find.text('Active'),
              matching: find.byType(InkWell),
            ),
          )
          .canRequestFocus,
      isTrue,
    );
  });

  testWidgets('the arrow keys move between tabs', (WidgetTester tester) async {
    final List<String> picked = await _pump(tester);

    await _key(tester, LogicalKeyboardKey.tab);
    await _key(tester, LogicalKeyboardKey.arrowRight);

    expect(picked, <String>['b']);

    await _key(tester, LogicalKeyboardKey.arrowRight);
    expect(picked, <String>['b', 'c']);

    // Wrapping keeps the arrows useful at either end.
    await _key(tester, LogicalKeyboardKey.arrowRight);
    expect(picked, <String>['b', 'c', 'a']);

    await _key(tester, LogicalKeyboardKey.arrowLeft);
    expect(picked, <String>['b', 'c', 'a', 'c']);
  });

  testWidgets('focus follows the selection so the arrows keep working', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _key(tester, LogicalKeyboardKey.tab);
    await _key(tester, LogicalKeyboardKey.arrowRight);
    await _key(tester, LogicalKeyboardKey.arrowRight);

    expect(
      FocusManager.instance.primaryFocus?.debugLabel,
      'tab',
      reason: 'the newly selected tab has to take the focus with it',
    );
  });

  testWidgets('a screen reader is told which tab is selected', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester);

    expect(tester.getSemantics(find.text('Active')).role, SemanticsRole.tab);
    expect(
      tester.getSemantics(find.text('Active')).flagsCollection.isSelected,
      Tristate.isTrue,
    );
    expect(
      tester.getSemantics(find.text('Failed')).flagsCollection.isSelected,
      Tristate.isFalse,
    );

    semantics.dispose();
  });
}
