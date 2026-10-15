import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: Center(child: SizedBox(width: 320, child: child)),
  ),
);

void main() {
  testWidgets('a text field answers to its own visible label', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    final TextEditingController controller = TextEditingController(
      text: 'Blade Runner',
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      _host(LabelledTextField(controller: controller, label: 'Title')),
    );

    // The visible label sits above the box, so without this the field
    // announces itself as an unnamed edit box.
    expect(
      tester.getSemantics(find.byType(TextField)),
      matchesSemantics(
        label: 'Title',
        value: 'Blade Runner',
        isTextField: true,
        hasEnabledState: true,
        isEnabled: true,
        isFocusable: true,
        hasTapAction: true,
        hasFocusAction: true,
        currentValueLength: 12,
      ),
    );

    semantics.dispose();
  });

  final ValueVariant<double> fitting = ValueVariant<double>(<double>{
    0.85,
    1.0,
    1.15,
    1.3,
  });

  testWidgets('a one-line field keeps the control height at any font size', (
    WidgetTester tester,
  ) async {
    tester.platformDispatcher.textScaleFactorTestValue = fitting.currentValue!;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    final TextEditingController controller = TextEditingController(
      text: 'Blade Runner',
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      _host(LabelledTextField(controller: controller, label: 'Title')),
    );

    expect(tester.getSize(find.byType(TextField)).height, kControlHeight);
  }, variant: fitting);

  testWidgets('a one-line field grows around text taller than the control', (
    WidgetTester tester,
  ) async {
    tester.platformDispatcher.textScaleFactorTestValue = 1.5;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    final TextEditingController controller = TextEditingController(
      text: 'Blade Runner',
    );
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      _host(LabelledTextField(controller: controller, label: 'Title')),
    );

    final Rect field = tester.getRect(find.byType(TextField));
    final Rect text = tester.getRect(find.byType(EditableText));
    expect(field.height, greaterThan(kControlHeight));
    expect(text.top, greaterThanOrEqualTo(field.top));
    expect(text.bottom, lessThanOrEqualTo(field.bottom));
  });

  testWidgets('a multi-line field keeps its padding at any font size', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController(
      text: 'A long overview',
    );
    addTearDown(controller.dispose);
    Future<double> inset(double scale) async {
      tester.platformDispatcher.textScaleFactorTestValue = scale;
      await tester.pumpWidget(
        _host(
          LabelledTextField(
            controller: controller,
            label: 'Overview',
            maxLines: 3,
          ),
        ),
      );
      return tester.getRect(find.byType(EditableText)).top -
          tester.getRect(find.byType(TextField)).top;
    }

    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
    expect(await inset(1.3), await inset(1.0));
  });

  testWidgets('a dropdown reads as its label and its current choice', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();

    await tester.pumpWidget(
      _host(
        LabelledDropdown<String>(
          label: 'Library kind',
          value: 'movie',
          items: const <(String, String)>[
            ('movie', 'Movies'),
            ('series', 'Series'),
          ],
          onChanged: (_) {},
        ),
      ),
    );

    expect(
      tester.getSemantics(find.text('Movies')).label,
      'Library kind\nMovies',
    );

    semantics.dispose();
  });
}
