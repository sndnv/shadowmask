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

const Map<String, int> _playerAudioCodecs = <String, int>{
  'aac': 8,
  'ac3': 6,
  'eac3': 8,
  'dts': 8,
  'truehd': 8,
  'flac': 8,
  'opus': 8,
  'vorbis': 8,
  'mp3': 2,
};

Future<ClientDecoding?> measureDecoding() async {
  final Set<String> decoders = await _playerDecoders();
  final ClientDecoding? measured = Platform.isAndroid || Platform.isIOS
      ? await _fromChannel()
      : ClientDecoding(video: desktopVideo(decoders));
  return withPlayerAudio(measured, decoders);
}

List<VideoCodecCap> desktopVideo(Set<String> decoders) => <VideoCodecCap>[
  for (final MapEntry<String, int> codec in _desktopCodecs.entries)
    if (decoders.contains(codec.key))
      VideoCodecCap(codec: codec.key, maxBitDepth: codec.value, smooth: true),
];

ClientDecoding? withPlayerAudio(
  ClientDecoding? measured,
  Set<String> decoders,
) {
  final ClientDecoding decoding = ClientDecoding(
    video: measured?.video ?? const <VideoCodecCap>[],
    audio: <AudioCodecCap>[
      for (final MapEntry<String, int> codec in _playerAudioCodecs.entries)
        if (decoders.contains(codec.key))
          AudioCodecCap(codec: codec.key, maxChannels: codec.value),
    ],
    hdr: measured?.hdr,
    maxWidth: measured?.maxWidth,
    maxHeight: measured?.maxHeight,
    maxFrameRate: measured?.maxFrameRate,
  );
  return decoding.isEmpty ? null : decoding;
}

Future<ClientDecoding?> _fromChannel() async {
  try {
    final Map<Object?, Object?>? raw = await capabilityChannel
        .invokeMapMethod<Object?, Object?>('decoding');
    if (raw == null) {
      return null;
    }
    return ClientDecoding.fromChannel(raw);
  } catch (_) {
    return null;
  }
}

Future<Set<String>> _playerDecoders() async {
  final Player player = Player();
  try {
    final NativePlayer native = player.platform! as NativePlayer;
    final int count =
        int.tryParse(await native.getProperty('decoder-list/count')) ?? 0;
    final Set<String> found = <String>{};
    for (int i = 0; i < count; i++) {
      found.add(await native.getProperty('decoder-list/$i/codec'));
    }
    return found;
  } catch (_) {
    return <String>{};
  } finally {
    await player.dispose();
  }
}
