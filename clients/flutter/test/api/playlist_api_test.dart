import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shared_preferences/shared_preferences.dart';

typedef _Sent = ({String method, String path, String? body});

Map<String, dynamic> _playlist(String id) => <String, dynamic>{
  'id': id,
  'name': 'Weekend',
  'created_at': '2026-11-06T10:00:00Z',
  'updated_at': '2026-11-06T10:00:00Z',
};

_Sent _sent(String method, String path, [Map<String, dynamic>? body]) =>
    (method: method, path: path, body: body == null ? null : jsonEncode(body));

PlaylistApi _api(List<_Sent> sent) => PlaylistApi(
  ApiClient(
    baseUrl: 'http://test',
    httpClient: MockClient((http.Request req) async {
      sent.add((
        method: req.method,
        path: req.url.path,
        body: req.body.isEmpty ? null : req.body,
      ));
      final String path = req.url.path;
      if (req.method == 'GET' && path.endsWith('/playlists')) {
        return http.Response(jsonEncode(<dynamic>[_playlist('pl1')]), 200);
      }
      if (req.method == 'POST' && path.endsWith('/playlists')) {
        return http.Response(jsonEncode(_playlist('pl9')), 201);
      }
      if (req.method == 'GET' && path.endsWith('/playlists/pl1')) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            ..._playlist('pl1'),
            'items': <dynamic>[],
          }),
          200,
        );
      }
      if (req.method == 'GET' && path.endsWith('/queue')) {
        return http.Response(
          jsonEncode(<dynamic>[
            <String, dynamic>{
              'id': '3',
              'card': <String, dynamic>{
                'type': 'movie',
                'id': 'm1',
                'title': 'One',
              },
            },
          ]),
          200,
        );
      }
      if (req.method == 'POST') {
        return http.Response(jsonEncode(<String, dynamic>{'added': 2}), 200);
      }
      return http.Response('', 204);
    }),
  ),
);

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test(
    'playlists are read, made, renamed and deleted under the user',
    () async {
      final List<_Sent> sent = <_Sent>[];
      final PlaylistApi api = _api(sent);

      expect((await api.playlists('u 1')).single.id, 'pl1');
      expect((await api.create('u 1', 'Late')).id, 'pl9');
      expect((await api.playlist('u 1', 'pl1')).items, isEmpty);
      await api.rename('u 1', 'pl1', 'Sunday');
      await api.delete('u 1', 'pl1');

      expect(sent, <_Sent>[
        _sent('GET', '/api/v1/users/u%201/playlists'),
        _sent('POST', '/api/v1/users/u%201/playlists', <String, dynamic>{
          'name': 'Late',
        }),
        _sent('GET', '/api/v1/users/u%201/playlists/pl1'),
        _sent('PUT', '/api/v1/users/u%201/playlists/pl1', <String, dynamic>{
          'name': 'Sunday',
        }),
        _sent('DELETE', '/api/v1/users/u%201/playlists/pl1'),
      ]);
    },
  );

  test('items are added by kind, removed and moved by title id', () async {
    final List<_Sent> sent = <_Sent>[];
    final PlaylistApi api = _api(sent);

    final int added = await api.add(
      'u1',
      'pl1',
      const TitleRef(type: TitleKind.season, id: 'se1'),
    );
    await api.remove('u1', 'pl1', 'e1');
    await api.move('u1', 'pl1', 'e1', 4);

    expect(added, 2);
    expect(sent, <_Sent>[
      _sent('POST', '/api/v1/users/u1/playlists/pl1/items', <String, dynamic>{
        'type': 'season',
        'id': 'se1',
      }),
      _sent('DELETE', '/api/v1/users/u1/playlists/pl1/items/e1'),
      _sent(
        'PUT',
        '/api/v1/users/u1/playlists/pl1/items/e1/position',
        <String, dynamic>{'position': 4},
      ),
    ]);
  });

  test(
    'the queue is read, filled in order or shuffled, emptied and trimmed',
    () async {
      final List<_Sent> sent = <_Sent>[];
      final PlaylistApi api = _api(sent);

      expect((await api.queue('u1')).single.id, '3');
      expect(await api.enqueue('u1', 'pl1'), 2);
      await api.enqueue('u1', 'pl1', shuffle: true);
      await api.clearQueue('u1');
      await api.dequeue('u1', '3');

      expect(sent.skip(1), <_Sent>[
        _sent('POST', '/api/v1/users/u1/queue', <String, dynamic>{
          'playlist_id': 'pl1',
          'shuffle': false,
        }),
        _sent('POST', '/api/v1/users/u1/queue', <String, dynamic>{
          'playlist_id': 'pl1',
          'shuffle': true,
        }),
        _sent('DELETE', '/api/v1/users/u1/queue'),
        _sent('DELETE', '/api/v1/users/u1/queue/items/3'),
      ]);
    },
  );
}
