import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/action_field.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  testWidgets('the field is named without swallowing its action button', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    final TextEditingController controller = TextEditingController(text: 'up');
    addTearDown(controller.dispose);
    int submitted = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 320,
              child: ActionField(
                controller: controller,
                enabled: true,
                label: 'Search',
                hintText: 'Search titles',
                actionIcon: Icons.search,
                actionTooltip: 'Run the search',
                onSubmitted: () => submitted++,
              ),
            ),
          ),
        ),
      ),
    );

    final SemanticsNode field = tester.getSemantics(find.byType(TextField));
    expect(field.label, 'Search');
    expect(
      field.label,
      isNot(contains('Run the search')),
      reason: 'merging the suffix button in would rename the field after it',
    );

    await tester.tap(find.byTooltip('Run the search'));
    await tester.pump();
    expect(submitted, 1, reason: 'the button is still its own control');

    semantics.dispose();
  });
}
