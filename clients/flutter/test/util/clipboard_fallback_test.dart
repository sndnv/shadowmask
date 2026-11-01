import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/clipboard_fallback.dart';
import 'package:shadowmask/util/clipboard_route_native.dart';

class _Engine extends BinaryMessenger {
  final List<String> sent = <String>[];
  final Map<String, MessageHandler?> handlers = <String, MessageHandler?>{};

  @override
  Future<void> handlePlatformMessage(
    String channel,
    ByteData? data,
    PlatformMessageResponseCallback? callback,
  ) async {}

  @override
  Future<ByteData?>? send(String channel, ByteData? message) {
    sent.add(channel);
    return Future<ByteData?>.value(
      const JSONMethodCodec().encodeSuccessEnvelope('engine'),
    );
  }

  @override
  void setMessageHandler(String channel, MessageHandler? handler) {
    handlers[channel] = handler;
  }
}

class _Page {
  _Page({required this.secure, this.copies = true});

  final bool secure;
  final bool copies;
  final List<String> copied = <String>[];

  bool copy(String text) {
    copied.add(text);
    return copies;
  }
}

MethodChannel _platform(BinaryMessenger messenger) => MethodChannel(
  SystemChannels.platform.name,
  const JSONMethodCodec(),
  messenger,
);

Future<Object?> _setData(BinaryMessenger messenger, String text) => _platform(
  messenger,
).invokeMethod<Object?>(kClipboardSetData, <String, Object?>{'text': text});

ClipboardFallbackMessenger _messenger(_Engine engine, _Page page) =>
    ClipboardFallbackMessenger(
      engine,
      secure: () => page.secure,
      copy: page.copy,
    );

void main() {
  test(
    'on a page without a secure context a copy goes through the fallback',
    () async {
      final _Engine engine = _Engine();
      final _Page page = _Page(secure: false);

      final Object? reply = await _setData(_messenger(engine, page), 'report');

      expect(reply, isTrue);
      expect(page.copied, <String>['report']);
      expect(engine.sent, isEmpty);
    },
  );

  test('a fallback the browser refuses answers copy_fail', () async {
    final _Engine engine = _Engine();
    final _Page page = _Page(secure: false, copies: false);

    await expectLater(
      _setData(_messenger(engine, page), 'report'),
      throwsA(
        isA<PlatformException>().having(
          (PlatformException e) => e.code,
          'code',
          kCopyFailCode,
        ),
      ),
    );
    expect(page.copied, <String>['report']);
    expect(engine.sent, isEmpty);
  });

  test('on a secure page a copy goes to the engine', () async {
    final _Engine engine = _Engine();
    final _Page page = _Page(secure: true);

    final Object? reply = await _setData(_messenger(engine, page), 'report');

    expect(reply, 'engine');
    expect(page.copied, isEmpty);
    expect(engine.sent, <String>[SystemChannels.platform.name]);
  });

  test('other platform calls and other channels go to the engine', () async {
    final _Engine engine = _Engine();
    final _Page page = _Page(secure: false);
    final ClipboardFallbackMessenger messenger = _messenger(engine, page);

    await _platform(messenger).invokeMethod<Object?>('Clipboard.getData');
    await messenger.send('flutter/lifecycle', null);
    await messenger.send(SystemChannels.platform.name, null);

    expect(page.copied, isEmpty);
    expect(engine.sent, <String>[
      SystemChannels.platform.name,
      'flutter/lifecycle',
      SystemChannels.platform.name,
    ]);
  });

  test('a copy without text goes to the engine', () async {
    final _Engine engine = _Engine();
    final _Page page = _Page(secure: false);

    await _platform(
      _messenger(engine, page),
    ).invokeMethod<Object?>(kClipboardSetData, <String, Object?>{'text': null});

    expect(page.copied, isEmpty);
    expect(engine.sent, <String>[SystemChannels.platform.name]);
  });

  test('handlers are set on the engine', () {
    final _Engine engine = _Engine();
    Future<ByteData?> handler(ByteData? _) async => null;

    _messenger(engine, _Page(secure: false)).setMessageHandler('x', handler);

    expect(engine.handlers['x'], same(handler));
  });

  test('the apps keep the engine as it is', () {
    final _Engine engine = _Engine();

    expect(withClipboardFallback(engine), same(engine));
  });
}
