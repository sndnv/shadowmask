import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/capability_scope.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/pages/account/playback_support_block.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

Widget _host({
  String platform = 'android',
  ClientDecoding? decoding,
  bool scoped = true,
  CapabilityOverrides overrides = const CapabilityOverrides(),
  ValueChanged<CapabilityOverrides>? onChanged,
}) {
  final Widget app = MaterialApp(
    theme: buildTheme(AppThemeVariant.dark),
    home: const Scaffold(
      body: SingleChildScrollView(child: PlaybackSupportBlock()),
    ),
  );
  return ThemeScope(
    variant: AppThemeVariant.dark,
    setVariant: (_) {},
    child: scoped
        ? CapabilityScope(
            platform: platform,
            measured: decoding,
            overrides: overrides,
            setOverrides: onChanged,
            child: app,
          )
        : app,
  );
}

void main() {
  testWidgets('a device that measured nothing shows only its device type', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(platform: 'chrome'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportHeading), findsOneWidget);
    expect(find.text('chrome'), findsOneWidget);
    expect(find.text(Strings.playbackSupportVideo), findsNothing);
  });

  testWidgets('a codec that only software decodes says so', (
    WidgetTester tester,
  ) async {
    // Supported but not smooth is the case a profile cannot express, and
    // hiding it is what made the phone stutter without explanation.
    await tester.pumpWidget(
      _host(
        decoding: const ClientDecoding(
          video: <VideoCodecCap>[
            VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: true),
            VideoCodecCap(codec: 'av1', maxBitDepth: 10, smooth: false),
          ],
          audio: <AudioCodecCap>[AudioCodecCap(codec: 'eac3', maxChannels: 8)],
          hdr: <String>['hdr10'],
          maxWidth: 3840,
          maxHeight: 2160,
          maxFrameRate: 30,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('3840x2160@30'), findsOneWidget);
    expect(find.text('hevc · 10-bit · hardware'), findsOneWidget);
    expect(find.text('av1 · 10-bit · software'), findsOneWidget);
    expect(find.text('eac3 · up to 8 channels'), findsOneWidget);
    expect(find.text('hdr10'), findsOneWidget);
  });

  testWidgets('a device with no high dynamic range says none', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        decoding: const ClientDecoding(
          video: <VideoCodecCap>[
            VideoCodecCap(codec: 'h264', maxBitDepth: 8, smooth: true),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportLargestPicture), findsNothing);
    expect(find.text(Strings.playbackSupportNone), findsNWidgets(2));
  });

  testWidgets('nothing renders without a scope to read', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(scoped: false));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportHeading), findsNothing);
  });

  // An override degrades every later session, so leaving one set by accident
  // has to be visible without opening the dialog.
  testWidgets('the section says when it is no longer just what was detected', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(decoding: _measured(), onChanged: (_) {}));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playbackSupportChanged), findsNothing);

    await tester.pumpWidget(
      _host(
        decoding: _measured(),
        overrides: const CapabilityOverrides(maxFrameRate: 30),
        onChanged: (_) {},
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportChanged), findsOneWidget);
  });

  testWidgets('a device that cannot be changed offers no way to change it', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(decoding: _measured()));
    await tester.pumpAndSettle();

    expect(find.text(Strings.editPlaybackSupport), findsNothing);
  });

  // The controls live behind the button, matching Profile beside it, so the
  // section itself stays a summary.
  testWidgets('the controls open in a dialog and report the change back', (
    WidgetTester tester,
  ) async {
    CapabilityOverrides? saved;
    await tester.pumpWidget(
      _host(
        decoding: _measured(),
        onChanged: (CapabilityOverrides value) => saved = value,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportFrameRate), findsNothing);

    await tester.tap(find.text(Strings.editPlaybackSupport));
    await tester.pumpAndSettle();
    expect(find.text(Strings.playbackSupportFrameRate), findsOneWidget);

    await tester.ensureVisible(
      find.text(Strings.playbackSupportDetectedAs('hdr10')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playbackSupportDetectedAs('hdr10')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.playbackSupportDeny).last);
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.save));
    await tester.pumpAndSettle();

    expect(saved?.hdr, HdrChoice.deny);
  });

  // The summary has to report what is actually sent, not what was measured,
  // or an override would be invisible on the page that set it.
  testWidgets('an override replaces the measured value in the summary', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        decoding: _measured(),
        overrides: const CapabilityOverrides(
          codecs: <String, CodecSupport>{'vp9': CodecSupport.software},
          maxHeight: 1080,
        ),
        onChanged: (_) {},
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('vp9 · 12-bit · software'), findsOneWidget);
    expect(find.text(Strings.playbackSupportUpTo(1080)), findsWidgets);
  });

  testWidgets('denying high dynamic range empties it in the summary', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _host(
        decoding: _measured(),
        overrides: const CapabilityOverrides(hdr: HdrChoice.deny),
        onChanged: (_) {},
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('hdr10'), findsNothing);
    expect(find.text(Strings.playbackSupportNone), findsOneWidget);
  });

  testWidgets('resetting is offered only once something was changed', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(_host(decoding: _measured(), onChanged: (_) {}));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.editPlaybackSupport));
    await tester.pumpAndSettle();

    expect(find.text(Strings.playbackSupportReset), findsNothing);
  });

  testWidgets('resetting puts every detected value back', (
    WidgetTester tester,
  ) async {
    CapabilityOverrides? saved;
    await tester.pumpWidget(
      _host(
        decoding: _measured(),
        overrides: const CapabilityOverrides(maxFrameRate: 30),
        onChanged: (CapabilityOverrides value) => saved = value,
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.editPlaybackSupport));
    await tester.pumpAndSettle();

    await tester.tap(find.text(Strings.playbackSupportReset));
    await tester.pumpAndSettle();
    await tester.tap(find.text(Strings.save));
    await tester.pumpAndSettle();

    expect(saved, const CapabilityOverrides());
  });
}

ClientDecoding _measured() => const ClientDecoding(
  video: <VideoCodecCap>[
    VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: true),
    VideoCodecCap(codec: 'vp9', maxBitDepth: 12, smooth: true),
  ],
  audio: <AudioCodecCap>[AudioCodecCap(codec: 'aac', maxChannels: 8)],
  hdr: <String>['hdr10'],
);
