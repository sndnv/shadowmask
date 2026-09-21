import 'dart:io';

import 'package:shadowmask/util/bundled_licenses.dart';

BundlePlatform bundlePlatform() {
  if (Platform.isMacOS) {
    return BundlePlatform.macos;
  }
  if (Platform.isIOS) {
    return BundlePlatform.ios;
  }
  if (Platform.isAndroid) {
    return BundlePlatform.android;
  }
  if (Platform.isLinux) {
    return BundlePlatform.linux;
  }
  return BundlePlatform.other;
}

String clientPlatform() {
  if (Platform.isAndroid) {
    return 'android';
  }
  if (Platform.isIOS) {
    return 'ios';
  }
  return 'desktop';
}
