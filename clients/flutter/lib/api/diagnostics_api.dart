import 'package:shadowmask/api/api_client.dart';

class DiagnosticsApi {
  DiagnosticsApi(this._api);

  final ApiClient _api;

  Future<String> send({
    required String app,
    required String platform,
    required String appVersion,
    required String? note,
    required Map<String, Object?> payload,
  }) async => _api.postJson('/api/v1/diagnostics/reports', <String, dynamic>{
    'player_id': await _api.playerId(),
    'app': app,
    'platform': platform,
    'app_version': appVersion,
    'note': note,
    'payload': payload,
  }, (Map<String, dynamic> json) => json['id'] as String);
}
