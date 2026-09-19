import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/playback_block.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shared_preferences/shared_preferences.dart';

Widget _host() => MaterialApp(
  theme: buildTheme(AppThemeVariant.dark),
  home: const Scaffold(
    body: ToastHost(child: SingleChildScrollView(child: PlaybackBlock())),
  ),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the account page offers what the player panel offers', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerDiagnostics), findsOneWidget);
    expect(find.byTooltip(Strings.playerAutoplayNext), findsOneWidget);
  });

  testWidgets('the autoplay control says what it is, on screen', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(
      find.text(Strings.playerAutoplayNext),
      findsOneWidget,
      reason: 'a tooltip is unreachable without a pointer',
    );
  });

  testWidgets('autoplay reads as a row, laid out like the toggle below it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    final Rect label = tester.getRect(find.text(Strings.playerAutoplayNext));
    final Rect help = tester.getRect(find.text(Strings.playerAutoplayNextHelp));
    final Rect control = tester.getRect(find.byType(AppDropdown<int>));

    expect(label.left, lessThan(control.left));
    expect(label.right, lessThanOrEqualTo(control.left));
    expect(help.right, lessThanOrEqualTo(control.left));

    TextStyle styleOf(String text) =>
        (tester.renderObject(find.text(text)) as RenderParagraph).text.style!;

    expect(
      styleOf(Strings.playerAutoplayNext).fontSize,
      styleOf(Strings.playerDiagnostics).fontSize,
    );
    expect(
      styleOf(Strings.playerAutoplayNext).fontWeight,
      styleOf(Strings.playerDiagnostics).fontWeight,
    );
    expect(
      styleOf(Strings.playerAutoplayNextHelp).fontSize,
      styleOf(Strings.playerDiagnosticsHelp).fontSize,
    );
  });

  testWidgets('the autoplay row is padded like the toggle under it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    final Rect row = tester.getRect(find.byType(ListTile).first);
    final Rect toggleRow = tester.getRect(find.byType(ListTile).last);
    final Rect label = tester.getRect(find.text(Strings.playerAutoplayNext));
    final Rect help = tester.getRect(find.text(Strings.playerAutoplayNextHelp));
    final Rect divider = tester.getRect(find.byType(Divider));
    final Rect toggle = tester.getRect(find.text(Strings.playerDiagnostics));

    expect(
      label.top - row.top,
      moreOrLessEquals(toggle.top - toggleRow.top, epsilon: 1),
      reason: 'the two rows inset their heading differently',
    );
    expect(
      divider.top - help.bottom,
      moreOrLessEquals(toggle.top - divider.bottom, epsilon: 1),
      reason: 'the divider sat closer to one row than the other',
    );
  });

  testWidgets('a setting with no save button confirms it was saved', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(find.byType(SwitchListTile));
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastSettingSaved), findsOneWidget);
  });

  testWidgets('it shows what the player already saved, not the defaults', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.autoplay': 30,
      'shadowmask.player.diagnostics': true,
    });

    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerAutoplayDelay(30)), findsOneWidget);
    expect(
      tester.widget<SwitchListTile>(find.byType(SwitchListTile)).value,
      isTrue,
    );
  });

  testWidgets('turning autoplay off from the account page persists it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerAutoplayDelay(10)));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerAutoplayOff).last);
    await tester.pumpAndSettle();

    expect(
      (await const PlayerPrefsStore().load()).autoplaySeconds,
      0,
      reason: 'the player reads the same store on its next launch',
    );
  });

  testWidgets('turning diagnostics on from the account page persists it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(find.byType(SwitchListTile));
    await tester.pumpAndSettle();

    expect((await const PlayerPrefsStore().load()).diagnostics, isTrue);
  });
}
