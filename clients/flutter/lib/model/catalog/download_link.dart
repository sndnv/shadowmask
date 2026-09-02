import 'package:freezed_annotation/freezed_annotation.dart';

part 'download_link.freezed.dart';
part 'download_link.g.dart';

@freezed
abstract class DownloadLink with _$DownloadLink {
  const factory DownloadLink({
    required String url,
    required String filename,
    @Default(0) int sizeBytes,
    String? expiresAt,
  }) = _DownloadLink;

  factory DownloadLink.fromJson(Map<String, dynamic> json) =>
      _$DownloadLinkFromJson(json);
}
