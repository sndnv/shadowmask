import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/server_probe.dart';

void main() {
  test('the probe asks the server for its info, unauthenticated', () async {
    final List<Uri> asked = <Uri>[];
    await serverAnswers(
      'http://host:8080',
      httpClient: MockClient((http.Request req) async {
        asked.add(req.url);
        expect(req.headers.containsKey('Authorization'), isFalse);
        return http.Response('', 401);
      }),
    );

    expect(asked.single.toString(), 'http://host:8080/api/v1/server/info');
  });

  test('an unauthorized answer still proves a server is there', () async {
    expect(
      await serverAnswers(
        'http://host',
        httpClient: MockClient(
          (http.Request _) async => http.Response('', 401),
        ),
      ),
      isTrue,
    );
  });

  test(
    'anything else answering is not the server we are looking for',
    () async {
      expect(
        await serverAnswers(
          'http://host',
          httpClient: MockClient(
            (http.Request _) async => http.Response('', 404),
          ),
        ),
        isFalse,
      );
    },
  );

  test('an address that cannot be reached at all is a no', () async {
    expect(
      await serverAnswers(
        'http://host',
        httpClient: MockClient(
          (http.Request _) async => throw http.ClientException('refused'),
        ),
      ),
      isFalse,
    );
  });
}
