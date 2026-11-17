import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/corner_brackets.dart';
import 'package:shadowmask/components/detail_split.dart';
import 'package:shadowmask/components/poster_play.dart';
import 'package:shadowmask/components/progress_bar.dart';
import 'package:shadowmask/components/resume_ring.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

void main() {
  // These cover the hover affordance, which is a pointer behaviour. Tests run
  // as Android by default, where the poster now shows a resting play badge
  // instead, because touch has no hover to reveal one.
  Future<void> pumpPoster(
    WidgetTester tester, {
    VoidCallback? onTap,
    AppThemeVariant variant = AppThemeVariant.dark,
    int? progress,
    int? resume,
    double width = 220,
  }) => tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(variant).copyWith(platform: TargetPlatform.macOS),
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: width,
            height: width * 1.5,
            child: PosterPlay(
              label: 'Play',
              onTap: onTap,
              progressPercent: progress,
              resumePercent: resume,
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

  testWidgets('on touch the poster is artwork rather than a button', (
    WidgetTester tester,
  ) async {
    // There is no hover to reveal an affordance, so a tappable poster is a
    // secret. The visible controls below it do the same job.
    int taps = 0;
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(
          AppThemeVariant.dark,
        ).copyWith(platform: TargetPlatform.android),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 220,
              height: 330,
              child: PosterPlay(
                label: 'Play',
                onTap: () => taps++,
                child: const ColoredBox(color: Color(0xFF000000)),
              ),
            ),
          ),
        ),
      ),
    );

    expect(find.byIcon(Icons.play_arrow), findsNothing);
    expect(find.byType(InkWell), findsNothing);

    await tester.tap(find.byType(PosterPlay), warnIfMissed: false);
    await tester.pump();

    expect(taps, 0);
  });

  Color litEdge(WidgetTester tester) {
    final DecoratedBox lit = tester.widget<DecoratedBox>(
      find
          .descendant(
            of: find.byType(PosterPlay),
            matching: find.byType(DecoratedBox),
          )
          .first,
    );
    final Decoration decoration = lit.decoration;
    return switch (decoration) {
      ShapeDecoration(:final OutlinedBorder shape) => shape.side.color,
      BoxDecoration(:final Border border) => border.top.color,
      _ => throw StateError('no lit edge'),
    };
  }

  testWidgets('hologram brackets a lit poster and draws its edge on the line', (
    WidgetTester tester,
  ) async {
    await pumpPoster(tester, onTap: () {}, variant: AppThemeVariant.hologram);
    expect(find.byType(CornerBrackets), findsNothing);

    await hover(tester);

    expect(
      tester.getRect(find.byType(CornerBrackets)),
      tester.getRect(find.byType(PosterPlay)).inflate(kBracketReach),
    );
    expect(litEdge(tester), Tokens.hologram.accent.withValues(alpha: 0.62));
    expect(
      tester
          .widget<Stack>(
            find
                .ancestor(
                  of: find.byType(CornerBrackets),
                  matching: find.byType(Stack),
                )
                .first,
          )
          .clipBehavior,
      Clip.none,
    );
  });

  Future<void> pumpFramed(WidgetTester tester, AppThemeVariant variant) =>
      tester.pumpWidget(
        MaterialApp(
          theme: buildTheme(variant).copyWith(platform: TargetPlatform.macOS),
          home: Scaffold(
            body: Padding(
              padding: const EdgeInsets.all(40),
              child: DetailSplit(
                poster: PosterPlay(
                  label: 'Play',
                  onTap: () {},
                  child: const SizedBox(
                    height: 330,
                    child: ColoredBox(color: Color(0xFF000000)),
                  ),
                ),
                info: const SizedBox.shrink(),
              ),
            ),
          ),
        ),
      );

  Iterable<Element> clipsAbove(Finder target) {
    bool clips(Widget w) =>
        w is ClipRect ||
        w is ClipRRect ||
        w is ClipPath ||
        w is ClipOval ||
        (w is Container && w.clipBehavior != Clip.none);
    final Set<Element> inSplit = find
        .descendant(
          of: find.byType(DetailSplit),
          matching: find.byWidgetPredicate(clips),
        )
        .evaluate()
        .toSet();
    return find
        .ancestor(of: target, matching: find.byWidgetPredicate(clips))
        .evaluate()
        .where(inSplit.contains);
  }

  testWidgets('a hologram poster in its page frame keeps its brackets', (
    WidgetTester tester,
  ) async {
    await pumpFramed(tester, AppThemeVariant.hologram);

    await hover(tester);

    expect(find.byType(CornerBrackets), findsOneWidget);
    expect(clipsAbove(find.byType(CornerBrackets)), isEmpty);
  });

  testWidgets('the other themes still clip the poster to its frame', (
    WidgetTester tester,
  ) async {
    await pumpFramed(tester, AppThemeVariant.dark);

    expect(clipsAbove(find.byType(PosterPlay)), isNotEmpty);
  });

  testWidgets('a poster reached by the keyboard keeps focus and plays', (
    WidgetTester tester,
  ) async {
    int taps = 0;
    await pumpPoster(tester, onTap: () => taps++);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
    expect(
      FocusManager.instance.primaryFocus?.context
          ?.findAncestorWidgetOfExactType<PosterPlay>(),
      isNotNull,
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pump();

    expect(taps, 1);
  });

  for (final AppThemeVariant variant in AppThemeVariant.values.where(
    (AppThemeVariant v) => v != AppThemeVariant.hologram,
  )) {
    testWidgets('${variant.name} lights a poster without brackets', (
      WidgetTester tester,
    ) async {
      await pumpPoster(tester, onTap: () {}, variant: variant);

      await hover(tester);

      expect(find.byType(CornerBrackets), findsNothing);
      expect(litEdge(tester), Tokens.of(variant).accent);
    });
  }

  testWidgets('a hologram poster shows the resume ring on hover', (
    WidgetTester tester,
  ) async {
    await pumpPoster(
      tester,
      onTap: () {},
      variant: AppThemeVariant.hologram,
      progress: 40,
    );
    expect(find.byType(ResumeRing), findsNothing);

    await hover(tester);

    expect(tester.widget<ResumeRing>(find.byType(ResumeRing)).percent, 40);
    expect(find.text(Strings.resumeAction), findsOneWidget);
    expect(find.text('40%'), findsOneWidget);
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
  });

  testWidgets('a hologram poster without progress shows the play ring', (
    WidgetTester tester,
  ) async {
    await pumpPoster(tester, onTap: () {}, variant: AppThemeVariant.hologram);

    await hover(tester);

    expect(tester.widget<ResumeRing>(find.byType(ResumeRing)).percent, isNull);
    expect(find.text(Strings.play), findsOneWidget);
  });

  testWidgets('a resume point fills the ring without a progress bar', (
    WidgetTester tester,
  ) async {
    await pumpPoster(
      tester,
      onTap: () {},
      variant: AppThemeVariant.hologram,
      resume: 40,
    );

    await hover(tester);

    expect(tester.widget<ResumeRing>(find.byType(ResumeRing)).percent, 40);
    expect(find.byType(ProgressBar), findsNothing);
  });

  testWidgets('keyboard focus shows the ring too', (WidgetTester tester) async {
    await pumpPoster(
      tester,
      onTap: () {},
      variant: AppThemeVariant.hologram,
      progress: 40,
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();

    expect(find.byType(ResumeRing), findsOneWidget);
  });

  testWidgets('the ring stays a picture on a poster that plays', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    int taps = 0;
    await pumpPoster(
      tester,
      onTap: () => taps++,
      variant: AppThemeVariant.hologram,
      progress: 40,
    );
    await hover(tester);

    await tester.tapAt(tester.getCenter(find.byType(ResumeRing)));

    expect(taps, 1);
    expect(find.bySemanticsLabel(Strings.resumeAction), findsNothing);
    semantics.dispose();
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('a ${variant.name} poster shows no tooltip and keeps its '
        'label', (WidgetTester tester) async {
      final SemanticsHandle semantics = tester.ensureSemantics();
      await pumpPoster(tester, onTap: () {}, variant: variant, progress: 40);
      await hover(tester);
      await tester.pump(const Duration(seconds: 1));

      expect(
        find.descendant(
          of: find.byType(PosterPlay),
          matching: find.byType(Tooltip),
        ),
        findsNothing,
      );
      expect(
        tester.getSemantics(find.byType(PosterPlay)),
        isSemantics(label: 'Play', isButton: true),
      );
      semantics.dispose();
    });
  }

  testWidgets('a narrow poster shrinks the ring to fit', (
    WidgetTester tester,
  ) async {
    await pumpPoster(
      tester,
      onTap: () {},
      variant: AppThemeVariant.hologram,
      progress: 40,
      width: 100,
    );
    await hover(tester);

    expect(
      tester.getRect(find.byType(ResumeRing)).width,
      lessThanOrEqualTo(100 - 2 * Space.s3),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('the other themes keep the round play on hover', (
    WidgetTester tester,
  ) async {
    await pumpPoster(tester, onTap: () {}, progress: 40);

    await hover(tester);

    expect(find.byType(ResumeRing), findsNothing);
    expect(find.byIcon(Icons.play_arrow), findsOneWidget);
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.hologram,
  ]) {
    testWidgets('a ${variant.name} poster keeps its progress '
        '${variant == AppThemeVariant.hologram ? 'off' : 'on'} the edges', (
      WidgetTester tester,
    ) async {
      await pumpPoster(tester, onTap: () {}, variant: variant, progress: 40);
      final double inset = variant == AppThemeVariant.hologram ? Space.s1 : 0;
      final Rect poster = tester.getRect(find.byType(PosterPlay));
      final Rect bar = tester.getRect(find.byType(ProgressBar));

      expect(bar.left, poster.left + inset);
      expect(bar.right, poster.right - inset);
      expect(bar.bottom, poster.bottom - inset);
    });
  }
}
