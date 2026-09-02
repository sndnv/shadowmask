import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/language_dropdown.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/util/subtitle_labels.dart';
import 'package:shadowmask/view/failure_reason.dart';

Future<bool> showUpscaleDialog(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  VideoTrack? video,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) =>
          _UpscaleDialog(admin: admin, versionId: versionId, video: video),
    ) ??
    false;

Future<bool> showTranscribeDialog(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required int audioTrackIndex,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) => _TranscribeDialog(
        admin: admin,
        versionId: versionId,
        audioTrackIndex: audioTrackIndex,
      ),
    ) ??
    false;

Future<bool> showTranslateDialog(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required SubtitleFile source,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) =>
          _TranslateDialog(admin: admin, versionId: versionId, source: source),
    ) ??
    false;

Future<bool> showCombineDialog(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required List<SubtitleFile> subs,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) =>
          _CombineDialog(admin: admin, versionId: versionId, subs: subs),
    ) ??
    false;

const List<(int, String)> _upscaleHeights = <(int, String)>[
  (480, Strings.height480),
  (720, Strings.height720),
  (1080, Strings.height1080),
  (1440, Strings.height1440),
  (2160, Strings.height2160),
];

List<(int, String)> upscaleOptions(VideoTrack? video) => <(int, String)>[
  for (final (int height, String label) in _upscaleHeights)
    if (height > (video?.height ?? 0)) (height, label),
];

class _UpscaleDialog extends StatefulWidget {
  const _UpscaleDialog({
    required this.admin,
    required this.versionId,
    required this.video,
  });

  final AdminApi admin;
  final String versionId;
  final VideoTrack? video;

  @override
  State<_UpscaleDialog> createState() => _UpscaleDialogState();
}

class _UpscaleDialogState extends State<_UpscaleDialog> {
  late final List<(int, String)> _options = upscaleOptions(widget.video);
  late int _height = _options.isEmpty ? 0 : _options.first.$1;
  bool _submitting = false;

  Future<void> _submit() async {
    setState(() => _submitting = true);
    try {
      await widget.admin.upscale(widget.versionId, targetHeight: _height);
      if (mounted) {
        Toasts.of(context).success(Strings.toastQueuedTrackJobs);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
        setState(() => _submitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final VideoTrack? v = widget.video;
    return FormDialog(
      title: Strings.upscale,
      submitting: _submitting,
      onSubmit: _options.isEmpty ? null : _submit,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        spacing: Space.s3,
        children: <Widget>[
          if (v != null && v.height > 0)
            Text('${Strings.currentResolution}: ${v.width}×${v.height}'),
          if (_options.isEmpty)
            const StatusText(Strings.alreadyMaxResolution)
          else
            LabelledDropdown<int>(
              label: Strings.fieldTargetHeight,
              help: Strings.targetHeightHelp,
              value: _height,
              items: _options,
              onChanged: (int h) => setState(() => _height = h),
            ),
        ],
      ),
    );
  }
}

class _TranscribeDialog extends StatefulWidget {
  const _TranscribeDialog({
    required this.admin,
    required this.versionId,
    required this.audioTrackIndex,
  });

  final AdminApi admin;
  final String versionId;
  final int audioTrackIndex;

  @override
  State<_TranscribeDialog> createState() => _TranscribeDialogState();
}

class _TranscribeDialogState extends State<_TranscribeDialog> {
  String _sourceLanguage = '';
  bool _submitting = false;

  Future<void> _submit() async {
    setState(() => _submitting = true);
    try {
      await widget.admin.transcribe(
        widget.versionId,
        audioTrackIndex: widget.audioTrackIndex,
        sourceLanguage: _sourceLanguage,
      );
      if (mounted) {
        Toasts.of(context).success(Strings.toastQueuedTrackJobs);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
        setState(() => _submitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.transcribe,
      submitting: _submitting,
      onSubmit: _submit,
      child: LanguageDropdown(
        label: Strings.fieldSourceLanguage,
        help: Strings.sourceLanguageHelp,
        value: _sourceLanguage,
        emptyLabel: Strings.optionAutoDetect,
        onChanged: (String v) => setState(() => _sourceLanguage = v),
      ),
    );
  }
}

class _TranslateDialog extends StatefulWidget {
  const _TranslateDialog({
    required this.admin,
    required this.versionId,
    required this.source,
  });

  final AdminApi admin;
  final String versionId;
  final SubtitleFile source;

  @override
  State<_TranslateDialog> createState() => _TranslateDialogState();
}

class _TranslateDialogState extends State<_TranslateDialog> {
  String _targetLanguage = '';
  bool _submitting = false;
  String? _targetError;

  Future<void> _submit() async {
    if (_targetLanguage.isEmpty) {
      setState(() => _targetError = Strings.requiredTargetLanguage);
      return;
    }
    setState(() => _submitting = true);
    try {
      await widget.admin.translate(
        widget.versionId,
        sourceSubtitleId: widget.source.id,
        targetLanguage: _targetLanguage,
      );
      if (mounted) {
        Toasts.of(context).success(Strings.toastQueuedTrackJobs);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
        setState(() => _submitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.translate,
      submitting: _submitting,
      onSubmit: _submit,
      child: LanguageDropdown(
        label: Strings.fieldTargetLanguage,
        help: Strings.targetLanguageHelp,
        value: _targetLanguage,
        emptyLabel: Strings.optionNoLanguage,
        error: _targetError,
        onChanged: (String v) => setState(() {
          _targetLanguage = v;
          _targetError = null;
        }),
      ),
    );
  }
}

class _CombineDialog extends StatefulWidget {
  const _CombineDialog({
    required this.admin,
    required this.versionId,
    required this.subs,
  });

  final AdminApi admin;
  final String versionId;
  final List<SubtitleFile> subs;

  @override
  State<_CombineDialog> createState() => _CombineDialogState();
}

class _CombineDialogState extends State<_CombineDialog> {
  late String _top = widget.subs.first.id;
  late String _bottom = widget.subs.length > 1
      ? widget.subs[1].id
      : widget.subs.first.id;
  bool _submitting = false;
  String? _bottomError;

  Future<void> _submit() async {
    if (_top == _bottom) {
      setState(() => _bottomError = Strings.requiredDistinctSubtitles);
      return;
    }
    setState(() => _submitting = true);
    try {
      await widget.admin.combineSubtitles(
        widget.versionId,
        topSubtitleId: _top,
        bottomSubtitleId: _bottom,
      );
      if (mounted) {
        Toasts.of(context).success(Strings.toastQueuedTrackJobs);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
        setState(() => _submitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.combineSubtitles,
      submitting: _submitting,
      onSubmit: _submit,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        spacing: Space.s3,
        children: <Widget>[
          LabelledDropdown<String>(
            label: Strings.fieldTopSubtitle,
            help: Strings.combineSubtitlesHelp,
            value: _top,
            items: _subtitleOptions(widget.subs),
            onChanged: (String v) => setState(() {
              _top = v;
              _bottomError = null;
            }),
          ),
          LabelledDropdown<String>(
            label: Strings.fieldBottomSubtitle,
            value: _bottom,
            items: _subtitleOptions(widget.subs),
            error: _bottomError,
            onChanged: (String v) => setState(() {
              _bottom = v;
              _bottomError = null;
            }),
          ),
        ],
      ),
    );
  }
}

List<(String, String)> _subtitleOptions(List<SubtitleFile> subs) =>
    <(String, String)>[
      for (final SubtitleFile s in subs) (s.id, subtitleFileLabel(s)),
    ];
