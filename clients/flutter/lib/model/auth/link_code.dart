import 'package:freezed_annotation/freezed_annotation.dart';

part 'link_code.freezed.dart';
part 'link_code.g.dart';

@freezed
abstract class LinkCode with _$LinkCode {
  const factory LinkCode({required String code, required String expiresAt}) =
      _LinkCode;

  factory LinkCode.fromJson(Map<String, dynamic> json) =>
      _$LinkCodeFromJson(json);
}
