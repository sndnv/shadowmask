class MpvTrack {
  const MpvTrack({
    required this.id,
    required this.type,
    required this.ffIndex,
    required this.external,
  });

  final int id;
  final String type;
  final int? ffIndex;
  final bool external;
}

MpvTrack? mpvTrackOf({
  required String id,
  required String type,
  required String ffIndex,
  required String external,
}) {
  final int? parsed = int.tryParse(id);
  if (parsed == null || type.isEmpty) {
    return null;
  }
  return MpvTrack(
    id: parsed,
    type: type,
    ffIndex: int.tryParse(ffIndex),
    external: external == 'yes',
  );
}

int? mpvTrackId(List<MpvTrack> tracks, String type, int ffIndex) {
  for (final MpvTrack track in tracks) {
    if (track.type == type && !track.external && track.ffIndex == ffIndex) {
      return track.id;
    }
  }
  return null;
}
