import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/admin/subtitle_dialogs.dart';
import 'package:shadowmask/components/admin/version_job_dialogs.dart';
import 'package:shadowmask/components/version_job_lines.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/version_job_status.dart';

Future<void> showAddSubtitlesDialog(
  BuildContext context, {
  required AdminApi admin,
  required ValueListenable<VersionDetail?> version,
  required ValueListenable<List<VersionJob>> jobs,
  required VoidCallback onWork,
  int? audioTrack,
  String? watchingFileId,
}) => showDialog<void>(
  context: context,
  builder: (BuildContext _) => AddSubtitlesDialog(
    admin: admin,
    version: version,
    jobs: jobs,
    onWork: onWork,
    audioTrack: audioTrack,
    watchingFileId: watchingFileId,
  ),
);

class AddSubtitlesDialog extends StatelessWidget {
  const AddSubtitlesDialog({
    super.key,
    required this.admin,
    required this.version,
    required this.jobs,
    required this.onWork,
    this.audioTrack,
    this.watchingFileId,
  });

  final AdminApi admin;
  final ValueListenable<VersionDetail?> version;
  final ValueListenable<List<VersionJob>> jobs;
  final VoidCallback onWork;
  final int? audioTrack;
  final String? watchingFileId;

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.addSubtitlesHeading,
      child: ValueListenableBuilder<VersionDetail?>(
        valueListenable: version,
        builder: (BuildContext context, VersionDetail? v, Widget? _) =>
            v == null ? const SizedBox.shrink() : _body(context, v),
      ),
    );
  }

  Widget _body(BuildContext context, VersionDetail v) {
    final List<SubtitleFile> files = v.subtitleFiles;
    final SubtitleFile? watching = files
        .where((SubtitleFile f) => f.id == watchingFileId)
        .firstOrNull;
    final int? track = v.audio.any((AudioTrack a) => a.index == audioTrack)
        ? audioTrack
        : v.audio.firstOrNull?.index;
    final Widget search = _action(
      Icons.search,
      Strings.searchSubtitles,
      () => showSubtitleSearch(
        context,
        admin: admin,
        versionId: v.id,
        existing: files,
      ),
    );
    final Widget transcribe = _action(
      Icons.record_voice_over_outlined,
      Strings.transcribe,
      track == null
          ? null
          : () => showTranscribeDialog(
              context,
              admin: admin,
              versionId: v.id,
              audioTrackIndex: track,
              tracks: v.audio,
              queuedToast: Strings.toastQueued,
            ),
    );
    final Widget translate = _action(
      Icons.translate,
      Strings.translate,
      files.isEmpty
          ? null
          : () => showTranslateDialog(
              context,
              admin: admin,
              versionId: v.id,
              source: watching ?? files.first,
              sources: files,
              queuedToast: Strings.toastQueued,
            ),
    );
    final Widget combine = _action(
      Icons.merge_outlined,
      Strings.combineSubtitles,
      files.length < 2
          ? null
          : () => showCombineDialog(
              context,
              admin: admin,
              versionId: v.id,
              subs: files,
              queuedToast: Strings.toastQueued,
            ),
    );
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(child: search),
            Expanded(child: combine),
          ],
        ),
        Row(
          children: <Widget>[
            Expanded(child: transcribe),
            Expanded(child: translate),
          ],
        ),
        ValueListenableBuilder<List<VersionJob>>(
          valueListenable: jobs,
          builder: (BuildContext context, List<VersionJob> rows, Widget? _) =>
              subtitleWork(rows).isEmpty
              ? const SizedBox.shrink()
              : Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Divider(color: context.tokens.border, height: Space.s5),
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            Strings.jobsHeading,
                            style: TextStyle(
                              color: context.tokens.muted,
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                        const FieldHelp(
                          title: Strings.jobsHeading,
                          body: Strings.subtitleJobsHelp,
                        ),
                      ],
                    ),
                    const SizedBox(height: Space.s1),
                    VersionJobLines(jobs: rows, detail: v),
                  ],
                ),
        ),
      ],
    );
  }

  Widget _action(IconData icon, String label, Future<bool> Function()? open) =>
      Align(
        alignment: Alignment.centerLeft,
        child: TextButton.icon(
          onPressed: open == null
              ? null
              : () async {
                  if (await open()) {
                    onWork();
                  }
                },
          icon: Icon(icon, size: 18),
          label: Text(label),
        ),
      );
}
