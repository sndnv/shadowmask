import 'package:shadowmask/model/catalog/version_detail.dart';

const double kMinVideoAspect = 0.5;
const double kMaxVideoAspect = 3.0;

double? videoAspect(VersionDetail? version) {
  final List<VideoTrack> tracks = version?.video ?? const <VideoTrack>[];
  if (tracks.isEmpty) {
    return null;
  }
  final VideoTrack track = tracks.first;
  if (track.width <= 0 || track.height <= 0) {
    return null;
  }
  final double ratio = track.width / track.height;
  if (ratio < kMinVideoAspect || ratio > kMaxVideoAspect) {
    return null;
  }
  return ratio;
}
