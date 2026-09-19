import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/auth/api_token.dart';
import 'package:shadowmask/model/auth/device.dart';
import 'package:shadowmask/model/auth/link_code.dart';

class AccountApi {
  AccountApi(this._api);

  final ApiClient _api;

  Future<void> changePassword(
    String userId, {
    String? current,
    required String newPassword,
  }) => _api.sendVoid(
    'PUT',
    '/api/v1/users/${_enc(userId)}/password',
    body: <String, dynamic>{
      if (current != null && current.isNotEmpty) 'current_password': current,
      'new_password': newPassword,
    },
  );

  Future<void> signOutEverywhere(String userId) =>
      _api.sendVoid('DELETE', '/api/v1/users/${_enc(userId)}/sessions');

  Future<void> signOutThisDevice(String? userId) async {
    final String deviceId = await _api.linkedDeviceId().catchError(
      (Object _) => '',
    );
    if (userId != null && userId.isNotEmpty && deviceId.isNotEmpty) {
      try {
        await revokeDevice(userId, deviceId);
      } catch (_) {}
    }
    await _api.logout();
  }

  Future<List<Device>> devices(String userId) => _api.getJsonArray(
    '/api/v1/users/${_enc(userId)}/devices',
    Device.fromJson,
  );

  Future<void> revokeDevice(String userId, String deviceId) => _api.sendVoid(
    'DELETE',
    '/api/v1/users/${_enc(userId)}/devices/${_enc(deviceId)}',
  );

  Future<List<ApiToken>> tokens(String userId) => _api.getJsonArray(
    '/api/v1/users/${_enc(userId)}/tokens',
    ApiToken.fromJson,
  );

  Future<void> revokeToken(String userId, String tokenId) => _api.sendVoid(
    'DELETE',
    '/api/v1/users/${_enc(userId)}/tokens/${_enc(tokenId)}',
  );

  Future<List<LinkCode>> linkCodes(String userId) => _api.getJsonArray(
    '/api/v1/users/${_enc(userId)}/link-codes',
    LinkCode.fromJson,
  );

  Future<LinkCode> createLinkCode(String userId, {int? ttlSecs}) =>
      _api.postJson('/api/v1/auth/link/create', <String, dynamic>{
        'user_id': userId,
        'ttl_secs': ?ttlSecs,
      }, LinkCode.fromJson);

  Future<void> revokeLinkCode(String userId, String code) => _api.sendVoid(
    'DELETE',
    '/api/v1/users/${_enc(userId)}/link-codes/${_enc(code)}',
  );

  String _enc(String s) => Uri.encodeComponent(s);
}
