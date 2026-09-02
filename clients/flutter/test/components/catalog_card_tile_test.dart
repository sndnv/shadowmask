import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_grid.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/components/watched_marker.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

void main() {
  _tooltipTests();

  testWidgets('a tile is the same height with and without a subtitle', (
    WidgetTester tester,
  ) async {
    final List<CatalogCard> cards = <CatalogCard>[
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
        route: '/title?type=movie&id=m1',
        title: 'Alpha',
        subtitle: '2020',
      ),
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm2'),
        route: '/title?type=movie&id=m2',
        title: 'A rather long title that wraps onto a second line',
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

    final Iterable<Size> sizes = tester
        .widgetList<CatalogCardTile>(find.byType(CatalogCardTile))
        .map(
          (CatalogCardTile tile) =>
              tester.getSize(find.byWidget(tile, skipOffstage: false)),
        );
    expect(sizes.length, 2);
    expect(sizes.first.height, sizes.last.height);
    expect(
      sizes.first.width,
      fittedCardWidth(800, CardAspect.poster),
      reason: 'the grid stretches its cards to fill the row it was given',
    );
  });

  testWidgets('hovering lights the border and does not move the card', (
    WidgetTester tester,
  ) async {
    await _pumpTile(tester, _card(watched: false));

    final Size resting = tester.getSize(find.byType(CatalogCardTile));
    final double width = _cardBorder(tester).width;
    expect(_cardBorder(tester).color, Tokens.dark.border);

    final TestGesture pointer = await tester.createGesture(
      kind: PointerDeviceKind.mouse,
    );
    addTearDown(pointer.removePointer);
    await pointer.addPointer(location: Offset.zero);
    await pointer.moveTo(tester.getCenter(find.byType(CatalogCardTile)));
    await tester.pumpAndSettle();

    expect(_cardBorder(tester).color, Tokens.dark.accent);
    expect(
      _cardBorder(tester).width,
      width,
      reason: 'only the colour changes; 2px reads too heavy on a 160px card',
    );
    expect(tester.getSize(find.byType(CatalogCardTile)), resting);
  });

  testWidgets('a dismissable card hides the watched marker', (
    WidgetTester tester,
  ) async {
    await _pumpTile(tester, _card(watched: true));
    expect(find.byType(WatchedMarker), findsOneWidget);

    await _pumpTile(
      tester,
      _card(watched: true),
      onDismiss: (CatalogCard _) {},
    );

    expect(
      find.byType(WatchedMarker),
      findsNothing,
      reason: 'both sat at top right; and in history the marker says nothing',
    );
  });

  testWidgets('a badge renders on the artwork', (WidgetTester tester) async {
    await _pumpTile(tester, _card(watched: false), badge: '3x');

    expect(find.text('3x'), findsOneWidget);
  });
}

CatalogCard _card({required bool watched}) => CatalogCard(
  ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
  route: '/title?type=movie&id=m1',
  title: 'Alpha',
  watched: watched,
);

BorderSide _cardBorder(WidgetTester tester) {
  final Material card = tester.widget<Material>(
    find
        .descendant(
          of: find.byType(CatalogCardTile),
          matching: find.byType(Material),
        )
        .first,
  );
  return (card.shape! as RoundedRectangleBorder).side;
}

void _tooltipTests() {
  testWidgets('the tooltip carries the full untruncated text', (
    WidgetTester tester,
  ) async {
    await _pumpTile(
      tester,
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
        route: '/title?type=movie&id=m1',
        title: 'A rather long title that the card has to cut short',
        subtitle: '2016',
      ),
    );

    const String tip =
        'A rather long title that the card has to cut short\n2016';
    expect(find.byTooltip(tip), findsWidgets);
    expect(
      find.descendant(of: find.byTooltip(tip), matching: find.byType(CardArt)),
      findsOneWidget,
      reason: 'hovering the poster must show it, not just the caption',
    );
  });

  testWidgets('a card in progress names the resume point in its tooltip', (
    WidgetTester tester,
  ) async {
    await _pumpTile(
      tester,
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
        route: '/title?type=movie&id=m1',
        title: 'Arrival',
        subtitle: '2016',
        progressPercent: 40,
      ),
    );

    expect(find.byTooltip('Arrival\n2016\nResume at 40%'), findsWidgets);
  });

  testWidgets('the dismiss button keeps its own tooltip, not the card text', (
    WidgetTester tester,
  ) async {
    await _pumpTile(
      tester,
      CatalogCard(
        ref: const TitleRef(type: TitleKind.movie, id: 'm1'),
        route: '/title?type=movie&id=m1',
        title: 'Arrival',
        subtitle: '2016',
      ),
      onDismiss: (CatalogCard _) {},
    );

    expect(find.byTooltip(Strings.dismiss), findsOneWidget);
    expect(find.byTooltip('Arrival\n2016'), findsNWidgets(2));
  });
}

Future<void> _pumpTile(
  WidgetTester tester,
  CatalogCard card, {
  void Function(CatalogCard card)? onDismiss,
  String? badge,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Center(
          child: CatalogCardTile(
            card: card,
            imageBase: 'http://host',
            width: kPosterCardWidth,
            onDismiss: onDismiss,
            badge: badge,
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}
