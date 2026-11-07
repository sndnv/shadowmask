import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/playlist.dart';
import 'package:shadowmask/model/playlist/playlist_detail.dart';
import 'package:shadowmask/model/playlist/queue_entry.dart';

class PlaylistApi {
  PlaylistApi(this._api);

  final ApiClient _api;

  String get imageBase => _api.baseUrl;

  Future<List<Playlist>> playlists(String userId) =>
      _api.getJsonArray(_playlists(userId), Playlist.fromJson);

  Future<Playlist> create(String userId, String name) => _api.postJson(
    _playlists(userId),
    <String, dynamic>{'name': name},
    Playlist.fromJson,
  );

  Future<PlaylistDetail> playlist(String userId, String id) =>
      _api.getJson(_playlist(userId, id), PlaylistDetail.fromJson);

  Future<void> rename(String userId, String id, String name) => _api.sendVoid(
    'PUT',
    _playlist(userId, id),
    body: <String, dynamic>{'name': name},
  );

  Future<void> delete(String userId, String id) =>
      _api.sendVoid('DELETE', _playlist(userId, id));

  Future<int> add(String userId, String id, TitleRef title) => _api.postJson(
    '${_playlist(userId, id)}/items',
    <String, dynamic>{'type': title.type.wire, 'id': title.id},
    _added,
  );

  Future<void> remove(String userId, String id, String titleId) => _api
      .sendVoid('DELETE', '${_playlist(userId, id)}/items/${_enc(titleId)}');

  Future<void> move(String userId, String id, String titleId, int position) =>
      _api.sendVoid(
        'PUT',
        '${_playlist(userId, id)}/items/${_enc(titleId)}/position',
        body: <String, dynamic>{'position': position},
      );

  Future<List<QueueEntry>> queue(String userId) =>
      _api.getJsonArray(_queue(userId), QueueEntry.fromJson);

  Future<int> enqueue(
    String userId,
    String playlistId, {
    bool shuffle = false,
  }) => _api.postJson(_queue(userId), <String, dynamic>{
    'playlist_id': playlistId,
    'shuffle': shuffle,
  }, _added);

  Future<void> clearQueue(String userId) =>
      _api.sendVoid('DELETE', _queue(userId));

  Future<void> dequeue(String userId, String entryId) =>
      _api.sendVoid('DELETE', '${_queue(userId)}/items/${_enc(entryId)}');

  static int _added(Map<String, dynamic> json) => json['added'] as int? ?? 0;

  static String _enc(String s) => Uri.encodeComponent(s);

  static String _playlists(String userId) =>
      '/api/v1/users/${_enc(userId)}/playlists';

  static String _playlist(String userId, String id) =>
      '${_playlists(userId)}/${_enc(id)}';

  static String _queue(String userId) => '/api/v1/users/${_enc(userId)}/queue';
}
