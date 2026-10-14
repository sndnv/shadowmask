import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:shadowmask/util/lan_client_native.dart';

void main() {
  test('the LAN client reaches a listener on this machine', () async {
    final HttpServer server = await HttpServer.bind(
      InternetAddress.loopbackIPv4,
      0,
    );
    addTearDown(() => server.close(force: true));
    server.listen((HttpRequest req) async {
      req.response.statusCode = HttpStatus.accepted;
      await req.response.close();
    });
    final http.Client client = lanClient(const Duration(seconds: 2));
    addTearDown(client.close);

    final http.Response res = await client.post(
      Uri(scheme: 'http', host: '127.0.0.1', port: server.port, path: '/play'),
    );

    expect(res.statusCode, HttpStatus.accepted);
  });
}
