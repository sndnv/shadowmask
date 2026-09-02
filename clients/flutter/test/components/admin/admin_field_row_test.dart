import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_field_row.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Widget _host(double width, List<AdminField> fields) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SizedBox(
      width: width,
      child: AdminFieldRow(fields: fields),
    ),
  ),
);

List<AdminField> _searchRow() => <AdminField>[
  const AdminField(flex: 1, SizedBox(key: Key('query'), height: 40)),
  const AdminField(width: 160, SizedBox(key: Key('language'), height: 40)),
  const AdminField(SizedBox(key: Key('button'), width: 110, height: 40)),
];

void main() {
  testWidgets('a roomy row keeps its controls side by side', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(600, _searchRow()));

    final Rect query = tester.getRect(find.byKey(const Key('query')));
    final Rect language = tester.getRect(find.byKey(const Key('language')));
    final Rect button = tester.getRect(find.byKey(const Key('button')));

    expect(language.top, query.top);
    expect(button.left, greaterThan(language.right - 1));
    expect(language.width, 160);
  });

  testWidgets('a cramped row gives each control its own line', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(280, _searchRow()));

    expect(tester.takeException(), isNull);
    final Rect query = tester.getRect(find.byKey(const Key('query')));
    final Rect language = tester.getRect(find.byKey(const Key('language')));
    final Rect button = tester.getRect(find.byKey(const Key('button')));

    expect(language.top, greaterThan(query.bottom - 1));
    expect(button.top, greaterThan(language.bottom - 1));
    expect(
      language.width,
      280,
      reason: 'a stacked control takes the row rather than its fixed width',
    );
  });

  testWidgets('the threshold follows what the fields actually need', (
    WidgetTester tester,
  ) async {
    // Two flexible fields need 180 each plus a 12 gap, so 380 is the line.
    final List<AdminField> pair = <AdminField>[
      const AdminField(flex: 1, SizedBox(key: Key('query'), height: 40)),
      const AdminField(flex: 1, SizedBox(key: Key('language'), height: 40)),
    ];

    await tester.pumpWidget(_host(400, pair));
    expect(
      tester.getRect(find.byKey(const Key('language'))).top,
      tester.getRect(find.byKey(const Key('query'))).top,
    );

    await tester.pumpWidget(_host(360, pair));
    expect(
      tester.getRect(find.byKey(const Key('language'))).top,
      greaterThan(tester.getRect(find.byKey(const Key('query'))).bottom - 1),
    );
  });
}
