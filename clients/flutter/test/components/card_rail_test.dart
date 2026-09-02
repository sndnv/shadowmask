import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/card_rail.dart';
import 'package:shadowmask/components/hover_tap.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/catalog_card.dart';

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
        body: SizedBox(
          width: width,
          child: CardRail(
            title: Strings.moreInPrefix,
            titleLink: 'Villeneuve',
            titleRoute: '/collection/c1',
            cards: <CatalogCard>[for (int i = 0; i < cards; i++) _card(i)],
            imageBase: '',
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
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
}
