import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shared_preferences/shared_preferences.dart';

CatalogCard _card(int i) => CatalogCard(
  ref: TitleRef(type: TitleKind.movie, id: 'm$i'),
  route: '/movie/m$i',
  title: 'Movie $i',
);

Future<void> _pump(
  WidgetTester tester, {
  required List<String> visited,
  double width = 600,
  int cards = 8,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      onGenerateRoute: (RouteSettings settings) {
        visited.add(settings.name ?? '');
        return MaterialPageRoute<void>(
          settings: settings,
          builder: (BuildContext _) => const SizedBox.shrink(),
        );
      },
      home: Scaffold(
        body: CardMenuHost(
          catalog: CatalogApi(
            ApiClient(
              baseUrl: 'http://test',
              httpClient: MockClient(
                (http.Request _) async => http.Response('[]', 200),
              ),
            ),
          ),
          userId: 'u1',
          child: SingleChildScrollView(
            key: _pageKey,
            child: SizedBox(
              width: width,
              child: Column(
                children: <Widget>[
                  CardRail(
                    title: Strings.moreInPrefix,
                    titleLink: 'Villeneuve',
                    titleRoute: '/collection/c1',
                    cards: <CatalogCard>[
                      for (int i = 0; i < cards; i++) _card(i),
                    ],
                    imageBase: '',
                  ),
                  const SizedBox(height: 2000),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

const Key _pageKey = ValueKey<String>('page');

ScrollableState _rail(WidgetTester tester) => tester.state<ScrollableState>(
  find.descendant(of: find.byType(CardRail), matching: find.byType(Scrollable)),
);

ScrollableState _page(WidgetTester tester) => tester.state<ScrollableState>(
  find
      .descendant(of: find.byKey(_pageKey), matching: find.byType(Scrollable))
      .first,
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('a long press inside a scrolling rail still opens the menu', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[]);
    await tester.longPress(find.byType(CardArt).first);
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.markWatched),
      findsOneWidget,
      reason: 'the horizontal drag must not swallow a press that never moved',
    );
  }, variant: TargetPlatformVariant.only(TargetPlatform.android));

  testWidgets('the heading link opens from the keyboard', (
    WidgetTester tester,
  ) async {
    final List<String> visited = <String>[];
    await _pump(tester, visited: visited);

    expect(
      find.text(Strings.moreInCollection('Villeneuve')),
      findsOneWidget,
      reason: 'the prefix and the name stay one line of text',
    );

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(visited, contains('/collection/c1'));
  });

  testWidgets('focus underlines the link the way hover does', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: Scaffold(
          body: SizedBox(
            width: 600,
            child: CardRail(
              title: Strings.moreInPrefix,
              titleLink: 'Villeneuve',
              titleRoute: '/collection/c1',
              cards: <CatalogCard>[_card(0)],
              imageBase: '',
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    TextStyle? linkStyle() {
      final RichText rich = tester.widget<RichText>(
        find.descendant(
          of: find.byType(HoverTap),
          matching: find.byType(RichText),
        ),
      );
      TextStyle? found;
      rich.text.visitChildren((InlineSpan span) {
        if (span is TextSpan && span.text == 'Villeneuve') {
          found = span.style;
          return false;
        }
        return true;
      });
      return found;
    }

    expect(linkStyle()?.decoration, isNot(TextDecoration.underline));

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expect(linkStyle()?.decoration, TextDecoration.underline);
  });

  testWidgets('the scroll arrows say which way they go', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[]);

    expect(find.byTooltip(Strings.next), findsOneWidget);
    expect(find.byTooltip(Strings.previous), findsNothing);
  });

  testWidgets('an ordinary up and down wheel moves the rail sideways', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[], cards: 40);
    final ScrollableState rail = _rail(tester);
    expect(rail.position.pixels, 0);

    final TestPointer mouse = TestPointer(1, PointerDeviceKind.mouse);
    mouse.hover(tester.getCenter(find.byType(ListView)));
    await tester.sendEventToBinding(mouse.scroll(const Offset(0, 120)));
    await tester.pumpAndSettle();
    expect(rail.position.pixels, 120);

    await tester.sendEventToBinding(mouse.scroll(const Offset(0, -40)));
    await tester.pumpAndSettle();
    expect(rail.position.pixels, 80);
  });

  testWidgets('a rail at its end keeps the wheel rather than passing it on', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[], cards: 40);
    final ScrollableState rail = _rail(tester);
    rail.position.jumpTo(rail.position.maxScrollExtent);
    await tester.pumpAndSettle();
    final ScrollMetrics page = _page(tester).position.copyWith();

    final TestPointer mouse = TestPointer(1, PointerDeviceKind.mouse);
    mouse.hover(tester.getCenter(find.byType(ListView)));
    await tester.sendEventToBinding(mouse.scroll(const Offset(0, 120)));
    await tester.pumpAndSettle();

    expect(rail.position.pixels, rail.position.maxScrollExtent);
    expect(_page(tester).position.pixels, page.pixels);
  });

  testWidgets('a rail whose cards all fit leaves the wheel to the page', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[], cards: 2);
    final ScrollableState rail = _rail(tester);
    expect(rail.position.maxScrollExtent, 0);

    final TestPointer mouse = TestPointer(1, PointerDeviceKind.mouse);
    mouse.hover(tester.getCenter(find.byType(ListView)));
    await tester.sendEventToBinding(mouse.scroll(const Offset(0, 120)));
    await tester.pumpAndSettle();

    expect(_page(tester).position.pixels, greaterThan(0));
  });

  testWidgets('a trackpad pan sideways still reaches the rail', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[], cards: 40);
    final ScrollableState rail = _rail(tester);

    final TestPointer mouse = TestPointer(1, PointerDeviceKind.mouse);
    mouse.hover(tester.getCenter(find.byType(ListView)));
    await tester.sendEventToBinding(mouse.scroll(const Offset(90, 0)));
    await tester.pumpAndSettle();

    expect(rail.position.pixels, 90);
  });

  testWidgets('a rail can be dragged with a mouse', (
    WidgetTester tester,
  ) async {
    await _pump(tester, visited: <String>[], cards: 40);
    final ScrollableState rail = _rail(tester);

    await tester.drag(
      find.byType(ListView),
      const Offset(-150, 0),
      kind: PointerDeviceKind.mouse,
    );
    await tester.pumpAndSettle();

    expect(rail.position.pixels, greaterThan(0));
  });

  testWidgets('dragging by mouse does not swallow a click on a card', (
    WidgetTester tester,
  ) async {
    final List<String> visited = <String>[];
    await _pump(tester, visited: visited, cards: 40);

    await tester.tap(find.text('Movie 1'));
    await tester.pumpAndSettle();

    expect(visited, contains('/movie/m1'));
  });
}
