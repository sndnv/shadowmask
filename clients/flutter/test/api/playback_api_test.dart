import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shared_preferences/shared_preferences.dart';

const String _session =
    '{"session_id":"s1","mode":"direct","manifest_url":"/stream/t/file",'
    '"origin_ms":0,"sequential":false,"heartbeat_interval_s":10}';

Future<Map<String, dynamic>> _startBody({ClientDecoding? decoding}) async {
  late http.Request seen;
  final PlaybackApi playback = PlaybackApi(
    ApiClient(
      baseUrl: 'http://test',
      httpClient: MockClient((http.Request req) async {
        seen = req;
        return http.Response(_session, 201);
      }),
    ),
  );

  await playback.startSession(
    versionId: 'v1',
    platform: 'android',
    decoding: decoding,
  );

  return jsonDecode(seen.body) as Map<String, dynamic>;
}

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('a client that measured nothing sends only its device type', () async {
    final Map<String, dynamic> body = await _startBody();

    expect(body['capabilities'], <String, dynamic>{
      'platform': 'android',
      'profile_version': 1,
    });
  });

  test('a measured report travels with the play request', () async {
    final Map<String, dynamic> body = await _startBody(
      decoding: const ClientDecoding(
        video: <VideoCodecCap>[
          VideoCodecCap(codec: 'av1', maxBitDepth: 10, smooth: false),
        ],
        hdr: <String>['hdr10'],
        maxWidth: 3840,
        maxHeight: 2160,
        maxFrameRate: 30,
      ),
    );

    expect(body['capabilities'], <String, dynamic>{
      'platform': 'android',
      'profile_version': 1,
      'decoding': <String, dynamic>{
        'video': <dynamic>[
          <String, dynamic>{
            'codec': 'av1',
            'max_bit_depth': 10,
            'smooth': false,
          },
        ],
        'hdr': <dynamic>['hdr10'],
        'max_width': 3840,
        'max_height': 2160,
        'max_frame_rate': 30,
      },
    });
  });
}
