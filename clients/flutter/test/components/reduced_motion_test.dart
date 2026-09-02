import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/menu_field.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/catalog_card.dart';

Widget _host({required bool reduced, required Widget child}) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: MediaQuery(
    data: MediaQueryData(disableAnimations: reduced),
    child: Scaffold(body: child),
  ),
);

CatalogCard _card(int i) => CatalogCard(
  ref: TitleRef(type: TitleKind.movie, id: 'm$i'),
  route: '/movie/m$i',
  title: 'Movie $i',
);

void main() {
  testWidgets('the backdrop swaps instead of cross-fading', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(reduced: true, child: const BackdropWash(url: null)),
    );

    // The largest moving surface in the client.
    expect(find.byType(AnimatedSwitcher), findsNothing);

    await tester.pumpWidget(
      _host(reduced: false, child: const BackdropWash(url: null)),
    );

    expect(find.byType(AnimatedSwitcher), findsOneWidget);
  });

  testWidgets('the dropdown chevron flips without spinning', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        reduced: true,
        child: MenuField(open: true, onTap: () {}, child: const Text('Added')),
      ),
    );

    expect(
      tester.widget<AnimatedRotation>(find.byType(AnimatedRotation)).duration,
      Duration.zero,
    );
  });

  testWidgets('the rail jumps to the next page of cards', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(500, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      _host(
        reduced: true,
        child: CardRail(
          title: 'Recently added',
          cards: <CatalogCard>[for (int i = 0; i < 12; i++) _card(i)],
          imageBase: '',
        ),
      ),
    );
    await tester.pumpAndSettle();

    final ScrollableState list = tester.state<ScrollableState>(
      find.byType(Scrollable).first,
    );
    expect(list.position.pixels, 0);

    await tester.tap(find.byTooltip(Strings.next));
    await tester.pump();

    expect(
      list.position.pixels,
      greaterThan(0),
      reason: 'a single frame, so it jumped rather than glided',
    );
  });

  testWidgets('the skeleton shimmer stays still', (WidgetTester tester) async {
    await tester.pumpWidget(
      _host(reduced: true, child: const SkeletonLines(lines: 2)),
    );

    expect(
      find.descendant(
        of: find.byType(SkeletonLines),
        matching: find.byType(AnimatedBuilder),
      ),
      findsNothing,
    );
  });
}
