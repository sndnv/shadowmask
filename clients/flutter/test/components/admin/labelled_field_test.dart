import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

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
