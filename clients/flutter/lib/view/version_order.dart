import 'package:shadowmask/model/catalog/version.dart';

List<Version> orderedVersions(List<Version> versions) {
  final List<Version> ordered = List<Version>.of(versions);
  ordered.sort((Version a, Version b) {
    final int byQuality = b.quality.index.compareTo(a.quality.index);
    if (byQuality != 0) {
      return byQuality;
    }
    final int bySize = b.sizeBytes.compareTo(a.sizeBytes);
    return bySize != 0 ? bySize : a.id.compareTo(b.id);
  });
  return ordered;
}

String versionNumberLabel(int index) => '${index + 1}';
