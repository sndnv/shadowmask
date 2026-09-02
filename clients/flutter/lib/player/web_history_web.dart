import 'dart:js_interop';

import 'package:web/web.dart' as web;

void replaceUrl(String relative) {
  web.window.history.replaceState(null, '', relative);
}

void addUnloadListener(void Function() onUnload) {
  web.window.addEventListener('pagehide', ((web.Event _) => onUnload()).toJS);
}
