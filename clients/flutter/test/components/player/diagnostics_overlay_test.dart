import 'dart:ui' show DisplayFeature, DisplayFeatureState, DisplayFeatureType;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/player/diagnostics_overlay.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/player/player_diagnostics.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

class _StubController implements PlayerController {
  @override
  PlayerDiagnostics diagnostics() =>
      const PlayerDiagnostics(<String>['mode: direct']);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

Widget _host({
  EdgeInsets padding = EdgeInsets.zero,
  EdgeInsets viewPadding = EdgeInsets.zero,
  List<DisplayFeature> features = const <DisplayFeature>[],
}) => ThemeScope(
  variant: AppThemeVariant.dark,
  setVariant: (_) {},
  child: MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: MediaQuery(
      data: MediaQueryData(
        padding: padding,
        viewPadding: viewPadding,
        displayFeatures: features,
      ),
      child: Scaffold(body: DiagnosticsOverlay(controller: _StubController())),
    ),
  ),
);

String _text(WidgetTester tester) =>
    tester.widget<SelectableText>(find.byType(SelectableText)).data ?? '';

void main() {
  testWidgets('the overlay reports the insets the layout is working from', (
    WidgetTester tester,
  ) async {
    // A fullscreen player hides the system bars, which zeroes padding while
    // the physical obstruction stays in viewPadding. Reading both is what
    // separates a real cutout from one the emulator only paints.
    await tester.pumpWidget(
      _host(
        viewPadding: const EdgeInsets.only(left: 44, bottom: 24),
        features: const <DisplayFeature>[
          DisplayFeature(
            bounds: Rect.fromLTWH(0, 0, 44, 120),
            type: DisplayFeatureType.cutout,
            state: DisplayFeatureState.unknown,
          ),
        ],
      ),
    );

    expect(
      _text(tester),
      contains('insets: padding 0,0,0,0  view 44,0,0,24  cutouts 1'),
    );
    expect(_text(tester), contains('mode: direct'));
  });

  testWidgets('a display with no cutout says so rather than staying silent', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(padding: const EdgeInsets.only(top: 24)));

    expect(
      _text(tester),
      contains('insets: padding 0,24,0,0  view 0,0,0,0  cutouts 0'),
    );
  });
}
