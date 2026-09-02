import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shared_preferences/shared_preferences.dart';

LibraryApi _lib(MockClient mock) =>
    LibraryApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

MockClient _capture(
  void Function(http.Request req) sink, {
  String body = '',
  int status = 200,
}) => MockClient((http.Request req) async {
  sink(req);
  return http.Response(body, status);
});

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('createLibrary posts the library body', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 201),
    );

    await api.createLibrary(<String, dynamic>{'name': 'Movies'});

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries');
    expect(jsonDecode(seen.body), <String, dynamic>{'name': 'Movies'});
  });

  test('updateLibrary PUTs to the encoded id', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 200),
    );

    await api.updateLibrary('l 1', <String, dynamic>{'name': 'x'});

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/libraries/l%201');
  });

  test('triggerScan posts to the scan sub-resource', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.triggerScan('l1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l1/scan');
  });

  test('refreshMetadata posts to the refresh-metadata sub-resource', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.refreshMetadata('l1');

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/libraries/l1/refresh-metadata');
  });

  test('resolveUnmatched wraps the target under a target key', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture((http.Request req) => seen = req, status: 202),
    );

    await api.resolveUnmatched('l1', 'u1', <String, dynamic>{
      'kind': 'existing',
      'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
    });

    expect(seen.url.path, '/api/v1/libraries/l1/unmatched/u1/resolve');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'target': <String, dynamic>{
        'kind': 'existing',
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
      },
    });
  });

  test('duplicates GETs the paged duplicates', () async {
    late http.Request seen;
    final LibraryApi api = _lib(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[],
          'total': 0,
          'offset': 0,
          'limit': 0,
        }),
      ),
    );

    await api.duplicates('l1');

    expect(seen.url.path, '/api/v1/libraries/l1/duplicates');
  });
}
