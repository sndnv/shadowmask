import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

void main() {
  Future<void> pumpPoster(WidgetTester tester, {VoidCallback? onTap}) =>
      tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(AppThemeVariant.dark),
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 220,
                height: 330,
                child: PosterPlay(
                  tooltip: 'Play',
                  onTap: onTap,
                  child: const ColoredBox(color: Color(0xFF000000)),
                ),
              ),
            ),
          ),
        ),
      );

  Future<void> hover(WidgetTester tester) async {
    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    await pointer.addPointer(location: Offset.zero);
    addTearDown(pointer.removePointer);
    await tester.pump();
    await pointer.moveTo(tester.getCenter(find.byType(PosterPlay)));
    await tester.pumpAndSettle();
  }

  testWidgets('the play affordance appears on hover and the poster plays', (
    WidgetTester tester,
  ) async {
    int taps = 0;
    await pumpPoster(tester, onTap: () => taps++);
    expect(find.byIcon(Icons.play_arrow), findsNothing);

    await hover(tester);
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);

    await tester.tap(find.byType(PosterPlay));
    await tester.pump();
    expect(taps, 1);
  });

  testWidgets('a poster with nothing to play stays inert', (
    WidgetTester tester,
  ) async {
    await pumpPoster(tester);

    await hover(tester);

    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.byType(InkWell), findsNothing);
  });
}
