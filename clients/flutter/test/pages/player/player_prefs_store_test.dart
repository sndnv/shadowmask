import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const PlayerPrefsStore store = PlayerPrefsStore();

  test(
    'autoplay defaults to ten seconds on a device with nothing stored',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});

      expect((await store.load()).autoplayNextS, 10);
    },
  );

  test('converting starts on Auto and a chosen default survives', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect((await store.load()).defaultDelivery, DeliveryPreference.auto);

    await store.saveDelivery(DeliveryPreference.always);

    expect((await store.load()).defaultDelivery, DeliveryPreference.always);
  });

  test(
    'the subtitle size starts on Default and a chosen one survives',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});

      expect((await store.load()).subtitleSize, SubtitleSize.deviceDefault);

      await store.saveSubtitleSize(SubtitleSize.large);

      expect((await store.load()).subtitleSize, SubtitleSize.large);
    },
  );

  test('a subtitle size no longer offered reads as Default', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.subtitle_size': 'huge',
    });

    expect((await store.load()).subtitleSize, SubtitleSize.deviceDefault);
  });

  test('a converting value no longer offered reads as Auto', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.delivery': 'sometimes',
    });

    expect((await store.load()).defaultDelivery, DeliveryPreference.auto);
  });

  test(
    'the sound settings start on their defaults and chosen ones survive',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});

      final LocalSettings fresh = await store.load();
      expect(fresh.defaultDownmix, isFalse);
      expect(fresh.defaultNormalize, isTrue);
      expect(fresh.defaultDialogue, DialogueLift.none);
      expect(fresh.defaultNight, NightMode.off);

      await store.saveDownmix(true);
      await store.saveNormalize(false);
      await store.saveDialogue(DialogueLift.db9);
      await store.saveNight(NightMode.strong);

      final LocalSettings saved = await store.load();
      expect(saved.defaultDownmix, isTrue);
      expect(saved.defaultNormalize, isFalse);
      expect(saved.defaultDialogue, DialogueLift.db9);
      expect(saved.defaultNight, NightMode.strong);
    },
  );

  test('a sound value no longer offered reads as its default', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.dialogue': 5,
      'shadowmask.player.night': 'loud',
    });

    final LocalSettings read = await store.load();
    expect(read.defaultDialogue, DialogueLift.none);
    expect(read.defaultNight, NightMode.off);
  });

  test('a chosen delay survives a reload', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveAutoplaySeconds(30);

    expect((await store.load()).autoplayNextS, 30);
  });

  test('off is a real choice, not a missing one', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveAutoplaySeconds(0);

    expect(
      (await store.load()).autoplayNextS,
      0,
      reason: 'zero has to survive, or opting out would silently revert to on',
    );
  });

  test('immediately is kept apart from off', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveAutoplaySeconds(kAutoplayImmediately);

    expect((await store.load()).autoplayNextS, kAutoplayImmediately);
    expect(kAutoplayImmediately, isNot(kAutoplayOff));
  });

  test('every autoplay choice reads as what it does', () {
    expect(autoplayLabel(kAutoplayOff), Strings.playerAutoplayOff);
    expect(
      autoplayLabel(kAutoplayImmediately),
      Strings.playerAutoplayImmediately,
    );
    expect(autoplayLabel(15), Strings.playerAutoplayDelay(15));
  });

  test('a delay that is no longer offered falls back to the default', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.autoplay': 45,
    });

    expect(
      (await store.load()).autoplayNextS,
      10,
      reason:
          'a stored value outside kAutoplayDelays would not match any '
          'dropdown item, and the dropdown would throw on a value it cannot show',
    );
  });

  test(
    'a first launch starts unmuted, windowed and with nothing extra on',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});

      final LocalSettings first = await store.load();

      expect(first.volume, 1);
      expect(first.muted, isFalse);
      expect(first.mutedByPolicy, isFalse);
      expect(first.fullscreen, isFalse);
      expect(first.pictureInPicture, isFalse);
      expect(first.remaining, isFalse);
      expect(first.diagnostics, isFalse);
    },
  );

  test('the other player preferences still round-trip', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveVolume(0.5);
    await store.saveRemaining(true);
    final LocalSettings prefs = await store.load();

    expect(prefs.volume, 0.5);
    expect(prefs.remaining, isTrue);
  });

  test('mute is remembered separately from the level it silences', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveVolume(0.4);
    await store.saveMuted(true);
    final LocalSettings prefs = await store.load();

    expect(
      prefs.muted,
      isTrue,
      reason:
          'each episode builds a fresh video element, which defaults to '
          'unmuted, so a mute that is not stored comes back at full volume',
    );
    expect(
      prefs.volume,
      0.4,
      reason: 'unmuting has to restore the level, not jump to full',
    );
  });

  test('wide screen is remembered', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect((await store.load()).wide, isFalse);

    await store.saveWide(true);

    expect((await store.load()).wide, isTrue);
  });

  test('the buffer target starts at a minute and survives a reload', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect((await store.load()).bufferS, 60);

    await store.saveBufferSeconds(300);

    expect((await store.load()).bufferS, 300);
  });

  test('a buffer target that is no longer offered falls back', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.buffer_seconds': 45,
    });

    expect(
      (await store.load()).bufferS,
      60,
      reason:
          'the dropdown throws on a value it cannot show, so a target outside '
          'kBufferTargets has to come back as the default',
    );
  });

  test('the buffer limit starts at 256MB and survives a reload', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect((await store.load()).bufferBytes, 256 * 1024 * 1024);

    await store.saveBufferBytes(1024 * 1024 * 1024);

    expect((await store.load()).bufferBytes, 1024 * 1024 * 1024);
  });

  test('a buffer limit that is no longer offered falls back', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.buffer_bytes': 12345,
    });

    expect((await store.load()).bufferBytes, 256 * 1024 * 1024);
  });

  test('waiting for the buffer is off until it is chosen', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect(
      (await store.load()).waitForBuffer,
      isFalse,
      reason: 'holding playback by default would make every start feel broken',
    );

    await store.saveWaitForBuffer(true);

    expect((await store.load()).waitForBuffer, isTrue);
  });

  test('a change saves the player settings that changed and leaves the '
      'wide screen and the converting default to their owners', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.wide': true,
      'shadowmask.player.delivery': 'never',
    });
    final LocalSettings was = await store.load();
    final LocalSettings now = was.copyWith(
      speed: 2,
      volume: 0.5,
      muted: true,
      remaining: true,
      diagnostics: true,
      autoplayNextS: 30,
      networkTimeoutS: 60,
      bufferS: 300,
      bufferBytes: 1024 * 1024 * 1024,
      waitForBuffer: true,
      subtitleSize: SubtitleSize.large,
      wide: false,
      defaultDelivery: DeliveryPreference.always,
    );

    await store.saveChanged(was, now);

    expect(
      await store.load(),
      now.copyWith(
        speed: 1,
        wide: true,
        defaultDelivery: DeliveryPreference.never,
      ),
    );
  });

  test('a change writes nothing for the settings it left alone', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveChanged(
      const LocalSettings(),
      const LocalSettings(volume: 0.5),
    );

    expect((await SharedPreferences.getInstance()).getKeys(), <String>{
      'shadowmask.player.volume',
    });
  });
}
