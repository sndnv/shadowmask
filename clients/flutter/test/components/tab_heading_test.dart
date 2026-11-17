import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/link_heading.dart';
import 'package:shadowmask/components/section_heading.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';

Future<void> _pump(
  WidgetTester tester,
  AppThemeVariant variant,
  Widget child, {
  double scale = 1,
  MaterialTapTargetSize tapTarget = MaterialTapTargetSize.padded,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(variant).copyWith(materialTapTargetSize: tapTarget),
      home: MediaQuery(
        data: MediaQueryData(textScaler: TextScaler.linear(scale)),
        child: Scaffold(
          body: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[child],
          ),
        ),
      ),
    ),
  );
}

Finder _inside(Type parent, Type child) =>
    find.descendant(of: find.byType(parent), matching: find.byType(child));

void _expectSlantToTheFoot(WidgetTester tester) {
  final Finder clip = find.byWidgetPredicate(
    (Widget w) => w is ClipPath && w.clipper is TabSlantClipper,
  );
  final Size size = tester.getSize(clip);
  final Path path = const TabSlantClipper().getClip(size);
  expect(path.contains(Offset(size.width - 0.5, size.height - 0.5)), isTrue);
  expect(path.contains(Offset(size.width - 0.5, size.height / 2)), isFalse);
  expect(path.contains(Offset(size.width - kTabSlant - 1, 0.5)), isTrue);
  expect(path.contains(Offset(size.width - kTabSlant + 1, 0.5)), isFalse);
}

void main() {
  test('the slant runs from the top edge down to the bottom corner', () {
    final Path path = const TabSlantClipper().getClip(const Size(120, 26));

    expect(path.contains(const Offset(103, 1)), isTrue);
    expect(path.contains(const Offset(119, 2)), isFalse);
    expect(path.contains(const Offset(119, 14)), isFalse);
    expect(path.contains(const Offset(119, 25)), isTrue);
    expect(path.contains(const Offset(1, 25)), isTrue);
  });

  for (final AppThemeVariant variant in <AppThemeVariant>[
    AppThemeVariant.dark,
    AppThemeVariant.light,
    AppThemeVariant.retro,
  ]) {
    testWidgets('${variant.name} keeps the plain heading', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant, const TabHeading('Versions', count: 2));

      final Text text = tester.widget<Text>(_inside(TabHeading, Text));
      expect(text.data, Strings.countLabel('Versions', 2));
      expect(
        text.style,
        Theme.of(
          tester.element(find.byType(TabHeading)),
        ).textTheme.headlineMedium,
      );
      expect(_inside(TabHeading, DecoratedBox), findsNothing);
    });

    testWidgets('${variant.name} draws no rule under a header', (
      WidgetTester tester,
    ) async {
      await _pump(tester, variant, const TabRule(child: Text('row')));

      expect(_inside(TabRule, DecoratedBox), findsNothing);
    });
  }

  testWidgets('a plain heading passes its own style and lines through', (
    WidgetTester tester,
  ) async {
    const TextStyle style = TextStyle(fontSize: 15);
    await _pump(
      tester,
      AppThemeVariant.dark,
      const TabHeading(
        'Subtitles',
        style: style,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
    );

    final Text text = tester.widget<Text>(_inside(TabHeading, Text));
    expect(text.style, style);
    expect(text.maxLines, 1);
    expect(text.overflow, TextOverflow.ellipsis);
  });

  testWidgets('hologram draws an accent tab in capitals with a slanted end', (
    WidgetTester tester,
  ) async {
    await _pump(tester, AppThemeVariant.hologram, const TabHeading('Versions'));

    expect(find.text('VERSIONS'), findsOneWidget);
    expect(
      tester.widget<ColoredBox>(_inside(TabHeading, ColoredBox)).color,
      Tokens.hologram.accent,
    );
    final TextStyle style = tester.widget<Text>(find.text('VERSIONS')).style!;
    expect(style.color, Tokens.hologram.accentContrast);
    expect(style.fontSize, 11);
    expect(style.fontWeight, FontWeight.w700);
    expect(style.letterSpacing, closeTo(11 * 0.16, 0.001));
    _expectSlantToTheFoot(tester);
  });

  testWidgets('hologram keeps the heading\'s line limit', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const TabHeading(
        'Versions',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
    );

    final Text text = tester.widget<Text>(find.text('VERSIONS'));
    expect(text.maxLines, 1);
    expect(text.overflow, TextOverflow.ellipsis);
  });

  testWidgets('hologram reads the heading as written', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle handle = tester.ensureSemantics();
    await _pump(tester, AppThemeVariant.hologram, const TabHeading('Versions'));

    expect(find.bySemanticsLabel('Versions'), findsOneWidget);
    expect(find.bySemanticsLabel('VERSIONS'), findsNothing);
    handle.dispose();
  });

  testWidgets('at twice the text size the slant and the spacing keep up', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const TabHeading('Versions'),
      scale: 2,
    );

    _expectSlantToTheFoot(tester);
    expect(
      tester.widget<Text>(find.text('VERSIONS')).style!.letterSpacing,
      closeTo(22 * 0.16, 0.001),
    );
  });

  testWidgets('hologram puts the count beside the tab in accent monospace', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const TabHeading('Versions', count: 2),
    );

    final TextStyle style = tester.widget<Text>(find.text('2')).style!;
    expect(style.color, Tokens.hologram.accent);
    expect(style.fontSize, 12);
    expect(style.fontFamily, kMonoFamily);
    expect(find.textContaining('('), findsNothing);
  });

  testWidgets('a search heading carries its count and link inside the tab', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle handle = tester.ensureSemantics();
    await _pump(
      tester,
      AppThemeVariant.hologram,
      LinkHeading(
        title: Strings.seeAllAfter(Strings.countLabel('Movies', 40)),
        linkLabel: Strings.seeAll,
        route: '/movies',
      ),
    );

    final String whole =
        Strings.seeAllAfter(Strings.countLabel('Movies', 40)) + Strings.seeAll;
    expect(_inside(TabRule, SlantedTab), findsOneWidget);
    expect(find.text(whole.toUpperCase()), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp(RegExp.escape(whole))), findsWidgets);
    handle.dispose();
  });

  testWidgets('a long rail heading wraps in the tab on a phone', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const LinkHeading(
        title: Strings.moreInPrefix,
        linkLabel: 'The Lord of the Rings Collection of Extended Editions',
        route: '/collection',
      ),
      scale: 1.5,
    );

    expect(tester.takeException(), isNull);
    expect(
      tester.getSize(find.byType(SlantedTab)).width,
      lessThanOrEqualTo(360),
    );
    _expectSlantToTheFoot(tester);
  });

  testWidgets('a rail heading keeps its one line under the other themes', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.dark,
      const LinkHeading(
        title: 'Recently added',
        linkLabel: ' · See all',
        route: '/movies',
      ),
    );

    expect(find.byType(TabHeading), findsNothing);
    expect(find.text('Recently added · See all'), findsOneWidget);
  });

  testWidgets('a counted section heading puts the count beside the tab', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const SectionHeading(title: 'Episodes', count: 12),
    );

    expect(find.text('EPISODES'), findsOneWidget);
    expect(tester.widget<Text>(find.text('12')).style?.fontFamily, kMonoFamily);
  });

  testWidgets('a counted section heading reads as before elsewhere', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.dark,
      const SectionHeading(title: 'Episodes', count: 12),
    );

    expect(find.text(Strings.countLabel('Episodes', 12)), findsOneWidget);
  });

  testWidgets('a section heading keeps its trailing widget beside the tab', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const SectionHeading(title: 'Cast', trailing: Text('12')),
    );

    expect(_inside(TabRule, TabHeading), findsOneWidget);
    expect(find.text('CAST'), findsOneWidget);
    expect(find.text('12'), findsOneWidget);
  });

  for (final MaterialTapTargetSize tapTarget in MaterialTapTargetSize.values) {
    testWidgets('a hologram section heading lifts its trailing button off the '
        'rule (${tapTarget.name})', (WidgetTester tester) async {
      await _pump(
        tester,
        AppThemeVariant.hologram,
        SectionHeading(
          title: 'Seasons',
          trailing: IconButton(
            onPressed: () {},
            visualDensity: VisualDensity.compact,
            icon: const Icon(Icons.shuffle, size: 18),
          ),
        ),
        tapTarget: tapTarget,
      );
      final double rule = tester.getBottomLeft(find.byType(TabRule)).dy;
      final Finder button = find.descendant(
        of: find.byType(IconButton),
        matching: find.byType(Material),
      );

      expect(tester.getBottomLeft(button).dy, rule - Space.s1);
      expect(tester.getBottomLeft(find.byType(TabHeading)).dy, rule);
    });
  }

  testWidgets('a touch heading spends no more height than its tap area', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      SectionHeading(
        title: 'Seasons',
        trailing: IconButton(
          onPressed: () {},
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.shuffle, size: 18),
        ),
      ),
    );

    expect(
      tester.getSize(find.byType(TabRule)).height,
      tester.getSize(find.byType(IconButton)).height,
    );
  });

  testWidgets('the other themes centre a trailing widget on the title', (
    WidgetTester tester,
  ) async {
    const Key trailing = ValueKey<String>('trailing');
    await _pump(
      tester,
      AppThemeVariant.dark,
      const SectionHeading(
        title: 'Seasons',
        trailing: SizedBox(key: trailing, width: 80, height: 34),
      ),
      tapTarget: MaterialTapTargetSize.shrinkWrap,
    );

    expect(
      tester.getCenter(find.byKey(trailing)).dy,
      tester.getCenter(find.byType(TabHeading)).dy,
    );
  });

  testWidgets('hologram draws a line under the whole header row', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      AppThemeVariant.hologram,
      const TabRule(child: Text('row')),
    );

    final BoxDecoration rule =
        tester.widget<DecoratedBox>(_inside(TabRule, DecoratedBox)).decoration
            as BoxDecoration;
    expect(
      rule.border,
      Border(
        bottom: BorderSide(
          color: Tokens.hologram.accent.withValues(alpha: 0.62),
        ),
      ),
    );
    expect(
      tester.getSize(_inside(TabRule, DecoratedBox)).width,
      tester.view.physicalSize.width / tester.view.devicePixelRatio,
    );
  });
}
