import 'package:freezed_annotation/freezed_annotation.dart';

part 'artwork.freezed.dart';
part 'artwork.g.dart';

@freezed
abstract class ImageSet with _$ImageSet {
  const ImageSet._();

  const factory ImageSet({
    required String base,
    @Default(<int>[]) List<int> widths,
  }) = _ImageSet;

  factory ImageSet.fromJson(Map<String, dynamic> json) =>
      _$ImageSetFromJson(json);

  String? url(String baseUrl, int width) {
    if (widths.isEmpty) {
      return null;
    }
    int largest = widths.first;
    int? smallestThatFits;
    for (final int candidate in widths) {
      if (candidate > largest) {
        largest = candidate;
      }
      if (candidate >= width &&
          (smallestThatFits == null || candidate < smallestThatFits)) {
        smallestThatFits = candidate;
      }
    }
    return '$baseUrl$base/${smallestThatFits ?? largest}';
  }
}

@freezed
abstract class Artwork with _$Artwork {
  const Artwork._();

  const factory Artwork({
    @Default(<ImageSet>[]) List<ImageSet> posters,
    @Default(<ImageSet>[]) List<ImageSet> backdrops,
  }) = _Artwork;

  factory Artwork.fromJson(Map<String, dynamic> json) =>
      _$ArtworkFromJson(json);

  String? posterUrl(String baseUrl, int width) =>
      posters.isEmpty ? null : posters.first.url(baseUrl, width);
  String? backdropUrl(String baseUrl, int width) =>
      backdrops.isEmpty ? null : backdrops.first.url(baseUrl, width);

  bool get hasMultiplePosters => posters.length > 1;

  List<String> posterMosaic(String baseUrl, int width, {int max = 4}) => posters
      .take(max)
      .map((ImageSet s) => s.url(baseUrl, width))
      .whereType<String>()
      .toList();
}
