import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Future<void> _pump(WidgetTester tester, Widget child) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(body: SizedBox(width: 400, child: child)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('a dense text field is exactly as tall as a dropdown', (
    WidgetTester tester,
  ) async {
    final TextEditingController controller = TextEditingController();
    addTearDown(controller.dispose);
    await _pump(
      tester,
      Column(
        children: <Widget>[
          LabelledTextField(controller: controller, label: 'Name'),
          AppDropdown<int>(
            value: 1,
            items: const <(int, String)>[(1, 'One')],
            onChanged: (_) {},
          ),
        ],
      ),
    );

    final double field = tester.getSize(find.byType(TextField)).height;
    final double dropdown = tester.getSize(find.byType(MenuField)).height;

    expect(dropdown, kControlHeight);
    expect(field, kControlHeight);
  });

  testWidgets('a heading with help is no taller than one without', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      const Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Expanded(
            child: FieldLabel(
              label: 'Plain',
              child: Placeholder(key: ValueKey<String>('plain')),
            ),
          ),
          Expanded(
            child: FieldLabel(
              label: 'Helped',
              help: 'Some help',
              child: Placeholder(key: ValueKey<String>('helped')),
            ),
          ),
        ],
      ),
    );

    expect(find.byType(FieldLabel), findsNWidgets(2));
    expect(
      tester.getTopLeft(find.byKey(const ValueKey<String>('plain'))).dy,
      tester.getTopLeft(find.byKey(const ValueKey<String>('helped'))).dy,
    );
  });

  testWidgets('only a multi line field is allowed to grow', (
    WidgetTester tester,
  ) async {
    final TextEditingController one = TextEditingController();
    final TextEditingController many = TextEditingController();
    addTearDown(one.dispose);
    addTearDown(many.dispose);
    await _pump(
      tester,
      Column(
        children: <Widget>[
          LabelledTextField(controller: one, label: 'One'),
          LabelledTextField(controller: many, label: 'Many', maxLines: 3),
        ],
      ),
    );

    expect(tester.getSize(find.byType(TextField).at(0)).height, kControlHeight);
    expect(
      tester.getSize(find.byType(TextField).at(1)).height,
      greaterThan(kControlHeight),
    );
  });

  testWidgets('a field button matches the control it sits beside', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: <Widget>[
          Expanded(
            child: AppDropdown<int>(
              value: 1,
              items: const <(int, String)>[(1, 'One')],
              onChanged: (_) {},
            ),
          ),
          FilledButton(
            onPressed: () {},
            style: kFieldButtonStyle,
            child: const Text('Search'),
          ),
        ],
      ),
    );

    expect(
      tester.getSize(find.byType(FilledButton)).height,
      tester.getSize(find.byType(MenuField)).height,
    );
    expect(tester.getSize(find.byType(FilledButton)).height, kControlHeight);
  });

  testWidgets('an ordinary button is left alone and stays taller', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      Row(
        children: <Widget>[
          FilledButton(onPressed: () {}, child: const Text('Save')),
        ],
      ),
    );

    expect(
      tester.getSize(find.byType(FilledButton)).height,
      greaterThan(kControlHeight),
    );
  });
}
