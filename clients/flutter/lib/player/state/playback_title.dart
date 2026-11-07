import 'package:freezed_annotation/freezed_annotation.dart';

import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/view/title_link.dart';

part 'playback_title.freezed.dart';

@freezed
abstract class PlaybackTitle with _$PlaybackTitle {
  const factory PlaybackTitle({
    VersionDetail? version,
    String? name,
    TitleLink? previous,
    TitleLink? next,
  }) = _PlaybackTitle;
}
