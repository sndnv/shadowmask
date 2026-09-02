import 'package:shadowmask/model/catalog/version.dart';

Version? resolvePlayTarget(
  List<Version> available,
  Set<String> inProgressVersionIds,
) {
  if (available.isEmpty) {
    return null;
  }
  if (available.length == 1) {
    return available.first;
  }
  final List<Version> resumable = available
      .where((Version v) => inProgressVersionIds.contains(v.id))
      .toList();
  return resumable.length == 1 ? resumable.first : null;
}
