import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/action_segments.dart';
import 'package:shadowmask/components/admin/subtitle_dialogs.dart';
import 'package:shadowmask/components/admin/version_job_dialogs.dart';
import 'package:shadowmask/components/outline_pill.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/version_job_lines.dart';
import 'package:shadowmask/components/version_label.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/download_link.dart';
import 'package:shadowmask/model/job/version_job.dart';
import 'package:shadowmask/model/session/resume_position.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_menu.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/absolute_url.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/languages.dart';
import 'package:shadowmask/util/subtitle_labels.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/version_job_status.dart';
import 'package:shadowmask/view/version_jobs_watch.dart';
import 'package:shadowmask/view/version_order.dart';

const double kVersionActionsWidth = 210;

const double _kIconButtonWidth = 48;
const double _kCcPillWidth = 56;
const double _kMinLabelWidth = 140;

const double kVersionRowSideBySide =
    _kMinLabelWidth +
    _kIconButtonWidth +
    _kCcPillWidth +
    Space.s3 +
    kVersionActionsWidth +
    Space.s2 +
    _kIconButtonWidth;

int distinctSubtitleLanguages(VersionDetail d) {
  final Set<String> langs = <String>{};
  for (final SubtitleTrack s in d.subtitles) {
    if (s.language != null) {
      langs.add(s.language!.toLowerCase());
    }
  }
  for (final SubtitleFile f in d.subtitleFiles) {
    if (f.language != null) {
      langs.add(f.language!.toLowerCase());
    }
  }
  return langs.length;
}

List<String> subtitleLanguages(VersionDetail d) {
  final Set<String> langs = <String>{};
  for (final SubtitleTrack s in d.subtitles) {
    if (s.language != null) {
      langs.add(languageLabel(s.language!));
    }
  }
  for (final SubtitleFile f in d.subtitleFiles) {
    if (f.language != null) {
      langs.add(languageLabel(f.language!));
    }
  }
  final List<String> sorted = langs.toList()..sort();
  return sorted;
}

class VersionPicker extends StatefulWidget {
  const VersionPicker({
    super.key,
    required this.catalog,
    required this.playback,
    required this.userId,
    required this.versions,
    this.onProgressCleared,
    this.showHeading = true,
    this.admin,
    this.adminPages = false,
  });

  final CatalogApi catalog;
  final PlaybackApi playback;
  final String userId;
  final List<Version> versions;
  final VoidCallback? onProgressCleared;
  final bool showHeading;
  final AdminApi? admin;
  final bool adminPages;

  @override
  State<VersionPicker> createState() => _VersionPickerState();
}

class _VersionPickerState extends State<VersionPicker> {
  late final List<Version> _ordered = orderedVersions(widget.versions);
  List<VersionDetail?>? _details;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _reload();
  }

  Future<List<VersionDetail?>> _reload() async {
    final int generation = ++_generation;
    final List<VersionDetail?> details = await _load();
    if (mounted && generation == _generation) {
      setState(() => _details = details);
    }
    return details;
  }

  Future<List<VersionDetail?>> _load() => Future.wait(
    _ordered.map((Version v) async {
      try {
        return await widget.catalog.version(v.id);
      } catch (_) {
        return null;
      }
    }),
  );

  @override
  Widget build(BuildContext context) {
    final int count = widget.versions.length;
    final List<VersionDetail?>? details = _details;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (widget.showHeading) ...<Widget>[
          Text(
            Strings.countLabel(Strings.versionsHeading, count),
            style: Theme.of(context).textTheme.headlineMedium,
          ),
          const SizedBox(height: Space.s3),
        ],
        if (count == 0)
          Text(
            Strings.noVersionsAvailable,
            style: TextStyle(color: context.tokens.muted),
          )
        else
          Container(
            clipBehavior: Clip.antiAlias,
            decoration: BoxDecoration(
              color: context.tokens.surface,
              borderRadius: const BorderRadius.all(Radii.md),
              border: Border.all(color: context.tokens.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                for (int i = 0; i < _ordered.length; i++)
                  _VersionRow(
                    version: _ordered[i],
                    number: versionNumberLabel(i),
                    detail: details != null && i < details.length
                        ? details[i]
                        : null,
                    playback: widget.playback,
                    userId: widget.userId,
                    showTopBorder: i > 0,
                    onDismissed: widget.onProgressCleared,
                    admin: widget.admin,
                    adminPages: widget.adminPages,
                    onChanged: _reload,
                  ),
              ],
            ),
          ),
        if (count > 0) ...<Widget>[
          const SizedBox(height: Space.s2),
          Text(
            Strings.downloadNoSubtitles,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
          ),
        ],
      ],
    );
  }
}

class _VersionRow extends StatefulWidget {
  const _VersionRow({
    required this.version,
    required this.number,
    required this.detail,
    required this.playback,
    required this.userId,
    required this.showTopBorder,
    required this.onDismissed,
    required this.admin,
    required this.adminPages,
    required this.onChanged,
  });

  final Version version;
  final String number;
  final VersionDetail? detail;
  final PlaybackApi playback;
  final String userId;
  final bool showTopBorder;
  final VoidCallback? onDismissed;
  final AdminApi? admin;
  final bool adminPages;
  final Future<List<VersionDetail?>> Function() onChanged;

  @override
  State<_VersionRow> createState() => _VersionRowState();
}

class _VersionRowState extends State<_VersionRow> {
  bool _open = false;
  int _positionMs = 0;
  bool _busyDismiss = false;
  bool _busyDownload = false;
  VersionJobsWatch? _jobs;

  void _toggleOpen() => setState(() => _open = !_open);

  Future<void> _download() async {
    setState(() => _busyDownload = true);
    try {
      final DownloadLink link = await widget.playback.downloadLink(
        widget.version.id,
      );
      final bool started = await startDownload(
        absoluteUrl(widget.playback.baseUrl, link.url),
        link.filename,
      );
      if (mounted) {
        started
            ? Toasts.of(context).success(Strings.toastDownloadStarted)
            : Toasts.of(context).error(Strings.errorDownload);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorDownload, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busyDownload = false);
      }
    }
  }

  Widget _downloadButton(Tokens t, Version v) => IconButton(
    onPressed: _busyDownload ? null : _download,
    tooltip: Strings.downloadVersion,
    icon: Icon(Icons.download_outlined, color: t.muted),
  );

  Future<void> _run(Future<bool> action) async {
    if (await action && mounted) {
      widget.onChanged();
      _jobs?.refresh();
    }
  }

  List<Widget> _adminActions(
    BuildContext context,
    Tokens t,
    Version v,
    VersionDetail? d,
  ) {
    final AdminApi? admin = widget.admin;
    return <Widget>[
      if (widget.adminPages)
        IconButton(
          onPressed: () => Navigator.of(context).pushNamed(versionRoute(v.id)),
          tooltip: Strings.openVersion,
          icon: Icon(Icons.settings_outlined, color: t.muted),
        ),
      if (admin != null) _subtitlesMenu(context, t, admin, v, d),
    ];
  }

  double get _adminActionsWidth =>
      _kIconButtonWidth *
      ((widget.adminPages ? 1 : 0) + (widget.admin == null ? 0 : 1));

  Widget _subtitlesMenu(
    BuildContext context,
    Tokens t,
    AdminApi admin,
    Version v,
    VersionDetail? d,
  ) {
    final MenuController controller = MenuController();
    final List<AudioTrack> audio = d?.audio ?? const <AudioTrack>[];
    final List<SubtitleFile> files = d?.subtitleFiles ?? const <SubtitleFile>[];
    void transcribe(AudioTrack track) => _run(
      showTranscribeDialog(
        context,
        admin: admin,
        versionId: v.id,
        audioTrackIndex: track.index,
        queuedToast: Strings.toastQueued,
      ),
    );
    return MenuAnchor(
      controller: controller,
      alignmentOffset: kMenuOffset,
      style: appMenuStyle(t),
      menuChildren: <Widget>[
        MenuItemButton(
          leadingIcon: const Icon(Icons.search),
          onPressed: d == null
              ? null
              : () => _run(
                  showSubtitleSearch(
                    context,
                    admin: admin,
                    versionId: v.id,
                    existing: files,
                  ),
                ),
          child: const Text(Strings.searchSubtitles),
        ),
        if (audio.length > 1)
          SubmenuButton(
            leadingIcon: const Icon(Icons.record_voice_over_outlined),
            menuChildren: <Widget>[
              for (final AudioTrack track in audio)
                MenuItemButton(
                  onPressed: () => transcribe(track),
                  child: Text(audioTrackLabel(track)),
                ),
            ],
            child: const Text(Strings.transcribe),
          )
        else
          MenuItemButton(
            leadingIcon: const Icon(Icons.record_voice_over_outlined),
            onPressed: audio.isEmpty ? null : () => transcribe(audio.first),
            child: const Text(Strings.transcribe),
          ),
        SubmenuButton(
          leadingIcon: const Icon(Icons.translate),
          menuChildren: <Widget>[
            for (final SubtitleFile file in files)
              MenuItemButton(
                onPressed: () => _run(
                  showTranslateDialog(
                    context,
                    admin: admin,
                    versionId: v.id,
                    source: file,
                    queuedToast: Strings.toastQueued,
                  ),
                ),
                child: Text(subtitleFileLabel(file)),
              ),
          ],
          child: const Text(Strings.translate),
        ),
        MenuItemButton(
          leadingIcon: const Icon(Icons.merge_outlined),
          onPressed: files.length < 2
              ? null
              : () => _run(
                  showCombineDialog(
                    context,
                    admin: admin,
                    versionId: v.id,
                    subs: files,
                    queuedToast: Strings.toastQueued,
                  ),
                ),
          child: const Text(Strings.combineSubtitles),
        ),
      ],
      child: IconButton(
        tooltip: Strings.subtitlesHeading,
        onPressed: () =>
            controller.isOpen ? controller.close() : controller.open(),
        icon: Icon(Icons.closed_caption_outlined, color: t.muted),
      ),
    );
  }

  @override
  void initState() {
    super.initState();
    if (widget.version.available) {
      _loadResume();
    }
    final AdminApi? admin = widget.admin;
    if (admin != null) {
      _jobs = VersionJobsWatch(
        fetch: () => admin.versionJobs(widget.version.id),
        onFinished: _reread,
      )..refresh();
    }
  }

  @override
  void dispose() {
    _jobs?.dispose();
    super.dispose();
  }

  Future<bool> _reread() async => (await widget.onChanged()).any(
    (VersionDetail? d) => d?.id == widget.version.id,
  );

  Future<void> _loadResume() async {
    try {
      final ResumePosition r = await widget.playback.resume(
        widget.userId,
        widget.version.id,
      );
      if (mounted) {
        setState(() => _positionMs = r.positionMs);
      }
    } catch (_) {}
  }

  int get _percent {
    final int dur = widget.version.durationMs;
    if (dur <= 0 || _positionMs <= 0) {
      return 0;
    }
    return ((_positionMs / dur) * 100).clamp(0, 100).round();
  }

  Future<void> _dismiss() async {
    setState(() => _busyDismiss = true);
    try {
      await widget.playback.clearProgress(widget.userId, widget.version.id);
      if (!mounted) {
        return;
      }
      setState(() => _positionMs = 0);
      Toasts.of(context).success(Strings.toastResumeDismissed);
      widget.onDismissed?.call();
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    } finally {
      if (mounted) {
        setState(() => _busyDismiss = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Version v = widget.version;
    final VersionDetail? d = widget.detail;
    final bool available = v.available;
    return DecoratedBox(
      decoration: BoxDecoration(
        border: widget.showTopBorder
            ? Border(top: BorderSide(color: t.border))
            : null,
      ),
      child: LayoutBuilder(
        builder: (BuildContext context, BoxConstraints constraints) {
          final double sideBySide = kVersionRowSideBySide + _adminActionsWidth;
          final bool narrow = constraints.maxWidth < sideBySide;
          final VersionJobsWatch? jobs = _jobs;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              if (narrow)
                _narrowHeader(context, t, v, d, available)
              else
                _wideHeader(context, t, v, d, available),
              if (_open) _detailBody(context, t, v, d, available, narrow),
              if (jobs != null) _jobLines(jobs, d),
            ],
          );
        },
      ),
    );
  }

  Widget _jobLines(VersionJobsWatch jobs, VersionDetail? d) =>
      ValueListenableBuilder<List<VersionJob>>(
        valueListenable: jobs,
        builder: (BuildContext context, List<VersionJob> rows, Widget? _) =>
            subtitleWork(rows).isEmpty
            ? const SizedBox.shrink()
            : Padding(
                padding: const EdgeInsets.fromLTRB(
                  Space.s3,
                  0,
                  Space.s3,
                  Space.s2,
                ),
                child: VersionJobLines(jobs: rows, detail: d),
              ),
      );

  Widget _label(Tokens t, Version v, bool available) => Text.rich(
    TextSpan(
      style: monoStyle.copyWith(color: available ? t.text : t.muted),
      children: versionLabelSpans(t, v, widget.number, available: available),
    ),
    overflow: TextOverflow.ellipsis,
  );

  Widget _narrowHeader(
    BuildContext context,
    Tokens t,
    Version v,
    VersionDetail? d,
    bool available,
  ) {
    return InkWell(
      onTap: _toggleOpen,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: Space.s3, vertical: 9),
        child: Row(
          children: <Widget>[
            Expanded(child: _label(t, v, available)),
            if (!available) ...<Widget>[
              const SizedBox(width: Space.s2),
              Text(
                Strings.unavailable,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.warn),
              ),
            ],
            const SizedBox(width: Space.s2),
            Icon(_open ? Icons.expand_less : Icons.expand_more, color: t.muted),
          ],
        ),
      ),
    );
  }

  Widget _wideHeader(
    BuildContext context,
    Tokens t,
    Version v,
    VersionDetail? d,
    bool available,
  ) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: Space.s3, vertical: 9),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Row(
              children: <Widget>[
                Flexible(
                  child: InkWell(
                    onTap: _toggleOpen,
                    child: _label(t, v, available),
                  ),
                ),
                if (available) _downloadButton(t, v),
                ..._adminActions(context, t, v, d),
              ],
            ),
          ),
          if (d != null) _ccPill(distinctSubtitleLanguages(d)),
          const SizedBox(width: Space.s3),
          SizedBox(
            width: kVersionActionsWidth,
            child: Align(
              alignment: Alignment.centerRight,
              child: _trailing(context, t, v, available),
            ),
          ),
          const SizedBox(width: Space.s2),
          IconButton(
            onPressed: _toggleOpen,
            tooltip: Strings.fullVersionDetails,
            icon: Icon(
              _open ? Icons.expand_less : Icons.expand_more,
              color: t.muted,
            ),
          ),
        ],
      ),
    );
  }

  Widget _trailing(BuildContext context, Tokens t, Version v, bool available) {
    if (!available) {
      return Text(
        Strings.unavailable,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(color: t.warn),
      );
    }
    if (_positionMs > 0) {
      return Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Tooltip(
            message: Strings.resume(_percent),
            child: FilledButton.icon(
              onPressed: _busyDismiss
                  ? null
                  : () => Navigator.of(context).pushNamed(watchRoute(v.id)),
              style: segmented(kActionButtonStyle, kSegmentLeading),
              icon: const Icon(Icons.play_arrow, size: 20),
              label: const Text(Strings.resumeAction),
            ),
          ),
          DismissSegment(onPressed: _busyDismiss ? null : _dismiss),
        ],
      );
    }
    return OutlinedButton(
      onPressed: () => Navigator.of(context).pushNamed(watchRoute(v.id)),
      child: const Text(Strings.play),
    );
  }

  Widget _ccPill(int count) => OutlinePill(
    icon: Icons.closed_caption_outlined,
    text: count > 0 ? '$count' : Strings.ccNone,
    active: count > 0,
  );

  Widget _detailBody(
    BuildContext context,
    Tokens t,
    Version v,
    VersionDetail? d,
    bool available,
    bool narrow,
  ) {
    final List<String> languages = d == null
        ? const <String>[]
        : subtitleLanguages(d);
    return Padding(
      padding: const EdgeInsets.fromLTRB(Space.s3, 0, Space.s3, Space.s3),
      child: Container(
        padding: const EdgeInsets.only(top: 10),
        decoration: BoxDecoration(
          border: Border(top: BorderSide(color: t.border)),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (narrow &&
                (available ||
                    widget.admin != null ||
                    widget.adminPages)) ...<Widget>[
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: <Widget>[
                  if (available)
                    Flexible(child: _trailing(context, t, v, available))
                  else
                    const Spacer(),
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      if (available) _downloadButton(t, v),
                      ..._adminActions(context, t, v, d),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: Space.s3),
            ],
            if (d == null)
              Text(
                Strings.noVersionsAvailable,
                style: TextStyle(color: t.muted),
              ),
            for (final VideoTrack track in d?.video ?? const <VideoTrack>[])
              _labelRow(
                t,
                Strings.videoHeading,
                _value(
                  t,
                  <String>[
                    track.codec,
                    '${track.width}×${track.height}',
                    track.hdr?.name ?? 'SDR',
                    '${track.bitDepth}-bit',
                  ].join(' · '),
                ),
              ),
            for (final AudioTrack track in d?.audio ?? const <AudioTrack>[])
              _labelRow(
                t,
                Strings.audioHeading,
                _value(
                  t,
                  <String>[
                    track.codec,
                    '${track.channels}ch',
                    if (track.language != null)
                      '(${languageLabel(track.language!)})',
                  ].join(' '),
                ),
              ),
            if (widget.version.sizeBytes > 0)
              _labelRow(
                t,
                Strings.factSize,
                _value(t, gigabytes(widget.version.sizeBytes)),
              ),
            if (d != null)
              _labelRow(
                t,
                Strings.subtitlesHeading,
                languages.isEmpty
                    ? _value(t, Strings.subtitlesNone)
                    : Wrap(
                        spacing: Space.s2,
                        runSpacing: Space.s2,
                        children: <Widget>[
                          for (final String lang in languages)
                            _langChip(t, lang),
                        ],
                      ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _labelRow(Tokens t, String label, Widget value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.s2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SizedBox(
            width: 84,
            child: Text(
              label.toUpperCase(),
              style: monoStyle.copyWith(
                color: t.muted,
                fontSize: 11,
                fontWeight: FontWeight.w600,
                letterSpacing: 0.6,
              ),
            ),
          ),
          const SizedBox(width: Space.s2),
          Expanded(child: value),
        ],
      ),
    );
  }

  Widget _value(Tokens t, String text) =>
      Text(text, style: monoStyle.copyWith(color: t.text, fontSize: 13));

  Widget _langChip(Tokens t, String lang) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.s2, vertical: 2),
      decoration: BoxDecoration(
        color: t.surfaceAlt,
        borderRadius: const BorderRadius.all(Radii.pill),
        border: Border.all(color: t.border),
      ),
      child: Text(lang, style: monoStyle.copyWith(color: t.text, fontSize: 12)),
    );
  }
}
