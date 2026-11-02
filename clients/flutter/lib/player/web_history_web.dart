import 'dart:js_interop';

import 'package:web/web.dart' as web;

void replaceUrl(String relative) {
  web.window.history.replaceState(null, '', relative);
}

void Function() addUnloadListener(void Function() onUnload) {
  final JSFunction listener = ((web.Event _) => onUnload()).toJS;
  web.window.addEventListener('pagehide', listener);
  return () => web.window.removeEventListener('pagehide', listener);
}

void Function() addRestoreListener(void Function() onRestore) {
  final JSFunction listener = ((web.PageTransitionEvent event) {
    if (event.persisted) {
      onRestore();
    }
  }).toJS;
  web.window.addEventListener('pageshow', listener);
  return () => web.window.removeEventListener('pageshow', listener);
}

void reloadPage() {
  web.window.location.reload();
}
