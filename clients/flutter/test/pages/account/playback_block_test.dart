import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/account/playback_block.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/setting_rows.dart';

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

    final Rect row = tester.getRect(
      find.widgetWithText(ListTile, Strings.playerAutoplayNext),
    );
    final Rect toggleRow = tester.getRect(
      find.widgetWithText(ListTile, Strings.playerDiagnostics),
    );
    final Rect label = tester.getRect(find.text(Strings.playerAutoplayNext));
    final Rect help = tester.getRect(find.text(Strings.playerAutoplayNextHelp));
    final Rect divider = tester.getRect(find.byType(Divider).first);
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

    await tester.ensureVisible(_diagnostics);
    await tester.tap(_diagnostics);
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
    expect(tester.widget<SwitchListTile>(_diagnostics).value, isTrue);
  });

  testWidgets('autoplay can be set to start the next episode at once', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerAutoplayDelay(10)));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerAutoplayImmediately).last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerAutoplayImmediately), findsOneWidget);
    expect(
      (await const PlayerPrefsStore().load()).autoplayNextS,
      kAutoplayImmediately,
    );
  });

  testWidgets('turning autoplay off from the account page persists it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playerAutoplayDelay(10)));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerAutoplayOff).hitTestable().last);
    await tester.pumpAndSettle();

    expect(
      (await const PlayerPrefsStore().load()).autoplayNextS,
      0,
      reason: 'the player reads the same store on its next launch',
    );
  });

  testWidgets('the settings are grouped into playback, video, subtitles and '
      'sound', (WidgetTester tester) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    double top(String text) => tester.getTopLeft(find.text(text)).dy;
    final List<(String, List<String>)> sections = <(String, List<String>)>[
      (
        Strings.accountPlaybackHeading,
        <String>[Strings.playerAutoplayNext, Strings.playerDiagnostics],
      ),
      (Strings.videoHeading, <String>[Strings.playerDelivery]),
      (Strings.subtitlesHeading, <String>[Strings.subtitleSize]),
      (
        Strings.soundHeading,
        <String>[
          Strings.playerStereoDownmix,
          Strings.playerPreventDistortion,
          Strings.playerDialogue,
          Strings.playerNightMode,
        ],
      ),
    ];
    for (int i = 0; i < sections.length; i++) {
      final (String heading, List<String> rows) = sections[i];
      final double next = i + 1 < sections.length
          ? top(sections[i + 1].$1)
          : double.infinity;
      for (final String row in rows) {
        expect(top(row), greaterThan(top(heading)), reason: row);
        expect(top(row), lessThan(next), reason: row);
      }
    }
  });

  testWidgets('every setting row is laid out alike, however long its help', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    for (final (String title, String help) in <(String, String)>[
      (Strings.playerAutoplayNext, Strings.playerAutoplayNextHelp),
      (Strings.playerDiagnostics, Strings.playerDiagnosticsHelp),
      (Strings.playerDelivery, Strings.playerDeliveryHelp),
      (Strings.subtitleSize, Strings.subtitleSizeHelp),
      (Strings.playerStereoDownmix, Strings.playerStereoDownmixHelp),
      (Strings.playerPreventDistortion, Strings.playerPreventDistortionHelp),
      (Strings.playerDialogue, Strings.playerDialogueHelp),
      (Strings.playerNightMode, Strings.playerNightModeHelp),
    ]) {
      expectSettingRow(tester, title, help);
    }
  });

  testWidgets('converting sets what each video starts with', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerDeliveryHelp), findsOneWidget);

    await tester.tap(find.text(Strings.playerDeliveryAuto));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playerDeliveryAlways).last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastSettingSaved), findsOneWidget);
    expect(
      (await const PlayerPrefsStore().load()).defaultDelivery,
      DeliveryPreference.always,
    );
  });

  testWidgets('the subtitle size is chosen here and saved', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.subtitleSizeHelp), findsOneWidget);

    await tester.ensureVisible(find.byType(AppDropdown<SubtitleSize>));
    await tester.tap(find.text(Strings.subtitleSizeDefault));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.subtitleSizeLarge).last);
    await tester.pumpAndSettle();

    expect(find.text(Strings.toastSettingSaved), findsOneWidget);
    expect(
      (await const PlayerPrefsStore().load()).subtitleSize,
      SubtitleSize.large,
    );
  });

  testWidgets('it shows the subtitle size already saved', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.subtitle_size': 'small',
    });

    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.subtitleSizeSmall), findsOneWidget);
  });

  testWidgets('it shows the converting default already saved', (
    WidgetTester tester,
  ) async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.delivery': 'never',
    });

    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    expect(find.text(Strings.playerDeliveryNever), findsOneWidget);
  });

  testWidgets('turning diagnostics on from the account page persists it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.ensureVisible(_diagnostics);
    await tester.tap(_diagnostics);
    await tester.pumpAndSettle();

    expect((await const PlayerPrefsStore().load()).diagnostics, isTrue);
  });

  testWidgets('each setting says here what it says in the player', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    for (final String help in <String>[
      Strings.playerDeliveryHelp,
      Strings.playerStereoDownmixHelp,
      Strings.playerPreventDistortionHelp,
      Strings.playerDialogueHelp,
      Strings.playerNightModeHelp,
    ]) {
      expect(find.text(help), findsOneWidget, reason: help);
    }
  });

  testWidgets('the sound switches start on their defaults and persist', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    final Finder downmix = find.widgetWithText(
      SwitchListTile,
      Strings.playerStereoDownmix,
    );
    final Finder normalize = find.widgetWithText(
      SwitchListTile,
      Strings.playerPreventDistortion,
    );
    expect(tester.widget<SwitchListTile>(downmix).value, isFalse);
    expect(tester.widget<SwitchListTile>(normalize).value, isTrue);

    for (final Finder row in <Finder>[downmix, normalize]) {
      await tester.ensureVisible(row);
      await tester.tap(row);
      await tester.pumpAndSettle();
    }

    final LocalSettings saved = await const PlayerPrefsStore().load();
    expect(saved.defaultDownmix, isTrue);
    expect(saved.defaultNormalize, isFalse);
  });

  testWidgets('the dialogue lift and night mode are chosen here and saved', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.byType(AppDropdown<DialogueLift>));
    await tester.tap(find.byType(AppDropdown<DialogueLift>));
    await tester.pumpAndSettle();
    await tester.tap(find.text(DialogueLift.db6.label).last);
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.byType(AppDropdown<NightMode>));
    await tester.tap(find.byType(AppDropdown<NightMode>));
    await tester.pumpAndSettle();
    await tester.tap(find.text(NightMode.strong.label).last);
    await tester.pumpAndSettle();

    final LocalSettings saved = await const PlayerPrefsStore().load();
    expect(saved.defaultDialogue, DialogueLift.db6);
    expect(saved.defaultNight, NightMode.strong);
  });
}

final Finder _diagnostics = find.widgetWithText(
  SwitchListTile,
  Strings.playerDiagnostics,
);
