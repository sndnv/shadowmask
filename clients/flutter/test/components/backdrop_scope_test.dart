import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/util/scoped_value.dart';

const Widget _pixel = SizedBox(key: Key('pixel'));

const Artwork _artwork = Artwork(
  backdrops: <ImageSet>[
    ImageSet(base: '/artwork/1', widths: <int>[kBackdropWidth]),
  ],
);

Future<Image> _pumpWash(WidgetTester tester, String? url) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(body: BackdropWash(url: url)),
    ),
  );
  await tester.pump();
  return tester.widget<Image>(find.byType(Image));
}

void main() {
  testWidgets('the wash ramps up from transparent once a frame decodes', (
    WidgetTester tester,
  ) async {
    final Image image = await _pumpWash(tester, 'http://test/backdrop.jpg');
    final BuildContext context = tester.element(find.byType(Image));

    final AnimatedOpacity pending =
        image.frameBuilder!(context, _pixel, null, false) as AnimatedOpacity;
    expect(pending.opacity, 0);
    expect(pending.duration, kBackdropFade);

    final AnimatedOpacity decoded =
        image.frameBuilder!(context, _pixel, 0, false) as AnimatedOpacity;
    expect(decoded.opacity, kBackdropOpacity);
  });

  testWidgets('an already decoded image is painted without a fade', (
    WidgetTester tester,
  ) async {
    final Image image = await _pumpWash(tester, 'http://test/backdrop.jpg');
    final BuildContext context = tester.element(find.byType(Image));

    final Widget cached = image.frameBuilder!(context, _pixel, 0, true);
    expect(cached, isA<Opacity>());
    expect((cached as Opacity).opacity, kBackdropOpacity);
  });

  testWidgets('clearing the url cross-fades the wash out', (
    WidgetTester tester,
  ) async {
    await _pumpWash(tester, 'http://test/backdrop.jpg');

    await tester.pumpWidget(
      MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: const Scaffold(body: BackdropWash(url: null)),
      ),
    );
    await tester.pump(const Duration(milliseconds: 40));
    expect(find.byType(Image), findsOneWidget);

    await tester.pump(kBackdropFade);
    expect(find.byType(Image), findsNothing);
  });

  testWidgets('leaving a page clears the backdrop without a locked tree', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> backdrop = ScopedValue<String?>(null);
    addTearDown(backdrop.dispose);

    Widget host({required bool onPage}) => MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Stack(
          children: <Widget>[
            ValueListenableBuilder<String?>(
              valueListenable: backdrop,
              builder: (BuildContext context, String? url, Widget? _) =>
                  BackdropWash(url: url),
            ),
            BackdropScope(
              url: backdrop,
              child: onPage
                  ? const PageBackdrop(
                      artwork: _artwork,
                      imageBase: 'http://test',
                    )
                  : const SizedBox.shrink(),
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(host(onPage: true));
    await tester.pump();
    expect(backdrop.value, 'http://test/artwork/1/$kBackdropWidth');

    await tester.pumpWidget(host(onPage: false));
    expect(tester.takeException(), isNull);

    await tester.pump();
    expect(backdrop.value, isNull);
  });

  testWidgets('a page that already handed the backdrop on does not clear it', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> backdrop = ScopedValue<String?>(null);
    addTearDown(backdrop.dispose);

    Widget host({required bool onPage}) => MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: BackdropScope(
          url: backdrop,
          child: onPage
              ? const PageBackdrop(artwork: _artwork, imageBase: 'http://test')
              : const SizedBox.shrink(),
        ),
      ),
    );

    await tester.pumpWidget(host(onPage: true));
    await tester.pump();

    const String taken = 'http://test/artwork/9/$kBackdropWidth';
    backdrop.value = taken;
    await tester.pumpWidget(host(onPage: false));
    await tester.pump();

    expect(backdrop.value, taken);
  });
}
