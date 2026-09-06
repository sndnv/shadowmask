import 'dart:ui';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/window_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('a window keeps the geometry it was left at', () async {
    const WindowStore store = WindowStore();
    await store.save(const Rect.fromLTWH(120, 64, 1280, 800));

    expect(await store.load(), const Rect.fromLTWH(120, 64, 1280, 800));
  });

  test('nothing saved yet means no opinion about geometry', () async {
    expect(await const WindowStore().load(), isNull);
  });

  test(
    'a half written or unparsable entry is discarded, not guessed at',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.window': <String>['1', '2', '3'],
      });
      expect(await const WindowStore().load(), isNull);

      SharedPreferences.setMockInitialValues(<String, Object>{
        'shadowmask.window': <String>['1', '2', 'wide', '4'],
      });
      expect(await const WindowStore().load(), isNull);
    },
  );
}
