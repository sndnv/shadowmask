import 'package:shadowmask/model/catalog/version.dart';

Version? resolvePlayTarget(
  List<Version> available,
  Set<String> inProgressVersionIds,
) =>
    available
        .where((Version v) => inProgressVersionIds.contains(v.id))
        .firstOrNull ??
    available.firstOrNull;
