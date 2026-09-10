import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/capability_overrides_store.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const CapabilityOverridesStore store = CapabilityOverridesStore();

  test('a device with nothing stored overrides nothing', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});

    expect(await store.load(), const CapabilityOverrides());
  });

  test('a choice survives a reload', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    const CapabilityOverrides overrides = CapabilityOverrides(
      codecs: <String, CodecSupport>{'vp9': CodecSupport.software},
      maxHeight: 1080,
      hdr: HdrChoice.deny,
    );

    await store.save(overrides);

    expect(await store.load(), overrides);
  });

  test('clearing the overrides removes the stored value', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{});
    await store.save(const CapabilityOverrides(maxFrameRate: 30));

    await store.save(const CapabilityOverrides());

    expect(await store.load(), const CapabilityOverrides());
  });

  // A device that stored a value under an older build must still start.
  test('a stored value that cannot be read overrides nothing', () async {
    SharedPreferences.setMockInitialValues(<String, Object>{
      'shadowmask.capabilities': 'not json',
    });

    expect(await store.load(), const CapabilityOverrides());
  });
}
