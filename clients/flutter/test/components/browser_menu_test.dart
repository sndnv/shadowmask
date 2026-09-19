import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/browser_menu.dart';
import 'package:shadowmask/components/card_menu.dart';
import 'package:shadowmask/components/catalog_card_tile.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/catalog_card.dart';

CatalogCard _card({TitleKind kind = TitleKind.movie}) => CatalogCard(
  ref: TitleRef(type: kind, id: 'm1'),
  route: '/title?type=${kind.wire}&id=m1',
  title: 'Alpha',
);

CatalogApi _catalog() => CatalogApi(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request _) async => http.Response('{}', 200)),
  ),
);

Future<void> _pumpTile(
  WidgetTester tester, {
  required bool menued,
  TitleKind kind = TitleKind.movie,
}) async {
  final Widget tile = Center(
    child: CatalogCardTile(
      card: _card(kind: kind),
      imageBase: 'http://host',
      width: 160,
    ),
  );
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: menued
            ? CardMenuHost(catalog: _catalog(), userId: 'u1', child: tile)
            : tile,
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<TestGesture> _hover(WidgetTester tester) async {
  final TestGesture pointer = await tester.createGesture(
    kind: PointerDeviceKind.mouse,
  );
  addTearDown(pointer.removePointer);
  await pointer.addPointer(location: Offset.zero);
  await pointer.moveTo(tester.getCenter(find.byType(CatalogCardTile)));
  await tester.pumpAndSettle();
  return pointer;
}

void main() {
  testWidgets('hovering a card that has our menu takes the browser menu away', (
    WidgetTester tester,
  ) async {
    expect(BrowserMenu.holds, 0);
    await _pumpTile(tester, menued: true);

    final TestGesture pointer = await _hover(tester);
    expect(BrowserMenu.holds, 1);

    await pointer.moveTo(const Offset(2000, 2000));
    await tester.pumpAndSettle();
    expect(
      BrowserMenu.holds,
      0,
      reason: 'off the card the browser is free to show its own menu again',
    );
  });

  testWidgets('a card with no menu of its own leaves the browser alone', (
    WidgetTester tester,
  ) async {
    expect(BrowserMenu.holds, 0);
    await _pumpTile(tester, menued: false);

    await _hover(tester);

    expect(
      BrowserMenu.holds,
      0,
      reason: 'nothing of ours opens here, so the browser keeps its menu',
    );
  });

  testWidgets('a person card keeps the browser menu even inside the host', (
    WidgetTester tester,
  ) async {
    expect(BrowserMenu.holds, 0);
    await _pumpTile(tester, menued: true, kind: TitleKind.person);

    await _hover(tester);

    expect(
      BrowserMenu.holds,
      0,
      reason:
          'a person supports no action, so we open no menu to make room for',
    );
  });

  testWidgets('a card disposed while hovered does not leak the suppression', (
    WidgetTester tester,
  ) async {
    expect(BrowserMenu.holds, 0);
    await _pumpTile(tester, menued: true);
    await _hover(tester);
    expect(BrowserMenu.holds, 1);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();

    expect(BrowserMenu.holds, 0);
  });

  test('restoring more often than suppressing cannot go negative', () {
    BrowserMenu.restore();
    expect(BrowserMenu.holds, 0);

    BrowserMenu.suppress();
    BrowserMenu.suppress();
    expect(BrowserMenu.holds, 2);
    BrowserMenu.restore();
    BrowserMenu.restore();
    BrowserMenu.restore();
    expect(BrowserMenu.holds, 0);
  });
}
