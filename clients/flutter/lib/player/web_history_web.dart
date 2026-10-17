import 'dart:js_interop';

import 'package:web/web.dart' as web;

void replaceUrl(String relative) {
  web.window.history.replaceState(null, '', relative);
}

void addUnloadListener(void Function() onUnload) {
  web.window.addEventListener('pagehide', ((web.Event _) => onUnload()).toJS);
}

void addRestoreListener(void Function() onRestore) {
  web.window.addEventListener(
    'pageshow',
    ((web.PageTransitionEvent event) {
      if (event.persisted) {
        onRestore();
      }
    }).toJS,
  );
}

void reloadPage() {
  web.window.location.reload();
}
