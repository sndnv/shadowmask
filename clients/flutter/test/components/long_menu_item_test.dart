import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/long_menu_item.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

const String _kLong =
    'English · srt · OpenSubtitles · '
    'Paper.Skies.2019.REMASTERED.1080p.BluRay.x264.DTS-HD.MA.5.1-SOMEGROUP';

Future<List<String>> _open(WidgetTester tester, String label) async {
  tester.view.physicalSize = const Size(360, 740);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final List<String> picked = <String>[];
  final MenuController menu = MenuController();
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Align(
          alignment: Alignment.topLeft,
          child: MenuAnchor(
            controller: menu,
            menuChildren: <Widget>[
              SubmenuButton(
                menuChildren: <Widget>[
                  LongMenuItem(
                    label: label,
                    onPressed: () => picked.add(label),
                  ),
                ],
                child: const Text('Translate'),
              ),
            ],
            child: TextButton(onPressed: menu.open, child: const Text('menu')),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('menu'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('Translate'));
  await tester.pumpAndSettle();
  return picked;
}

RenderParagraph _paragraph(WidgetTester tester, String label) =>
    tester.renderObject<RenderParagraph>(
      find.descendant(of: find.text(label), matching: find.byType(RichText)),
    );

void main() {
  testWidgets('a long label is cut inside the screen', (
    WidgetTester tester,
  ) async {
    await _open(tester, _kLong);

    expect(_paragraph(tester, _kLong).didExceedMaxLines, isTrue);
    expect(
      tester.getRect(find.byType(MenuItemButton)).right,
      lessThanOrEqualTo(360),
    );
  });

  testWidgets('a long press reads the full label and picks nothing', (
    WidgetTester tester,
  ) async {
    final List<String> picked = await _open(tester, _kLong);

    await tester.longPress(find.text(_kLong));
    await tester.pumpAndSettle();

    expect(find.text(_kLong), findsNWidgets(2));
    expect(picked, isEmpty);
  });

  testWidgets('a short label shows whole and a tap picks it', (
    WidgetTester tester,
  ) async {
    final List<String> picked = await _open(tester, 'English · srt');

    expect(_paragraph(tester, 'English · srt').didExceedMaxLines, isFalse);

    await tester.tap(find.text('English · srt'));
    await tester.pumpAndSettle();

    expect(picked, <String>['English · srt']);
  });
}
