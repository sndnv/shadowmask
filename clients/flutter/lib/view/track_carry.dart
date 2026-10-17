import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/subtitle_choice.dart';

String? _clean(String? language) =>
    (language != null && language.isNotEmpty) ? language : null;

String? audioLanguageOf(VersionDetail version, int? index) {
  if (index == null) {
    return null;
  }
  for (final AudioTrack track in version.audio) {
    if (track.index == index) {
      return _clean(track.language);
    }
  }
  return null;
}

String? subtitleLanguageOf(
  VersionDetail version,
  SubtitleSelection? selection,
) {
  if (selection == null) {
    return null;
  }
  if (selection.kind == SubtitleKind.embedded) {
    for (final SubtitleTrack track in version.subtitles) {
      if (track.index == selection.index) {
        return _clean(track.language);
      }
    }
    return null;
  }
  for (final SubtitleFile file in version.subtitleFiles) {
    if (file.id == selection.id) {
      return _clean(file.language);
    }
  }
  return null;
}

PlaybackControls carryTracks(PlaybackControls controls, VersionDetail version) {
  final String? audio =
      audioLanguageOf(version, controls.audioTrack) ?? controls.audioLanguage;
  final bool off = controls.subtitle is SubtitleOff;
  final String? subtitle = off
      ? null
      : subtitleLanguageOf(version, controls.subtitle.selection) ??
            controls.subtitleLanguage;
  return PlaybackControls(
    audioLanguage: audio,
    subtitleLanguage: subtitle,
    subtitle: off ? const SubtitleChoice.off() : const SubtitleChoice.auto(),
  );
}
