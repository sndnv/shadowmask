import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const PlayerPrefsStore store = PlayerPrefsStore();

  test(
    'autoplay defaults to ten seconds on a device with nothing stored',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});

      expect((await store.load()).autoplaySeconds, 10);
    },
  );

  test('a chosen delay survives a reload', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveAutoplaySeconds(30);

    expect((await store.load()).autoplaySeconds, 30);
  });

  test('off is a real choice, not a missing one', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveAutoplaySeconds(0);

    expect(
      (await store.load()).autoplaySeconds,
      0,
      reason: 'zero has to survive, or opting out would silently revert to on',
    );
  });

  test('a delay that is no longer offered falls back to the default', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.autoplay': 45,
    });

    expect(
      (await store.load()).autoplaySeconds,
      10,
      reason:
          'a stored value outside kAutoplayDelays would not match any '
          'dropdown item, and the dropdown would throw on a value it cannot show',
    );
  });

  test('the other player preferences still round-trip', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveVolume(0.5);
    await store.saveRemaining(true);
    final PlayerPrefs prefs = await store.load();

    expect(prefs.volume, 0.5);
    expect(prefs.remaining, isTrue);
  });

  test('mute is remembered separately from the level it silences', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    await store.saveVolume(0.4);
    await store.saveMuted(true);
    final PlayerPrefs prefs = await store.load();

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

    expect((await store.load()).bufferSeconds, 60);

    await store.saveBufferSeconds(300);

    expect((await store.load()).bufferSeconds, 300);
  });

  test('a buffer target that is no longer offered falls back', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.player.buffer_seconds': 45,
    });

    expect(
      (await store.load()).bufferSeconds,
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
}
