import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';

String? relinkBlockForVersions(List<Version> versions) {
  final int stranded = versions.where((Version v) => !v.available).length;
  return stranded == 0 ? null : Strings.relinkBlockedMovie(stranded);
}

String? relinkBlockForSeries(int episodesTotal, int episodesPlayable) {
  if (episodesTotal == 0) {
    return Strings.relinkBlockedEmptySeries;
  }
  final int stranded = episodesTotal - episodesPlayable;
  return stranded == 0 ? null : Strings.relinkBlockedSeries(stranded);
}

String? deleteBlockForVersions(int versions) =>
    versions == 0 ? null : Strings.blockedByVersions(versions);

String? deleteBlockForEpisodes(int episodes) =>
    episodes == 0 ? null : Strings.blockedByEpisodes(episodes);

String? deleteBlockForSeasons(int seasons) =>
    seasons == 0 ? null : Strings.blockedBySeasons(seasons);
