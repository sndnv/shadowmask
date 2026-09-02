import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/view/card_aspect.dart';

Future<void> _pump(
  WidgetTester tester, {
  int? percent,
  VoidCallback? onTap,
}) async {
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 200,
              child: PosterPlay(
                tooltip: 'Play',
                onTap: onTap,
                progressPercent: percent,
                child: const CardArt(
                  artwork: null,
                  aspect: CardAspect.poster,
                  imageBase: '',
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('no progress means no bar', (WidgetTester tester) async {
    await _pump(tester, onTap: () {});
    expect(find.byType(ProgressBar), findsNothing);
  });

  testWidgets('the bar sits inside the artwork, not below it', (
    WidgetTester tester,
  ) async {
    await _pump(tester, percent: 40, onTap: () {});

    final Rect art = tester.getRect(find.byType(CardArt));
    final Rect bar = tester.getRect(find.byType(ProgressBar));

    expect(
      bar.bottom,
      moreOrLessEquals(art.bottom, epsilon: 0.01),
      reason: 'the bar is flush with the poster, not floating under it',
    );
    expect(bar.left, moreOrLessEquals(art.left, epsilon: 0.01));
    expect(bar.right, moreOrLessEquals(art.right, epsilon: 0.01));
    expect(bar.height, kProgressBarHeight);
  });

  testWidgets('the bar is clipped by the poster corners, not its own', (
    WidgetTester tester,
  ) async {
    await _pump(tester, percent: 40, onTap: () {});

    final Finder clips = find.ancestor(
      of: find.byType(ProgressBar),
      matching: find.byType(ClipRRect),
    );
    expect(clips, findsWidgets);

    for (int i = 0; i < clips.evaluate().length; i++) {
      final Finder clip = clips.at(i);
      final BorderRadius radius =
          tester.widget<ClipRRect>(clip).borderRadius as BorderRadius;
      final Size size = tester.getSize(clip);
      expect(
        radius.bottomLeft.y,
        lessThanOrEqualTo(size.height / 2),
        reason:
            'a corner radius taller than half the box it clips is what warped '
            'the bar ends (radius ${radius.bottomLeft.y} on a ${size.height}px '
            'box)',
      );
    }
  });

  testWidgets('a poster with no tap still shows its progress', (
    WidgetTester tester,
  ) async {
    await _pump(tester, percent: 70);
    expect(find.byType(ProgressBar), findsOneWidget);
    expect(tester.widget<ProgressBar>(find.byType(ProgressBar)).percent, 70);
  });
}
