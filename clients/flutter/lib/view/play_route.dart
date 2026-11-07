import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/view/play_target.dart';
import 'package:shadowmask/view/title_link.dart';
import 'package:shadowmask/view/version_order.dart';

Future<Set<String>?> inProgressVersionIds(
  CatalogApi catalog,
  String userId,
) async {
  try {
    return (await catalog.continueFeed(userId)).resumeProgress.keys.toSet();
  } catch (_) {
    return null;
  }
}

Future<String> playRouteFor(
  CatalogApi catalog,
  String userId,
  TitleLink link, {
  String? seriesId,
  Map<String, String?> controls = const <String, String?>{},
}) async {
  final Future<Set<String>?> inProgress = inProgressVersionIds(catalog, userId);
  try {
    final Paged<Version> versions = link.kind == TitleKind.movie
        ? await catalog.movieVersions(link.id)
        : await catalog.episodeVersions(
            link.id,
            series: seriesId,
            season: link.seasonId,
          );
    final List<Version> available = orderedVersions(
      versions.items,
    ).where((Version v) => v.available).toList();
    final Set<String>? ids = await inProgress;
    final Version? target = ids == null && available.length > 1
        ? null
        : resolvePlayTarget(available, ids ?? const <String>{});
    if (target != null) {
      return watchRoute(
        target.id,
        controls: controls,
        queue: link.queueEntryId,
      );
    }
  } catch (_) {}
  return link.kind == TitleKind.movie
      ? movieRoute(link.id)
      : episodeRoute(link.id, series: seriesId);
}
