import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/user_admin_api.dart';
import 'package:shared_preferences/shared_preferences.dart';

UserAdminApi _users(MockClient mock) =>
    UserAdminApi(ApiClient(baseUrl: 'http://test', httpClient: mock));

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

  test('users GETs the paged users collection', () async {
    late http.Request seen;
    final UserAdminApi api = _users(
      _capture(
        (http.Request req) => seen = req,
        body: jsonEncode(<String, dynamic>{
          'items': <dynamic>[
            <String, dynamic>{
              'id': 'u1',
              'username': 'pat',
              'role': 'admin',
              'created_at': 'a',
              'updated_at': 'b',
            },
          ],
          'total': 1,
          'offset': 0,
          'limit': 20,
        }),
      ),
    );

    final page = await api.users();

    expect(seen.method, 'GET');
    expect(seen.url.path, '/api/v1/users');
    expect(page.items.single.username, 'pat');
  });

  test('createUser posts the user body', () async {
    late http.Request seen;
    final UserAdminApi api = _users(
      _capture((http.Request req) => seen = req, status: 201),
    );

    await api.createUser(<String, dynamic>{
      'username': 'sam',
      'password': 'secret',
      'role': 'user',
    });

    expect(seen.method, 'POST');
    expect(seen.url.path, '/api/v1/users');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'username': 'sam',
      'password': 'secret',
      'role': 'user',
    });
  });

  test('deleteUser DELETEs the encoded id', () async {
    late http.Request seen;
    final UserAdminApi api = _users(
      _capture((http.Request req) => seen = req, status: 204),
    );

    await api.deleteUser('u 1');

    expect(seen.method, 'DELETE');
    expect(seen.url.path, '/api/v1/users/u%201');
  });

  test('libraryAccess maps the response down to library ids', () async {
    final UserAdminApi api = _users(
      _capture(
        (_) {},
        body: jsonEncode(<dynamic>[
          <String, dynamic>{'user_id': 'u1', 'library_id': 'l1'},
          <String, dynamic>{'user_id': 'u1', 'library_id': 'l2'},
        ]),
      ),
    );

    final ids = await api.libraryAccess('u1');

    expect(ids, <String>['l1', 'l2']);
  });

  test('setLibraryAccess PUTs the libraries list', () async {
    late http.Request seen;
    final UserAdminApi api = _users(
      _capture((http.Request req) => seen = req, status: 204),
    );

    await api.setLibraryAccess('u1', <String>['l1', 'l2']);

    expect(seen.method, 'PUT');
    expect(seen.url.path, '/api/v1/users/u1/libraries');
    expect(jsonDecode(seen.body), <String, dynamic>{
      'libraries': <String>['l1', 'l2'],
    });
  });
}
