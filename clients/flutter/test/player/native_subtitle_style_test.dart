import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:shadowmask/player/native_player_controller.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

final ValueVariant<TargetPlatform> _platforms = ValueVariant<TargetPlatform>(
  <TargetPlatform>{TargetPlatform.android, TargetPlatform.linux},
);

void main() {
  testWidgets('native subtitles take the app font and keep their own look', (
    WidgetTester tester,
  ) async {
    debugDefaultTargetPlatformOverride = _platforms.currentValue;
    final ThemeData theme = buildTheme(AppThemeVariant.dark);
    late TextStyle style;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme,
        home: Builder(
          builder: (BuildContext context) {
            style = subtitleTextStyle(context, 20);
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    debugDefaultTargetPlatformOverride = null;

    final TextStyle body = theme.textTheme.bodyMedium!;
    final TextStyle own = const SubtitleViewConfiguration().style;
    expect(style.inherit, isFalse);
    expect(body.fontFamily, isNotNull);
    expect(style.fontFamily, body.fontFamily);
    expect(style.fontFamilyFallback, isEmpty);
    expect(style.fontSize, 20);
    expect(style.color, own.color);
    expect(style.backgroundColor, own.backgroundColor);
    expect(style.height, own.height);
    expect(style.fontWeight, own.fontWeight);
    expect(style.letterSpacing, own.letterSpacing);
    expect(style.wordSpacing, own.wordSpacing);
  }, variant: _platforms);
}
