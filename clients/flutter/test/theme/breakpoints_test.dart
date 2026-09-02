import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/theme/breakpoints.dart';

Future<bool> _compactAt(WidgetTester tester, Size size) async {
  late bool result;
  await tester.pumpWidget(
    MediaQuery(
      data: MediaQueryData(size: size),
      child: Builder(
        builder: (BuildContext context) {
          result = compactViewport(context);
          return const SizedBox.shrink();
        },
      ),
    ),
  );
  return result;
}

void main() {
  testWidgets('a phone is compact in either orientation', (
    WidgetTester tester,
  ) async {
    expect(await _compactAt(tester, const Size(360, 800)), isTrue);
    expect(
      await _compactAt(tester, const Size(800, 360)),
      isTrue,
      reason: 'landscape is wide but too short for a stage plus a page scroll',
    );
  });

  testWidgets('a desktop window is not compact', (WidgetTester tester) async {
    expect(await _compactAt(tester, const Size(1200, 900)), isFalse);
    expect(await _compactAt(tester, const Size(560, 560)), isFalse);
  });
}
