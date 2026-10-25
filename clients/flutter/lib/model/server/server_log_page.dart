import 'package:freezed_annotation/freezed_annotation.dart';

part 'server_log_page.freezed.dart';
part 'server_log_page.g.dart';

@freezed
abstract class ServerLogPage with _$ServerLogPage {
  const factory ServerLogPage({
    @Default(<String>[]) List<String> entries,
    String? before,
    String? after,
    @Default(false) bool more,
  }) = _ServerLogPage;

  factory ServerLogPage.fromJson(Map<String, dynamic> json) =>
      _$ServerLogPageFromJson(json);
}
