import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'package:shadowmask/util/clipboard_route.dart';

class AppBinding extends WidgetsFlutterBinding {
  @override
  BinaryMessenger createBinaryMessenger() =>
      withClipboardFallback(super.createBinaryMessenger());
}
