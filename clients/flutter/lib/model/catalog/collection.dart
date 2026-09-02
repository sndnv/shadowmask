import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/model/catalog/movie.dart';

part 'collection.freezed.dart';
part 'collection.g.dart';

@freezed
abstract class Collection with _$Collection {
  const factory Collection({
    required String id,
    required String name,
    String? overview,
    @Default(<String>[]) List<String> movies,
    @Default(<Movie>[]) List<Movie> items,
    String? addedAt,
    String? updatedAt,
    Artwork? artwork,
  }) = _Collection;

  factory Collection.fromJson(Map<String, dynamic> json) =>
      _$CollectionFromJson(json);
}
