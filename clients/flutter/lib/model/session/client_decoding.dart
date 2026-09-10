class VideoCodecCap {
  const VideoCodecCap({
    required this.codec,
    required this.maxBitDepth,
    required this.smooth,
  });

  factory VideoCodecCap.fromChannel(Map<Object?, Object?> raw) => VideoCodecCap(
    codec: raw['codec'] as String? ?? '',
    maxBitDepth: (raw['max_bit_depth'] as num?)?.toInt() ?? 8,
    smooth: raw['smooth'] as bool? ?? true,
  );

  final String codec;
  final int maxBitDepth;
  final bool smooth;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'codec': codec,
    'max_bit_depth': maxBitDepth,
    'smooth': smooth,
  };

  @override
  bool operator ==(Object other) =>
      other is VideoCodecCap &&
      other.codec == codec &&
      other.maxBitDepth == maxBitDepth &&
      other.smooth == smooth;

  @override
  int get hashCode => Object.hash(codec, maxBitDepth, smooth);
}

class AudioCodecCap {
  const AudioCodecCap({required this.codec, required this.maxChannels});

  factory AudioCodecCap.fromChannel(Map<Object?, Object?> raw) => AudioCodecCap(
    codec: raw['codec'] as String? ?? '',
    maxChannels: (raw['max_channels'] as num?)?.toInt() ?? 2,
  );

  final String codec;
  final int maxChannels;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'codec': codec,
    'max_channels': maxChannels,
  };

  @override
  bool operator ==(Object other) =>
      other is AudioCodecCap &&
      other.codec == codec &&
      other.maxChannels == maxChannels;

  @override
  int get hashCode => Object.hash(codec, maxChannels);
}

class ClientDecoding {
  const ClientDecoding({
    this.video = const <VideoCodecCap>[],
    this.audio = const <AudioCodecCap>[],
    this.hdr,
    this.maxWidth,
    this.maxHeight,
    this.maxFrameRate,
  });

  factory ClientDecoding.fromChannel(Map<Object?, Object?> raw) {
    List<T> list<T>(String key, T Function(Map<Object?, Object?>) read) =>
        (raw[key] as List<Object?>? ?? <Object?>[])
            .whereType<Map<Object?, Object?>>()
            .map(read)
            .toList();
    final List<String> hdr = (raw['hdr'] as List<Object?>? ?? <Object?>[])
        .whereType<String>()
        .toList();
    return ClientDecoding(
      video: list('video', VideoCodecCap.fromChannel),
      audio: list('audio', AudioCodecCap.fromChannel),
      hdr: hdr.isEmpty ? null : hdr,
      maxWidth: (raw['max_width'] as num?)?.toInt(),
      maxHeight: (raw['max_height'] as num?)?.toInt(),
      maxFrameRate: (raw['max_frame_rate'] as num?)?.toInt(),
    );
  }

  final List<VideoCodecCap> video;
  final List<AudioCodecCap> audio;
  final List<String>? hdr;
  final int? maxWidth;
  final int? maxHeight;
  final int? maxFrameRate;

  bool get isEmpty => video.isEmpty && audio.isEmpty;

  String? get ceiling {
    if (maxWidth == null || maxHeight == null) {
      return null;
    }
    final String size = '${maxWidth}x$maxHeight';
    return maxFrameRate == null ? size : '$size@$maxFrameRate';
  }

  Map<String, dynamic> toJson() => <String, dynamic>{
    if (video.isNotEmpty)
      'video': video.map((VideoCodecCap c) => c.toJson()).toList(),
    if (audio.isNotEmpty)
      'audio': audio.map((AudioCodecCap c) => c.toJson()).toList(),
    if (hdr != null) 'hdr': hdr,
    if (maxWidth != null) 'max_width': maxWidth,
    if (maxHeight != null) 'max_height': maxHeight,
    if (maxFrameRate != null) 'max_frame_rate': maxFrameRate,
  };
}
