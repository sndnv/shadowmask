import 'dart:io';

import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';

import 'package:shadowmask/model/session/client_decoding.dart';

const MethodChannel capabilityChannel = MethodChannel(
  'io.github.sndnv.shadowmask/capabilities',
);

const Map<String, int> _desktopCodecs = <String, int>{
  'h264': 10,
  'hevc': 12,
  'vp9': 12,
  'av1': 12,
};

Future<ClientDecoding?> measureDecoding() async {
  if (Platform.isAndroid || Platform.isIOS) {
    return _fromChannel();
  }
  return _fromPlayer();
}

Future<ClientDecoding?> _fromChannel() async {
  try {
    final Map<Object?, Object?>? raw = await capabilityChannel
        .invokeMapMethod<Object?, Object?>('decoding');
    if (raw == null) {
      return null;
    }
    final ClientDecoding decoding = ClientDecoding.fromChannel(raw);
    return decoding.isEmpty ? null : decoding;
  } catch (_) {
    return null;
  }
}

Future<ClientDecoding?> _fromPlayer() async {
  final Player player = Player();
  try {
    final NativePlayer native = player.platform! as NativePlayer;
    final int count =
        int.tryParse(await native.getProperty('decoder-list/count')) ?? 0;
    final Set<String> found = <String>{};
    for (int i = 0; i < count; i++) {
      found.add(await native.getProperty('decoder-list/$i/codec'));
    }
    final List<VideoCodecCap> video = <VideoCodecCap>[
      for (final MapEntry<String, int> codec in _desktopCodecs.entries)
        if (found.contains(codec.key))
          VideoCodecCap(
            codec: codec.key,
            maxBitDepth: codec.value,
            smooth: true,
          ),
    ];
    return video.isEmpty ? null : ClientDecoding(video: video);
  } catch (_) {
    return null;
  } finally {
    await player.dispose();
  }
}
