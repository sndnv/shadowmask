import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

abstract final class BrowserMenu {
  static int _holds = 0;

  static int get holds => _holds;

  static void suppress() {
    _holds += 1;
    if (kIsWeb && _holds == 1) {
      unawaited(BrowserContextMenu.disableContextMenu());
    }
  }

  static void restore() {
    if (_holds == 0) {
      return;
    }
    _holds -= 1;
    if (kIsWeb && _holds == 0) {
      unawaited(BrowserContextMenu.enableContextMenu());
    }
  }
}
