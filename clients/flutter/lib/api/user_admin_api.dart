import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/page.dart';

class UserAdminApi {
  UserAdminApi(this._api);

  final ApiClient _api;

  Future<Paged<AccountProfile>> users({int offset = 0, int? limit}) =>
      _api.getPage(
        withQuery('/api/v1/users', _pageParams(offset, limit)),
        AccountProfile.fromJson,
      );

  Future<void> createUser(Map<String, dynamic> body) =>
      _api.sendVoid('POST', '/api/v1/users', body: body);

  Future<void> deleteUser(String id) =>
      _api.sendVoid('DELETE', '/api/v1/users/${_enc(id)}');

  Future<void> setActive(String id, bool active) => _api.sendVoid(
    'PUT',
    '/api/v1/users/${_enc(id)}/active',
    body: <String, dynamic>{'active': active},
  );

  Future<List<String>> libraryAccess(String id) => _api.getJsonArray(
    '/api/v1/users/${_enc(id)}/libraries',
    (Map<String, dynamic> json) => json['library_id'] as String,
  );

  Future<void> setLibraryAccess(String id, List<String> libraries) =>
      _api.sendVoid(
        'PUT',
        '/api/v1/users/${_enc(id)}/libraries',
        body: <String, dynamic>{'libraries': libraries},
      );

  Map<String, String?> _pageParams(int offset, int? limit) => <String, String?>{
    if (offset > 0) 'offset': offset.toString(),
    if (limit != null) 'limit': limit.toString(),
  };

  String _enc(String s) => Uri.encodeComponent(s);
}
