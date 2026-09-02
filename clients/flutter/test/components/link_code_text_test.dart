import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/link_code_text.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(WidgetTester tester, String code) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(body: Center(child: LinkCodeText(code))),
    ),
  );
  await tester.pumpAndSettle();
}

Color? _colourOf(WidgetTester tester, String character) {
  final RichText rich = tester.widget<RichText>(find.byType(RichText));
  Color? found;
  rich.text.visitChildren((InlineSpan span) {
    if (span is TextSpan && span.text == character) {
      found = span.style?.color;
      return false;
    }
    return true;
  });
  return found;
}

void main() {
  test('the code is shown in groups of four', () {
    expect(groupedLinkCode('ABCD2345'), 'ABCD 2345');
    expect(groupedLinkCode('AB'), 'AB');
    expect(groupedLinkCode(''), '');
  });

  testWidgets('digits and letters are told apart by colour', (
    WidgetTester tester,
  ) async {
    await _pump(tester, 'AB2D3456');

    // Read off a screen onto a remote, so the two runs must not blur.
    expect(_colourOf(tester, '2'), Tokens.dark.accent);
    expect(_colourOf(tester, 'A'), Tokens.dark.text);
  });

  testWidgets('a screen reader hears the code one character at a time', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    await _pump(tester, 'AB2D3456');

    expect(tester.getSemantics(find.byType(RichText)).label, 'A B 2 D 3 4 5 6');

    semantics.dispose();
  });
}
