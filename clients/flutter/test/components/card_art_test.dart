import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/card_aspect.dart';

Widget _host(Widget child) => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: Scaffold(
    body: Center(child: SizedBox(width: 120, child: child)),
  ),
);

Widget _art(Artwork? artwork) => CardArt(
  artwork: artwork,
  aspect: CardAspect.poster,
  imageBase: 'https://example.invalid/',
);

void main() {
  testWidgets('each aspect has its own empty placeholder', (
    WidgetTester tester,
  ) async {
    for (final (CardAspect aspect, IconData icon) in <(CardAspect, IconData)>[
      (CardAspect.poster, Icons.movie_outlined),
      (CardAspect.landscape, Icons.tv_outlined),
      (CardAspect.person, Icons.person_outline),
    ]) {
      await tester.pumpWidget(
        _host(
          CardArt(
            artwork: null,
            aspect: aspect,
            imageBase: 'https://example.invalid/',
          ),
        ),
      );
      expect(find.byIcon(icon), findsOneWidget, reason: aspect.name);
    }
  });

  testWidgets('a person card is portrait, not landscape', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const CardArt(
          artwork: null,
          aspect: CardAspect.person,
          imageBase: 'https://example.invalid/',
        ),
      ),
    );

    final AspectRatio ratio = tester.widget<AspectRatio>(
      find.byType(AspectRatio).first,
    );
    expect(ratio.aspectRatio, 2 / 3);
  });

  testWidgets('with no artwork it draws the empty placeholder', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(_art(null)));

    expect(find.byIcon(Icons.movie_outlined), findsOneWidget);
    expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
    expect(find.byType(Skeleton), findsNothing);
  });

  testWidgets('a card asks for the rung that matches what it draws', (
    WidgetTester tester,
  ) async {
    const Artwork art = Artwork(
      posters: <ImageSet>[
        ImageSet(base: 'p1', widths: <int>[180, 480, 960]),
      ],
    );

    for (final (double dpr, String rung) in <(double, String)>[
      (1, '180'),
      (2, '480'),
      (4, '960'),
    ]) {
      tester.view.devicePixelRatio = dpr;
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        _host(
          const CardArt(
            artwork: art,
            aspect: CardAspect.poster,
            imageBase: 'https://example.invalid/',
            width: 160,
          ),
        ),
      );

      final Image image = tester.widget<Image>(find.byType(Image));
      final ResizeImage resized = image.image as ResizeImage;
      expect(
        (resized.imageProvider as NetworkImage).url,
        'https://example.invalid/p1/$rung',
        reason: 'a 160px card at dpr $dpr should take the $rung rung',
      );
      expect(
        resized.width,
        (160 * dpr).round(),
        reason: 'and decode at the size it draws, not at source size',
      );
      expect(
        image.width,
        double.infinity,
        reason: 'the layout size must not change with the rung',
      );
    }
  });

  testWidgets('art with no declared width keeps the old fixed rung', (
    WidgetTester tester,
  ) async {
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      _host(
        _art(
          const Artwork(
            posters: <ImageSet>[
              ImageSet(base: 'p1', widths: <int>[180, 480, 960]),
            ],
          ),
        ),
      ),
    );

    expect(
      (tester.widget<Image>(find.byType(Image)).image as NetworkImage).url,
      'https://example.invalid/p1/480',
    );
  });

  testWidgets('a failed image is visibly distinct from an absent one', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        _art(
          const Artwork(
            posters: <ImageSet>[
              ImageSet(base: 'nope', widths: <int>[480]),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));

    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    expect(find.byIcon(Icons.movie_outlined), findsNothing);
  });
}
