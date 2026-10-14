import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:shadowmask/util/command_listener_native.dart';
import 'package:shadowmask/util/command_listener_types.dart';

void main() {
  late CommandListener listener;
  late List<String> tokens;
  late int answer;

  Uri at(String path) =>
      Uri(scheme: 'http', host: '127.0.0.1', port: listener.port, path: path);

  setUp(() async {
    tokens = <String>[];
    answer = 202;
    listener = await bindCommandListener(0, (String token) async {
      tokens.add(token);
      return answer;
    }, address: InternetAddress.loopbackIPv4);
  });

  tearDown(() => listener.close());

  test('binds a free port when asked for none', () {
    expect(listener.port, greaterThan(0));
  });

  test(
    'a token posted to play reaches the handler and gets its answer',
    () async {
      final http.Response res = await http.post(
        at('/play'),
        headers: <String, String>{'Content-Type': 'application/json'},
        body: jsonEncode(<String, String>{'token': 'tok'}),
      );

      expect(res.statusCode, 202);
      expect(tokens, <String>['tok']);
    },
  );

  for (final int status in <int>[401, 403, 502]) {
    test('the handler answer [$status] goes back as is', () async {
      answer = status;

      final http.Response res = await http.post(
        at('/play'),
        body: jsonEncode(<String, String>{'token': 'tok'}),
      );

      expect(res.statusCode, status);
    });
  }

  test('another path is not found', () async {
    final http.Response res = await http.post(at('/other'), body: '{}');

    expect(res.statusCode, HttpStatus.notFound);
    expect(tokens, isEmpty);
  });

  test('another method is not allowed', () async {
    final http.Response res = await http.get(at('/play'));

    expect(res.statusCode, HttpStatus.methodNotAllowed);
  });

  for (final String body in <String>[
    '',
    'not json',
    '[]',
    '{"token": 7}',
    '{"token": ""}',
    '{"other": "tok"}',
  ]) {
    test('a body without a token [$body] is a bad request', () async {
      final http.Response res = await http.post(at('/play'), body: body);

      expect(res.statusCode, HttpStatus.badRequest);
      expect(tokens, isEmpty);
    });
  }

  test('bytes that are not text are a bad request', () async {
    final http.Response res = await http.post(
      at('/play'),
      body: <int>[0xff, 0xfe, 0xfd],
    );

    expect(res.statusCode, HttpStatus.badRequest);
  });

  test('a body over the limit is refused unread', () async {
    final http.Response res = await http.post(
      at('/play'),
      body: jsonEncode(<String, String>{
        'token': 'x' * (kCommandBodyLimit + 1),
      }),
    );

    expect(res.statusCode, HttpStatus.requestEntityTooLarge);
    expect(tokens, isEmpty);
  });

  test('a body that stalls is answered with a timeout', () async {
    final CommandListener slow = await bindCommandListener(
      0,
      (String token) async {
        tokens.add(token);
        return 202;
      },
      address: InternetAddress.loopbackIPv4,
      readTimeout: const Duration(milliseconds: 100),
    );
    addTearDown(slow.close);
    final Socket socket = await Socket.connect(
      InternetAddress.loopbackIPv4,
      slow.port,
    );
    addTearDown(socket.destroy);

    socket.write(
      'POST /play HTTP/1.1\r\nHost: x\r\nContent-Length: 100\r\n\r\n{"tok',
    );
    final String reply = await utf8.decoder.bind(socket).first;

    expect(reply, startsWith('HTTP/1.1 408'));
    expect(tokens, isEmpty);
  });

  test('a client that leaves mid-body leaves the listener serving', () async {
    final Socket socket = await Socket.connect(
      InternetAddress.loopbackIPv4,
      listener.port,
    );
    socket.write(
      'POST /play HTTP/1.1\r\nHost: x\r\nContent-Length: 100\r\n\r\n{"tok',
    );
    await socket.flush();
    socket.destroy();
    await Future<void>.delayed(const Duration(milliseconds: 50));

    final http.Response res = await http.post(
      at('/play'),
      body: jsonEncode(<String, String>{'token': 'tok'}),
    );

    expect(res.statusCode, 202);
    expect(tokens, <String>['tok']);
  });

  test('a closed listener takes no more connections', () async {
    final Uri play = at('/play');
    await listener.close();

    await expectLater(
      http.post(play, body: '{"token":"tok"}'),
      throwsA(isA<http.ClientException>()),
    );
  });

  test('the device lists only IPv4 interfaces', () async {
    final List<InterfaceAddress> found = await interfaceAddresses();

    expect(found.every((InterfaceAddress a) => a.octets.length == 4), isTrue);
  });
}
