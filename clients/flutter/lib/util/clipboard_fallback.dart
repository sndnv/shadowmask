import 'dart:ui' as ui;

import 'package:flutter/services.dart';

const String kClipboardSetData = 'Clipboard.setData';
const String kCopyFailCode = 'copy_fail';

class ClipboardFallbackMessenger extends BinaryMessenger {
  const ClipboardFallbackMessenger(
    this._inner, {
    required this._secure,
    required this._copy,
  });

  static const MethodCodec _codec = JSONMethodCodec();

  final BinaryMessenger _inner;
  final bool Function() _secure;
  final bool Function(String text) _copy;

  @override
  Future<void> handlePlatformMessage(
    String channel,
    ByteData? data,
    ui.PlatformMessageResponseCallback? callback,
  ) async {
    ui.channelBuffers.push(
      channel,
      data,
      (ByteData? reply) => callback?.call(reply),
    );
  }

  @override
  Future<ByteData?>? send(String channel, ByteData? message) {
    final String? text = _copiedText(channel, message);
    if (text == null) {
      return _inner.send(channel, message);
    }
    if (_copy(text)) {
      return Future<ByteData?>.value(_codec.encodeSuccessEnvelope(true));
    }
    return Future<ByteData?>.value(
      _codec.encodeErrorEnvelope(
        code: kCopyFailCode,
        message: 'The browser refused to copy.',
      ),
    );
  }

  @override
  void setMessageHandler(String channel, MessageHandler? handler) =>
      _inner.setMessageHandler(channel, handler);

  String? _copiedText(String channel, ByteData? message) {
    if (channel != SystemChannels.platform.name ||
        message == null ||
        _secure()) {
      return null;
    }
    final MethodCall call = _codec.decodeMethodCall(message);
    if (call.method != kClipboardSetData) {
      return null;
    }
    final Object? arguments = call.arguments;
    if (arguments is! Map) {
      return null;
    }
    final Object? text = arguments['text'];
    return text is String ? text : null;
  }
}
