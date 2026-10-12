import 'package:freezed_annotation/freezed_annotation.dart';

part 'library_prep.freezed.dart';
part 'library_prep.g.dart';

@freezed
abstract class LibraryPrep with _$LibraryPrep {
  const factory LibraryPrep({
    required String id,
    required String name,
    @Default(0) int outdated,
    @Default(0) int unprepared,
    @Default(<String>[]) List<String> preparedWith,
  }) = _LibraryPrep;

  factory LibraryPrep.fromJson(Map<String, dynamic> json) =>
      _$LibraryPrepFromJson(json);
}
