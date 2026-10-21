import 'dart:io';

import 'package:flutter/services.dart';
import 'package:media_kit/media_kit.dart';

import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/util/player_report.dart';

const MethodChannel capabilityChannel = MethodChannel(
  'io.github.sndnv.shadowmask/capabilities',
);

Future<ClientDecoding?> measureDecoding() async {
  final ({Set<String> decoders, Set<String> demuxers}) player =
      await _playerLists();
  final bool phone = Platform.isAndroid || Platform.isIOS;
  return playerReport(
    decoders: player.decoders,
    demuxers: player.demuxers,
    phone: phone,
    hardware: phone ? await _fromChannel() : null,
  );
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

Future<({Set<String> decoders, Set<String> demuxers})> _playerLists() async {
  final Player player = Player();
  try {
    final NativePlayer native = player.platform! as NativePlayer;
    final int count =
        int.tryParse(await native.getProperty('decoder-list/count')) ?? 0;
    final Set<String> decoders = <String>{};
    for (int i = 0; i < count; i++) {
      decoders.add(await native.getProperty('decoder-list/$i/codec'));
    }
    return (
      decoders: decoders,
      demuxers: demuxerNames(await native.getProperty('demuxer-lavf-list')),
    );
  } catch (_) {
    return (decoders: <String>{}, demuxers: <String>{});
  } finally {
    await player.dispose();
  }
}
