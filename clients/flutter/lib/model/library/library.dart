import 'package:freezed_annotation/freezed_annotation.dart';

part 'library.freezed.dart';
part 'library.g.dart';

enum LibraryKind {
  @JsonValue('movie')
  movie,
  @JsonValue('tv')
  tv,
}

enum LibraryOrigin {
  @JsonValue('local')
  local,
  @JsonValue('external')
  external,
}

enum WatcherStrategy {
  @JsonValue('local')
  local,
  @JsonValue('polling')
  polling,
  @JsonValue('scheduled')
  scheduled,
  @JsonValue('manual')
  manual,
}

@freezed
abstract class Library with _$Library {
  const factory Library({
    required String id,
    required String name,
    required LibraryKind kind,
    @Default(LibraryOrigin.local) LibraryOrigin origin,
    @Default(<String>[]) List<String> roots,
    required WatcherStrategy watcher,
    String? scanSchedule,
    @Default(<String>[]) List<String> metadataSources,
    @Default(<String>[]) List<String> sortArticles,
    required String createdAt,
    required String updatedAt,
  }) = _Library;

  factory Library.fromJson(Map<String, dynamic> json) =>
      _$LibraryFromJson(json);
}
