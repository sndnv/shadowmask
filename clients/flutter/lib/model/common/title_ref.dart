import 'package:freezed_annotation/freezed_annotation.dart';

part 'title_ref.freezed.dart';
part 'title_ref.g.dart';

enum TitleKind {
  @JsonValue('movie')
  movie,
  @JsonValue('series')
  series,
  @JsonValue('season')
  season,
  @JsonValue('episode')
  episode,
  @JsonValue('person')
  person,
  @JsonValue('collection')
  collection,
}

@freezed
abstract class TitleRef with _$TitleRef {
  const factory TitleRef({required TitleKind type, required String id}) =
      _TitleRef;

  factory TitleRef.fromJson(Map<String, dynamic> json) =>
      _$TitleRefFromJson(json);
}

extension TitleKindApi on TitleKind {
  String get wire => switch (this) {
    TitleKind.movie => 'movie',
    TitleKind.series => 'series',
    TitleKind.season => 'season',
    TitleKind.episode => 'episode',
    TitleKind.person => 'person',
    TitleKind.collection => 'collection',
  };

  bool get isLeaf => this == TitleKind.movie || this == TitleKind.episode;
}

extension TitleRefKey on TitleRef {
  String get key => '${type.wire}:$id';
}
