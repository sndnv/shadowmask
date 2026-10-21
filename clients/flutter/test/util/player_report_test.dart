import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/player_report.dart';

void main() {
  const Set<String> decoders = <String>{
    'h264',
    'hevc',
    'mpeg4',
    'msmpeg4v3',
    'mpeg2video',
    'aac',
    'truehd',
    'mp3',
    'mp2',
    'pcm_s16le',
  };
  final Set<String> demuxers = demuxerNames(
    'aac,asf,avi,flv,hls,matroska,webm,mov,mp4,m4a,3gp,3g2,mj2,mpeg,mpegts',
  );

  test(
    'the desktop reports every candidate its player decodes, all smooth',
    () {
      final ClientDecoding? report = playerReport(
        decoders: decoders,
        demuxers: demuxers,
      );

      expect(report?.video, const <VideoCodecCap>[
        VideoCodecCap(codec: 'h264', maxBitDepth: 10, smooth: true),
        VideoCodecCap(codec: 'hevc', maxBitDepth: 12, smooth: true),
        VideoCodecCap(codec: 'mpeg4', maxBitDepth: 8, smooth: true),
        VideoCodecCap(codec: 'msmpeg4v3', maxBitDepth: 8, smooth: true),
      ]);
      expect(report?.audio, const <AudioCodecCap>[
        AudioCodecCap(codec: 'aac', maxChannels: 8),
        AudioCodecCap(codec: 'truehd', maxChannels: 8),
        AudioCodecCap(codec: 'mp3', maxChannels: 2),
        AudioCodecCap(codec: 'mp2', maxChannels: 2),
      ]);
      expect(report?.hdr, isNull);
      expect(report?.ceiling, isNull);
    },
  );

  test('the containers are the ones its player opens', () {
    expect(
      playerReport(decoders: decoders, demuxers: demuxers)?.containers,
      <String>[
        'mkv',
        'mp4',
        'ts',
        'avi',
        'mov',
        'm4v',
        'mpg',
        'm2ts',
        'wmv',
        'flv',
      ],
    );
    expect(
      playerReport(
        decoders: decoders,
        demuxers: demuxerNames('mov,mp4,m4a'),
      )?.containers,
      <String>['mkv', 'mp4', 'mov', 'm4v'],
    );
    expect(
      playerReport(
        decoders: decoders,
        demuxers: <String>{},
      )?.toJson().containsKey('containers'),
      isFalse,
    );
  });

  test('a phone takes what its hardware says about a codec it names', () {
    const ClientDecoding hardware = ClientDecoding(
      video: <VideoCodecCap>[
        VideoCodecCap(codec: 'h264', maxBitDepth: 8, smooth: true),
        VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: false),
        VideoCodecCap(codec: 'av1', maxBitDepth: 10, smooth: true),
      ],
      hdr: <String>['hdr10'],
      maxWidth: 3840,
      maxHeight: 2160,
      maxFrameRate: 60,
    );

    final ClientDecoding? report = playerReport(
      decoders: <String>{...decoders, 'vp9'},
      demuxers: demuxers,
      phone: true,
      hardware: hardware,
    );

    expect(report?.video, const <VideoCodecCap>[
      VideoCodecCap(codec: 'h264', maxBitDepth: 8, smooth: true),
      VideoCodecCap(codec: 'hevc', maxBitDepth: 10, smooth: false),
      VideoCodecCap(codec: 'vp9', maxBitDepth: 12, smooth: false),
      VideoCodecCap(codec: 'mpeg4', maxBitDepth: 8, smooth: true),
      VideoCodecCap(codec: 'msmpeg4v3', maxBitDepth: 8, smooth: true),
    ]);
    expect(report?.hdr, <String>['hdr10']);
    expect(report?.ceiling, '3840x2160@60');
    expect(report?.containers, contains('avi'));
  });

  test(
    'a phone whose hardware said nothing leaves the picture to its profile',
    () {
      for (final ClientDecoding? hardware in <ClientDecoding?>[
        null,
        const ClientDecoding(hdr: <String>['hlg']),
      ]) {
        final ClientDecoding? report = playerReport(
          decoders: decoders,
          demuxers: demuxers,
          phone: true,
          hardware: hardware,
        );
        expect(report?.video, isEmpty);
        expect(report?.audio, isNotEmpty);
        expect(report?.toJson().containsKey('video'), isFalse);
      }
    },
  );

  test(
    'a player whose lists could not be read keeps what the hardware said',
    () {
      const ClientDecoding hardware = ClientDecoding(
        video: <VideoCodecCap>[
          VideoCodecCap(codec: 'h264', maxBitDepth: 8, smooth: true),
        ],
        maxWidth: 1920,
        maxHeight: 1080,
      );

      expect(
        playerReport(
          decoders: <String>{},
          demuxers: <String>{},
          phone: true,
          hardware: hardware,
        ),
        same(hardware),
      );
      expect(playerReport(decoders: <String>{}, demuxers: demuxers), isNull);
      expect(
        playerReport(
          decoders: <String>{},
          demuxers: demuxers,
          phone: true,
          hardware: const ClientDecoding(),
        ),
        isNull,
      );
    },
  );

  test('a player that decodes no candidate reports nothing', () {
    expect(
      playerReport(decoders: <String>{'pcm_s16le'}, demuxers: demuxers),
      isNull,
    );
  });

  test('the demuxer list is read as names whatever the spacing', () {
    expect(demuxerNames(' avi, mov,mp4 ,,'), <String>{'avi', 'mov', 'mp4'});
    expect(demuxerNames(''), isEmpty);
  });

  test('every candidate has a table entry', () {
    final Map<String, dynamic> candidates =
        jsonDecode(
              File(
                '../../assets/capabilities/candidates.json',
              ).readAsStringSync(),
            )
            as Map<String, dynamic>;
    expect(
      kPlayerVideo.keys.toSet(),
      (candidates['video'] as List<dynamic>).cast<String>().toSet(),
    );
    expect(
      kPlayerAudio.keys.toSet(),
      (candidates['audio'] as List<dynamic>).cast<String>().toSet(),
    );
    expect(<String>{
      ...kPlayerOwnContainers,
      ...kPlayerDemuxers.keys,
    }, (candidates['containers'] as List<dynamic>).cast<String>().toSet());
  });
}
