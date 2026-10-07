import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/components/paged_card_grid.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/catalog_card.dart';

void _noop() {}

Widget _host(
  double width,
  List<CatalogCard> cards,
  int remaining, {
  VoidCallback onLoad = _noop,
  bool loading = true,
  bool failed = false,
}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: SingleChildScrollView(
      child: SizedBox(
        width: width,
        child: PagedCardGrid(
          cards: cards,
          imageBase: 'http://host',
          remaining: remaining,
          loading: loading,
          failed: failed,
          onLoad: onLoad,
        ),
      ),
    ),
  ),
);

List<CatalogCard> _cards(int count) => <CatalogCard>[
  for (int i = 0; i < count; i++)
    CatalogCard(
      ref: TitleRef(type: TitleKind.movie, id: 'm$i'),
      route: '/title?type=movie&id=m$i',
      title: 'Title $i',
    ),
];

void main() {
  testWidgets('a placeholder is the same width as a loaded card', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(1), 9));

    final double card = tester
        .getSize(find.byType(CatalogCardTile).first)
        .width;
    final double placeholder = tester
        .getSize(find.byType(SkeletonCard).first)
        .width;

    // A placeholder left at the unfitted 160 would break the row it shares
    // with a stretched card, pushing itself onto a line of its own.
    expect(placeholder, card);
  });

  testWidgets('one loaded card and its placeholder share a row', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(1), 9));

    final Offset card = tester.getTopLeft(find.byType(CatalogCardTile).first);
    final Offset placeholder = tester.getTopLeft(
      find.byType(SkeletonCard).first,
    );

    expect(placeholder.dy, card.dy);
    expect(placeholder.dx, greaterThan(card.dx));
  });

  testWidgets('the placeholder follows the card width as space changes', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(600, _cards(2), 9));
    final double wide = tester.getSize(find.byType(SkeletonCard).first).width;
    expect(wide, tester.getSize(find.byType(CatalogCardTile).first).width);

    await tester.pumpWidget(_host(304, _cards(2), 9));
    final double narrow = tester.getSize(find.byType(SkeletonCard).first).width;
    expect(narrow, tester.getSize(find.byType(CatalogCardTile).first).width);

    expect(narrow, lessThan(wide));
  });

  testWidgets('a page too short to scroll still asks for the next one', (
    WidgetTester tester,
  ) async {
    int loads = 0;
    await tester.pumpWidget(
      _host(304, _cards(1), 9, onLoad: () => loads++, loading: false),
    );
    await tester.pump();

    // One card cannot fill the viewport, so there is no scroll to trigger on
    // and the request has to come from the post-frame check instead.
    expect(loads, greaterThan(0));
  });

  testWidgets('an idle grid with pages left shows no placeholders', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(2), 9, loading: false));
    await tester.pumpAndSettle();

    expect(find.byType(SkeletonCard), findsNothing);
    expect(tester.binding.hasScheduledFrame, isFalse);
  });

  testWidgets('a page in flight shows its placeholders under one pulse', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(2), 9));
    await tester.pump();

    expect(find.byType(SkeletonCard), findsNWidgets(kPlaceholderCards));
    expect(find.byType(SkeletonPulse), findsOneWidget);
    expect(tester.binding.hasScheduledFrame, isTrue);

    await tester.pumpWidget(_host(304, _cards(2), 2));
    await tester.pump();

    expect(find.byType(SkeletonCard), findsNWidgets(2));
  });

  testWidgets('an arrived page takes the placeholders and the pulse away', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(2), 9));
    await tester.pump();

    await tester.pumpWidget(_host(304, _cards(4), 7, loading: false));
    await tester.pumpAndSettle();

    expect(find.byType(SkeletonCard), findsNothing);
    expect(find.byType(CatalogCardTile), findsNWidgets(4));
    expect(tester.binding.hasScheduledFrame, isFalse);
  });

  testWidgets('a failed page offers a retry instead of placeholders', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(600, _cards(2), 9, failed: true));
    await tester.pumpAndSettle();

    expect(find.byType(SkeletonCard), findsNothing);
    expect(
      tester.getCenter(find.text(Strings.retry)).dy,
      moreOrLessEquals(
        tester.getCenter(find.text(Strings.couldNotLoadMore)).dy,
      ),
    );
  });

  testWidgets('the retry folds under its message on a phone', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(304, _cards(2), 9, failed: true));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(
      tester.getTopLeft(find.text(Strings.retry)).dy,
      greaterThan(tester.getBottomLeft(find.text(Strings.couldNotLoadMore)).dy),
    );
  });
}
