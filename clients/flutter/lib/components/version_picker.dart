import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/action_segments.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/download_link.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/session/resume_position.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/absolute_url.dart';
import 'package:shadowmask/util/downloads.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/languages.dart';
import 'package:shadowmask/view/failure_reason.dart';
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

String versionMetaRest(Version v, VersionDetail? d) {
  final List<String> bits = <String>[v.container];
  final VideoTrack? video = (d?.video.isNotEmpty ?? false)
      ? d!.video.first
      : null;
  if (video != null) {
    bits.add(video.codec);
  }
  final AudioTrack? audio = (d?.audio.isNotEmpty ?? false)
      ? d!.audio.first
      : null;
  if (audio != null) {
    bits.add('${audio.codec} ${audio.channels}ch');
  }
  return bits.join(' · ');
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
  });

  final CatalogApi catalog;
  final PlaybackApi playback;
  final String userId;
  final List<Version> versions;
  final VoidCallback? onProgressCleared;
  final bool showHeading;

  @override
  State<VersionPicker> createState() => _VersionPickerState();
}

class _VersionPickerState extends State<VersionPicker> {
  late final List<Version> _ordered = orderedVersions(widget.versions);
  late final Future<List<VersionDetail?>> _details = _load();

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
    return FutureBuilder<List<VersionDetail?>>(
      future: _details,
      builder:
          (BuildContext context, AsyncSnapshot<List<VersionDetail?>> snap) {
            final List<VersionDetail?>? details = snap.data;
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
                          ),
                      ],
                    ),
                  ),
                if (count > 0) ...<Widget>[
                  const SizedBox(height: Space.s2),
                  Text(
                    Strings.downloadNoSubtitles,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: context.tokens.muted,
                    ),
                  ),
                ],
              ],
            );
          },
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
  });

  final Version version;
  final String number;
  final VersionDetail? detail;
  final PlaybackApi playback;
  final String userId;
  final bool showTopBorder;
  final VoidCallback? onDismissed;

  @override
  State<_VersionRow> createState() => _VersionRowState();
}

class _VersionRowState extends State<_VersionRow> {
  bool _open = false;
  int _positionMs = 0;
  bool _busyDismiss = false;
  bool _busyDownload = false;

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

  @override
  void initState() {
    super.initState();
    if (widget.version.available) {
      _loadResume();
    }
  }

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
          final bool narrow = constraints.maxWidth < kVersionRowSideBySide;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              if (narrow)
                _narrowHeader(context, t, v, d, available)
              else
                _wideHeader(context, t, v, d, available),
              if (_open) _detailBody(context, t, v, d, available, narrow),
            ],
          );
        },
      ),
    );
  }

  Widget _label(Tokens t, Version v, VersionDetail? d, bool available) =>
      Text.rich(
        TextSpan(
          style: monoStyle.copyWith(color: available ? t.text : t.muted),
          children: <InlineSpan>[
            TextSpan(
              text: '${widget.number} · ',
              style: TextStyle(color: t.muted),
            ),
            TextSpan(
              text: v.quality.label,
              style: TextStyle(color: available ? t.accent : t.muted),
            ),
            TextSpan(text: ' · ${versionMetaRest(v, d)}'),
          ],
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
            Expanded(child: _label(t, v, d, available)),
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
                    child: _label(t, v, d, available),
                  ),
                ),
                if (available) _downloadButton(t, v),
              ],
            ),
          ),
          if (d != null) _ccPill(t, distinctSubtitleLanguages(d)),
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

  Widget _ccPill(Tokens t, int count) {
    final bool has = count > 0;
    final Color fg = has ? t.accent : t.muted;
    final Color borderColor = has
        ? Color.alphaBlend(t.accent.withValues(alpha: 0.45), t.border)
        : t.border;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.s2, vertical: 2),
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radii.pill),
        border: Border.all(color: borderColor),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.closed_caption_outlined, size: 14, color: fg),
          const SizedBox(width: 5),
          Text(
            has ? '$count' : Strings.ccNone,
            style: monoStyle.copyWith(
              color: fg,
              fontSize: 11,
              fontWeight: FontWeight.w600,
              fontFeatures: const <FontFeature>[FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }

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
            if (narrow && available) ...<Widget>[
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: <Widget>[
                  Flexible(child: _trailing(context, t, v, available)),
                  _downloadButton(t, v),
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
                _value(t, megabytes(widget.version.sizeBytes)),
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
