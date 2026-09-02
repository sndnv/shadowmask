import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(WidgetTester tester, double width) async {
  tester.view.physicalSize = Size(width, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(
          body: DetailSplit(
            posterWidth: 320,
            poster: SizedBox(key: Key('poster'), height: 180),
            info: Text('info'),
          ),
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('a wide split keeps the poster at its asked width', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 1200);

    expect(
      tester.getSize(find.byKey(const Key('poster'))).width,
      closeTo(320, 2),
      reason: 'the frame spends a pixel of border on each side',
    );
  });

  testWidgets('a phone shrinks the poster to the width it actually has', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 300);

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byKey(const Key('poster'))).width,
      lessThanOrEqualTo(300),
      reason: 'collapsing to one column does not pay for a 320px poster',
    );
  });
}
