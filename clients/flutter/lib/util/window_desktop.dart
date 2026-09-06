import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:screen_retriever/screen_retriever.dart';
import 'package:window_manager/window_manager.dart';

import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/breakpoints.dart';
import 'package:shadowmask/util/window_store.dart';

const Size kMinimumWindowSize = Size(Breakpoints.sm, Breakpoints.sm);
const Duration kGeometrySettleDelay = Duration(milliseconds: 400);

Future<void> configureWindow({WindowStore store = const WindowStore()}) async {
  if (!Platform.isLinux && !Platform.isMacOS && !Platform.isWindows) {
    return;
  }
  await windowManager.ensureInitialized();
  await windowManager.setMinimumSize(kMinimumWindowSize);
  await windowManager.setTitle(Strings.appTitle);
  final Rect? saved = await store.load();
  if (saved != null && await onScreen(saved)) {
    await windowManager.setBounds(saved);
  }
  windowManager.addListener(_GeometryListener(store));
}

Future<bool> onScreen(Rect bounds) async {
  final List<Display> displays = await screenRetriever.getAllDisplays();
  return displays.any((Display display) {
    final Offset origin = display.visiblePosition ?? Offset.zero;
    final Size size = display.visibleSize ?? display.size;
    return (origin & size).contains(bounds.topLeft);
  });
}

class _GeometryListener with WindowListener {
  _GeometryListener(this._store);

  final WindowStore _store;
  Timer? _settle;

  @override
  void onWindowResized() => _schedule();

  @override
  void onWindowMoved() => _schedule();

  void _schedule() {
    _settle?.cancel();
    _settle = Timer(kGeometrySettleDelay, _save);
  }

  Future<void> _save() async {
    if (await windowManager.isFullScreen() ||
        await windowManager.isMaximized()) {
      return;
    }
    await _store.save(await windowManager.getBounds());
  }
}
