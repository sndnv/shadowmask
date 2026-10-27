import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/child_mode_store.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('child mode is off until an age is saved', () async {
    expect(await const ChildModeStore().load(), isNull);
  });

  test(
    'a saved age is loaded back and clearing it turns child mode off',
    () async {
      const ChildModeStore store = ChildModeStore();

      await store.save(13);
      expect(await store.load(), 13);

      await store.save(0);
      expect(await store.load(), 0);

      await store.save(null);
      expect(await store.load(), isNull);
    },
  );

  test('an age no longer offered reads as the offered one below it', () async {
    const ChildModeStore store = ChildModeStore();

    await store.save(12);
    expect(await store.load(), 8);

    await store.save(6);
    expect(await store.load(), 0);

    await store.save(40);
    expect(await store.load(), 17);
  });
}
