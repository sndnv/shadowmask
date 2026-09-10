import 'package:shadowmask/model/session/client_decoding.dart';

enum CodecSupport { auto, hardware, software, unsupported }

enum HdrChoice { auto, allow, deny }

const List<String> kKnownVideoCodecs = <String>['h264', 'hevc', 'vp9', 'av1'];
const List<int> kPictureHeights = <int>[2160, 1440, 1080, 720];
const List<int> kFrameRateCaps = <int>[60, 30];
const List<String> kHdrFormats = <String>['hdr10', 'hdr10plus', 'hlg'];
const int kAddedCodecBitDepth = 10;

class CapabilityOverrides {
  const CapabilityOverrides({
    this.codecs = const <String, CodecSupport>{},
    this.maxHeight,
    this.maxFrameRate,
    this.hdr = HdrChoice.auto,
  });

  factory CapabilityOverrides.fromJson(Map<String, dynamic> raw) {
    T? pick<T extends Enum>(List<T> values, Object? name) {
      for (final T value in values) {
        if (value.name == name) {
          return value;
        }
      }
      return null;
    }

    final Map<String, CodecSupport> codecs = <String, CodecSupport>{};
    final Object? stored = raw['codecs'];
    if (stored is Map<String, dynamic>) {
      for (final MapEntry<String, dynamic> entry in stored.entries) {
        final CodecSupport? support = pick(CodecSupport.values, entry.value);
        if (support != null) {
          codecs[entry.key] = support;
        }
      }
    }
    return CapabilityOverrides(
      codecs: codecs,
      maxHeight: (raw['max_height'] as num?)?.toInt(),
      maxFrameRate: (raw['max_frame_rate'] as num?)?.toInt(),
      hdr: pick(HdrChoice.values, raw['hdr']) ?? HdrChoice.auto,
    );
  }

  final Map<String, CodecSupport> codecs;
  final int? maxHeight;
  final int? maxFrameRate;
  final HdrChoice hdr;

  bool get isEmpty =>
      maxHeight == null &&
      maxFrameRate == null &&
      hdr == HdrChoice.auto &&
      codecs.values.every((CodecSupport s) => s == CodecSupport.auto);

  CodecSupport supportFor(String codec) => codecs[codec] ?? CodecSupport.auto;

  CapabilityOverrides withCodec(String codec, CodecSupport support) =>
      CapabilityOverrides(
        codecs: <String, CodecSupport>{...codecs, codec: support},
        maxHeight: maxHeight,
        maxFrameRate: maxFrameRate,
        hdr: hdr,
      );

  CapabilityOverrides withMaxHeight(int? height) => CapabilityOverrides(
    codecs: codecs,
    maxHeight: height,
    maxFrameRate: maxFrameRate,
    hdr: hdr,
  );

  CapabilityOverrides withMaxFrameRate(int? rate) => CapabilityOverrides(
    codecs: codecs,
    maxHeight: maxHeight,
    maxFrameRate: rate,
    hdr: hdr,
  );

  CapabilityOverrides withHdr(HdrChoice choice) => CapabilityOverrides(
    codecs: codecs,
    maxHeight: maxHeight,
    maxFrameRate: maxFrameRate,
    hdr: choice,
  );

  ClientDecoding? applyTo(ClientDecoding? measured) {
    if (isEmpty) {
      return measured;
    }
    return ClientDecoding(
      video: _video(measured),
      audio: measured?.audio ?? const <AudioCodecCap>[],
      hdr: _hdr(measured),
      maxWidth: measured?.maxWidth,
      maxHeight: maxHeight ?? measured?.maxHeight,
      maxFrameRate: maxFrameRate ?? measured?.maxFrameRate,
    );
  }

  List<VideoCodecCap> _video(ClientDecoding? measured) {
    if (measured == null) {
      return const <VideoCodecCap>[];
    }
    final List<VideoCodecCap> out = <VideoCodecCap>[];
    for (final String codec in kKnownVideoCodecs) {
      final VideoCodecCap? found = _find(measured, codec);
      switch (supportFor(codec)) {
        case CodecSupport.auto:
          if (found != null) {
            out.add(found);
          }
        case CodecSupport.unsupported:
          break;
        case CodecSupport.hardware:
          out.add(_cap(codec, found, smooth: true));
        case CodecSupport.software:
          out.add(_cap(codec, found, smooth: false));
      }
    }
    for (final VideoCodecCap cap in measured.video) {
      if (!kKnownVideoCodecs.contains(cap.codec)) {
        out.add(cap);
      }
    }
    return out.isEmpty ? measured.video : out;
  }

  VideoCodecCap? _find(ClientDecoding measured, String codec) {
    for (final VideoCodecCap cap in measured.video) {
      if (cap.codec == codec) {
        return cap;
      }
    }
    return null;
  }

  VideoCodecCap _cap(
    String codec,
    VideoCodecCap? found, {
    required bool smooth,
  }) => VideoCodecCap(
    codec: codec,
    maxBitDepth: found?.maxBitDepth ?? kAddedCodecBitDepth,
    smooth: smooth,
  );

  List<String>? _hdr(ClientDecoding? measured) {
    switch (hdr) {
      case HdrChoice.auto:
        return measured?.hdr;
      case HdrChoice.allow:
        final List<String>? found = measured?.hdr;
        return found == null || found.isEmpty ? kHdrFormats : found;
      case HdrChoice.deny:
        return const <String>[];
    }
  }

  Map<String, dynamic> toJson() => <String, dynamic>{
    'codecs': <String, String>{
      for (final MapEntry<String, CodecSupport> entry in codecs.entries)
        if (entry.value != CodecSupport.auto) entry.key: entry.value.name,
    },
    if (maxHeight != null) 'max_height': maxHeight,
    if (maxFrameRate != null) 'max_frame_rate': maxFrameRate,
    'hdr': hdr.name,
  };

  @override
  bool operator ==(Object other) =>
      other is CapabilityOverrides &&
      other.maxHeight == maxHeight &&
      other.maxFrameRate == maxFrameRate &&
      other.hdr == hdr &&
      _sameCodecs(other.codecs);

  bool _sameCodecs(Map<String, CodecSupport> other) {
    for (final String codec in <String>{...codecs.keys, ...other.keys}) {
      if ((codecs[codec] ?? CodecSupport.auto) !=
          (other[codec] ?? CodecSupport.auto)) {
        return false;
      }
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(
    maxHeight,
    maxFrameRate,
    hdr,
    Object.hashAllUnordered(<Object>[
      for (final MapEntry<String, CodecSupport> entry in codecs.entries)
        if (entry.value != CodecSupport.auto)
          '${entry.key}:${entry.value.name}',
    ]),
  );
}
