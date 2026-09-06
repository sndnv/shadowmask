import 'dart:typed_data';

import 'package:shadowmask/model/catalog/download_link.dart';
import 'package:shadowmask/model/session/negotiation.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/model/session/playback_session.dart';
import 'package:shadowmask/model/session/resume_position.dart';
import 'package:shadowmask/model/server/server_info.dart';
import 'package:shadowmask/api/api_client.dart';

class PlaybackApi {
  PlaybackApi(this._api);

  final ApiClient _api;

  String get baseUrl => _api.baseUrl;

  Future<ServerInfo> serverInfo() =>
      _api.getJson('/api/v1/server/info', ServerInfo.fromJson);

  Future<ResumePosition> resume(String userId, String versionId) =>
      _api.getJson(
        '/api/v1/users/${_enc(userId)}/progress/${_enc(versionId)}',
        ResumePosition.fromJson,
      );

  Future<void> clearProgress(String userId, String versionId) => _api.sendVoid(
    'DELETE',
    '/api/v1/users/${_enc(userId)}/progress/${_enc(versionId)}',
  );

  Future<DownloadLink> downloadLink(String versionId) => _api.postJson(
    '/api/v1/versions/${_enc(versionId)}/download',
    const <String, dynamic>{},
    DownloadLink.fromJson,
  );

  Future<PlaybackSession> startSession({
    required String versionId,
    int startPositionMs = 0,
    int profileVersion = 1,
    String platform = 'generic',
    PlaybackControls controls = const PlaybackControls(),
  }) => _api.postJson('/api/v1/sessions', <String, dynamic>{
    'version_id': versionId,
    'start_position_ms': startPositionMs,
    'capabilities': <String, dynamic>{
      'platform': platform,
      'profile_version': profileVersion,
    },
    ...controls.toStartBody(),
  }, PlaybackSession.fromJson);

  Future<void> progress(String sessionId, int positionMs, String state) async {
    try {
      await _api.sendVoid(
        'POST',
        '/api/v1/sessions/${_enc(sessionId)}/progress',
        body: <String, dynamic>{'position_ms': positionMs, 'state': state},
      );
    } catch (_) {}
  }

  Future<void> endSession(String sessionId) async {
    try {
      await _api.sendVoid('DELETE', '/api/v1/sessions/${_enc(sessionId)}');
    } catch (_) {}
  }

  Future<Negotiation> seek(String sessionId, int positionMs) => _api.postJson(
    '/api/v1/sessions/${_enc(sessionId)}/seek',
    <String, dynamic>{'position_ms': positionMs},
    Negotiation.fromJson,
  );

  Future<Negotiation> update(String sessionId, PlaybackControls controls) =>
      _api.postJson(
        '/api/v1/sessions/${_enc(sessionId)}/update',
        controls.toUpdateBody(),
        Negotiation.fromJson,
      );

  Future<Uint8List> trickplaySheet(String versionId, int sheet) =>
      _api.bytes('/api/v1/trickplay/${_enc(versionId)}/$sheet');

  String _enc(String s) => Uri.encodeComponent(s);
}
