import 'package:shadowmask/model/catalog/version.dart';

Version? resolvePlayTarget(
  List<Version> available,
  Set<String> inProgressVersionIds,
) =>
    available
        .where((Version v) => inProgressVersionIds.contains(v.id))
        .firstOrNull ??
    available.firstOrNull;

int switchStartMs(
  List<Version> versions,
  String versionId,
  int positionMs, {
  required bool ended,
}) {
  if (ended || positionMs < 0) {
    return 0;
  }
  final int durationMs =
      versions
          .where((Version v) => v.id == versionId)
          .firstOrNull
          ?.durationMs ??
      0;
  return durationMs > 0 && positionMs >= durationMs ? 0 : positionMs;
}
