import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

Future<double> _widthAt(WidgetTester tester, double viewport) async {
  final TextEditingController controller = TextEditingController();
  addTearDown(controller.dispose);
  tester.view.physicalSize = Size(viewport, 600);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: AdminFilterField(
            controller: controller,
            hintText: 'Filter',
            onChanged: (_) {},
          ),
        ),
      ),
    ),
  );
  return tester.getSize(find.byType(AdminFilterField)).width;
}

void main() {
  testWidgets('a roomy page gives the filter its full width', (
    WidgetTester tester,
  ) async {
    expect(await _widthAt(tester, 1200), 320);
  });

  testWidgets('a phone narrows the filter instead of overflowing', (
    WidgetTester tester,
  ) async {
    expect(await _widthAt(tester, 300), lessThanOrEqualTo(300));
    expect(tester.takeException(), isNull);
  });

  testWidgets('the filter keeps its name once the hint is typed over', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    final TextEditingController controller = TextEditingController(
      text: 'failed',
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: AdminFilterField(
            controller: controller,
            hintText: Strings.filterJobs,
            onChanged: (_) {},
          ),
        ),
      ),
    );

    // The hint is the only thing naming this box, and it disappears the
    // moment anything is typed.
    expect(
      tester.getSemantics(find.byType(TextField)).label,
      Strings.fieldFilter,
    );

    semantics.dispose();
  });
}
