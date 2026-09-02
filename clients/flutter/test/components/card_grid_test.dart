import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/model/common/title_ref.dart';

void main() {
  _fittedWidthTests();

  testWidgets('CardGrid renders every card and marks watched ones', (
    WidgetTester tester,
  ) async {
    final List<CatalogCard> cards = <CatalogCard>[
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
        route: '/title?type=movie&id=m1',
        title: 'Alpha',
        subtitle: '2020',
        watched: true,
      ),
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm2'),
        route: '/title?type=movie&id=m2',
        title: 'Beta',
      ),
    ];

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: CardGrid(cards: cards, imageBase: 'http://host'),
        ),
      ),
    );

    expect(find.text('Alpha'), findsOneWidget);
    expect(find.text('Beta'), findsOneWidget);
    // Exactly one watched marker (a check disc) is rendered.
    expect(find.byIcon(Icons.check), findsOneWidget);
  });
}

const double _kPhoneBody = 304;

void _fittedWidthTests() {
  test('the fitted width is a no-op at the design content width', () {
    // 6 posters at 160 with 5 gaps of 16 is exactly 1040, so the rule must
    // leave the width the design was drawn for untouched.
    expect(fittedCardWidth(1040, CardAspect.poster), kPosterCardWidth);
  });

  test('a phone body fits two posters with no trailing gap', () {
    final double w = fittedCardWidth(_kPhoneBody, CardAspect.poster);
    expect(w * 2 + Space.s4, _kPhoneBody);
  });

  test('a phone body gives an episode card the whole row', () {
    expect(fittedCardWidth(_kPhoneBody, CardAspect.landscape), _kPhoneBody);
  });

  test('an explicit target is fitted the same way as the default', () {
    // Cast cards ask for 116, which packs eight into the design width.
    expect(
      fittedCardWidth(1040, CardAspect.person, target: kCastCardWidth),
      kCastCardWidth,
    );
    expect(
      fittedCardWidth(_kPhoneBody, CardAspect.person, target: kCastCardWidth),
      greaterThan(kCastCardWidth),
    );
  });

  test('an unbounded width falls back to the preferred size', () {
    expect(
      fittedCardWidth(double.infinity, CardAspect.poster),
      kPosterCardWidth,
    );
    expect(fittedCardWidth(0, CardAspect.poster), kPosterCardWidth);
  });

  testWidgets('a phone-width grid draws two cards per row', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(360, 720);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    final List<CatalogCard> cards = <CatalogCard>[
      for (int i = 0; i < 4; i++)
        CatalogCard(
          ref: TitleRef(type: TitleKind.movie, id: 'm$i'),
          route: '/title?type=movie&id=m$i',
          title: 'Title $i',
        ),
    ];

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: Padding(
            padding: const EdgeInsets.all(28),
            child: CardGrid(cards: cards, imageBase: 'http://host'),
          ),
        ),
      ),
    );

    expect(tester.takeException(), isNull);
    final List<Offset> tops = tester
        .widgetList<CatalogCardTile>(find.byType(CatalogCardTile))
        .map((CatalogCardTile t) => tester.getTopLeft(find.byWidget(t)))
        .toList();
    expect(tops.length, 4);
    // Two rows of two: the third card starts back at the left edge.
    expect(tops[1].dy, tops.first.dy);
    expect(tops[2].dx, tops.first.dx);
    expect(tops[2].dy, greaterThan(tops.first.dy));

    final Size card = tester.getSize(find.byType(CatalogCardTile).first);
    expect(card.width * 2 + Space.s4, _kPhoneBody);
  });
}
