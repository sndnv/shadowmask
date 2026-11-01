import 'dart:js_interop';

import 'package:flutter/services.dart';
import 'package:web/web.dart' as web;

import 'package:shadowmask/util/clipboard_fallback.dart';

BinaryMessenger withClipboardFallback(BinaryMessenger inner) =>
    ClipboardFallbackMessenger(
      inner,
      secure: () => web.window.isSecureContext,
      copy: copyWithCommand,
    );

bool copyWithCommand(String text) {
  bool written = false;
  final JSFunction listener = ((web.ClipboardEvent event) {
    final web.DataTransfer? data = event.clipboardData;
    if (data == null) {
      return;
    }
    data.setData('text/plain', text);
    event.preventDefault();
    written = true;
  }).toJS;
  web.document.addEventListener('copy', listener);
  try {
    return web.document.execCommand('copy') && written;
  } on Object {
    return false;
  } finally {
    web.document.removeEventListener('copy', listener);
  }
}
