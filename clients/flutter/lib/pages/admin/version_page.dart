import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_header_split.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/admin/refresh_metadata_dialog.dart';
import 'package:shadowmask/components/admin/relink_dialog.dart';
import 'package:shadowmask/components/admin/subtitle_dialogs.dart';
import 'package:shadowmask/components/admin/trickplay_sheets.dart';
import 'package:shadowmask/components/admin/version_job_dialogs.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/card_art.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/movie_detail.dart';
import 'package:shadowmask/model/catalog/series_detail.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/languages.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

typedef _VersionData = ({
  VersionDetail detail,
  CatalogCard? card,
  String? seriesId,
  bool targetEdited,
});

typedef SubtitleRow = ({
  String format,
  String? language,
  String flags,
  SubtitleFile? file,
});

class VersionPage extends StatelessWidget {
  const VersionPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    final String id = Uri.base.queryParameters['id'] ?? '';
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      errorText: Strings.couldNotLoadVersion,
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _VersionBody(api: api, id: id)
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _VersionBody extends StatefulWidget {
  const _VersionBody({required this.api, required this.id});

  final ApiClient api;
  final String id;

  @override
  State<_VersionBody> createState() => _VersionBodyState();
}

class _VersionBodyState extends State<_VersionBody>
    with Mutations<_VersionBody> {
  late final AdminApi _admin = AdminApi(widget.api);
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late final PlaybackApi _playback = PlaybackApi(widget.api);
  late Future<_VersionData> _future = _load();

  Future<_VersionData> _load() async {
    final VersionDetail detail = await _catalog.version(widget.id);
    CatalogCard? card;
    try {
      final List<CatalogCard> cards = await _catalog.titleCards(<TitleRef>[
        detail.title,
      ], asSeriesPoster: true);
      card = cards.isEmpty ? null : cards.first;
    } catch (_) {}
    final ({String? seriesId, bool edited}) target = await _refreshTargetState(
      detail,
    );
    return (
      detail: detail,
      card: card,
      seriesId: target.seriesId,
      targetEdited: target.edited,
    );
  }

  Future<({String? seriesId, bool edited})> _refreshTargetState(
    VersionDetail detail,
  ) async {
    try {
      if (detail.title.type == TitleKind.movie) {
        final MovieDetail movie = await _catalog.movie(detail.title.id);
        return (seriesId: null, edited: movie.manuallyEdited);
      }
      if (detail.title.type == TitleKind.episode) {
        final String? series = (await _catalog.episode(
          detail.title.id,
        )).seriesId;
        if (series != null) {
          final SeriesDetail show = await _catalog.seriesDetail(series);
          return (seriesId: series, edited: show.manuallyEdited);
        }
      }
    } catch (_) {}
    return (seriesId: null, edited: false);
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<void> _after(Future<bool> action) => runBusy(() async {
    if (await action && mounted) {
      _reload();
    }
  });

  ({TitleKind kind, String id})? _refreshTarget(_VersionData data) {
    final TitleRef ref = data.detail.title;
    final String? series = data.seriesId;
    if (ref.type == TitleKind.movie) {
      return (kind: TitleKind.movie, id: ref.id);
    }
    if (ref.type == TitleKind.episode && series != null) {
      return (kind: TitleKind.series, id: series);
    }
    return null;
  }

  Future<void> _refresh(
    _VersionData data,
    ({TitleKind kind, String id}) target,
  ) => _after(
    showRefreshMetadataDialog(
      context,
      admin: _admin,
      kind: target.kind,
      id: target.id,
      manuallyEdited: data.targetEdited,
    ),
  );

  Future<void> _delete(_VersionData data) async {
    final VersionDetail d = data.detail;
    final bool ok = await confirmDialog(
      context,
      title: Strings.removeVersion,
      message: Strings.confirmRemoveVersion(d.path ?? d.id),
      confirmLabel: Strings.remove,
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => _admin.deleteVersion(d.id),
      successText: Strings.toastVersionRemoved,
      errorText: Strings.errorDelete,
      then: () => Navigator.of(
        context,
      ).pushReplacementNamed(data.card?.route ?? adminVersionsRoute()),
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<_VersionData>(
      future: _future,
      errorText: Strings.couldNotLoadVersion,
      builder: (BuildContext context, _VersionData data) {
        final VersionDetail d = data.detail;
        final ({TitleKind kind, String id})? target = _refreshTarget(data);
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Breadcrumbs(<Crumb>[
              Crumb(Strings.adminHeading, route: adminRoute()),
              Crumb(Strings.adminVersions, route: adminVersionsRoute()),
              Crumb(_shortId(d.id)),
            ]),
            const SizedBox(height: Space.s4),
            AdminHeaderSplit(
              start: _Summary(
                detail: d,
                onDelete: busy() ? null : () => _delete(data),
              ),
              end: _LinkedTitle(
                detail: d,
                card: data.card,
                imageBase: _catalog.imageBase,
                onRelink: !d.available
                    ? null
                    : () => _after(
                        showRelinkDialog(
                          context,
                          admin: _admin,
                          catalog: _catalog,
                          versionIds: <String>[d.id],
                        ),
                      ),
                onRefresh: target == null ? null : () => _refresh(data, target),
              ),
            ),
            _Video(detail: d, run: _after, admin: _admin),
            _Audio(detail: d, run: _after, admin: _admin),
            _Subtitles(detail: d, run: _after, admin: _admin),
            _Extras(detail: d, playback: _playback),
          ],
        );
      },
    );
  }
}

const double _kFourIconActions = 200;
const double _kPosterWidth = 96;
const double _kFactLabelWidth = 120;
const double _kMinFactValueWidth = 160;
const double _kPosterBesideFacts =
    _kPosterWidth + Space.s4 + _kFactLabelWidth + _kMinFactValueWidth;

String _shortId(String id) => id.split('-').first;

String _kindLabel(TitleKind kind) => switch (kind) {
  TitleKind.movie => Strings.kindMovie,
  TitleKind.series => Strings.kindSeries,
  TitleKind.season => Strings.kindSeason,
  TitleKind.episode => Strings.kindEpisode,
  TitleKind.person => Strings.kindPerson,
  TitleKind.collection => Strings.kindCollection,
};

class _Summary extends StatelessWidget {
  const _Summary({required this.detail, this.onDelete});

  final VersionDetail detail;
  final VoidCallback? onDelete;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final VersionDetail d = detail;
    return SectionBlock(
      title: '${d.quality.label} ${d.container}',
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.delete_outline,
          label: Strings.delete,
          danger: true,
          onPressed: onDelete,
        ),
      ],
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          _fact(context, t, Strings.factQuality, d.quality.label),
          _fact(context, t, Strings.factContainer, d.container),
          if (d.durationMs > 0)
            _fact(context, t, Strings.factDuration, durationText(d.durationMs)),
          if (d.sizeBytes > 0)
            _fact(context, t, Strings.factSize, megabytes(d.sizeBytes)),
          _fact(
            context,
            t,
            Strings.columnAvailable,
            d.available ? Strings.yes : Strings.no,
            valueColor: d.available ? t.ok : t.danger,
          ),
          if (dateText(d.addedAt) != null)
            _fact(context, t, Strings.factAdded, dateText(d.addedAt)!),
          if (dateText(d.updatedAt) != null)
            _fact(context, t, Strings.factUpdated, dateText(d.updatedAt)!),
          if (d.path != null)
            _fact(context, t, Strings.factPath, d.path!, mono: true),
        ],
      ),
    );
  }
}

class _LinkedTitle extends StatelessWidget {
  const _LinkedTitle({
    required this.detail,
    required this.card,
    required this.imageBase,
    required this.onRelink,
    required this.onRefresh,
  });

  final VersionDetail detail;
  final CatalogCard? card;
  final String imageBase;
  final VoidCallback? onRelink;
  final VoidCallback? onRefresh;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final TitleRef ref = detail.title;
    final CatalogCard? c = card;
    final bool episode = ref.type == TitleKind.episode;
    final String? year = episode ? null : c?.subtitle;
    return SectionBlock(
      title: Strings.linkedTitleHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.link,
          label: Strings.relink,
          tooltip: onRelink == null ? Strings.relinkBlockedVersion : null,
          onPressed: onRelink,
        ),
        PageAction(
          icon: Icons.refresh,
          label: Strings.refreshMetadata,
          tooltip: onRefresh == null ? Strings.refreshUnavailable : null,
          onPressed: onRefresh,
        ),
      ],
      child: AdminHeaderSplit(
        startWidth: _kPosterWidth,
        breakpoint: _kPosterBesideFacts,
        start: CardArt(
          artwork: c?.artwork,
          aspect: CardAspect.poster,
          imageBase: imageBase,
        ),
        end: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (c != null)
              _fact(context, t, Strings.factName, _name(c, episode)),
            _fact(context, t, Strings.factKind, _kindLabel(ref.type)),
            if (year != null) _fact(context, t, Strings.factYear, year),
            _fact(context, t, Strings.factId, ref.id, mono: true),
            const SizedBox(height: Space.s3),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                onPressed: () => Navigator.of(
                  context,
                ).pushNamed(c?.route ?? titleRoute(ref.type, ref.id)),
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text(Strings.openTitle),
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _name(CatalogCard c, bool episode) => episode
      ? <String>[c.title, ?c.subtitle, ?c.caption].join(' · ')
      : c.title;
}

Widget _fact(
  BuildContext context,
  Tokens t,
  String label,
  String value, {
  bool mono = false,
  Color? valueColor,
}) => Padding(
  padding: const EdgeInsets.symmetric(vertical: Space.s1),
  child: Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      SizedBox(
        width: _kFactLabelWidth,
        child: Text(
          label,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: t.muted),
        ),
      ),
      Expanded(
        child: Text(
          value,
          style: mono
              ? monoStyle.copyWith(color: valueColor ?? t.text, fontSize: 12)
              : Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: valueColor),
        ),
      ),
    ],
  ),
);

class _Video extends StatelessWidget {
  const _Video({required this.detail, required this.run, required this.admin});

  final VersionDetail detail;
  final Future<void> Function(Future<bool>) run;
  final AdminApi admin;

  @override
  Widget build(BuildContext context) {
    final VersionDetail d = detail;
    return SectionBlock(
      title: Strings.videoHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.arrow_upward,
          label: Strings.upscale,
          onPressed: () => run(
            showUpscaleDialog(
              context,
              admin: admin,
              versionId: d.id,
              video: d.video.isEmpty ? null : d.video.first,
            ),
          ),
        ),
      ],
      child: AdminTable<VideoTrack>(
        minWidth: 420,
        rows: d.video,
        emptyText: Strings.nothingToShowYet,
        columns: <AdminColumn<VideoTrack>>[
          AdminColumn<VideoTrack>(
            label: Strings.columnCodec,
            essential: true,
            cell: (BuildContext context, VideoTrack v) => Text(v.codec),
          ),
          AdminColumn<VideoTrack>(
            label: Strings.columnResolution,
            essential: true,
            cell: (BuildContext context, VideoTrack v) =>
                Text('${v.width}×${v.height}'),
          ),
          AdminColumn<VideoTrack>(
            label: Strings.columnDepth,
            size: AdminColumnSize.small,
            cell: (BuildContext context, VideoTrack v) =>
                Text('${v.bitDepth}-bit'),
          ),
          AdminColumn<VideoTrack>(
            label: Strings.columnHdr,
            size: AdminColumnSize.small,
            cell: (BuildContext context, VideoTrack v) =>
                Text(v.hdr?.name.toUpperCase() ?? Strings.sdr),
          ),
          AdminColumn<VideoTrack>(
            label: Strings.columnFrameRate,
            size: AdminColumnSize.small,
            align: AdminColumnAlign.end,
            cell: (BuildContext context, VideoTrack v) =>
                Text('${v.frameRate.toStringAsFixed(0)} fps'),
          ),
        ],
      ),
    );
  }
}

class _Audio extends StatelessWidget {
  const _Audio({required this.detail, required this.run, required this.admin});

  final VersionDetail detail;
  final Future<void> Function(Future<bool>) run;
  final AdminApi admin;

  @override
  Widget build(BuildContext context) {
    final VersionDetail d = detail;
    return SectionBlock(
      title: Strings.audioHeading,
      child: AdminTable<AudioTrack>(
        minWidth: 420,
        rows: d.audio,
        emptyText: Strings.nothingToShowYet,
        columns: <AdminColumn<AudioTrack>>[
          AdminColumn<AudioTrack>(
            label: Strings.columnCodec,
            essential: true,
            cell: (BuildContext context, AudioTrack a) => Text(a.codec),
          ),
          AdminColumn<AudioTrack>(
            label: Strings.columnChannels,
            size: AdminColumnSize.small,
            cell: (BuildContext context, AudioTrack a) =>
                Text('${a.channels}ch'),
          ),
          AdminColumn<AudioTrack>(
            label: Strings.columnLanguage,
            size: AdminColumnSize.small,
            essential: true,
            cell: (BuildContext context, AudioTrack a) =>
                Text(a.language == null ? '-' : languageLabel(a.language!)),
          ),
          AdminColumn<AudioTrack>(
            label: Strings.columnActions,
            align: AdminColumnAlign.end,
            essential: true,
            cell: (BuildContext context, AudioTrack a) => IconButton(
              tooltip: Strings.transcribe,
              visualDensity: VisualDensity.compact,
              onPressed: () => run(
                showTranscribeDialog(
                  context,
                  admin: admin,
                  versionId: d.id,
                  audioTrackIndex: a.index,
                ),
              ),
              icon: const Icon(Icons.record_voice_over_outlined),
            ),
          ),
        ],
      ),
    );
  }
}

class _Subtitles extends StatelessWidget {
  const _Subtitles({
    required this.detail,
    required this.run,
    required this.admin,
  });

  final VersionDetail detail;
  final Future<void> Function(Future<bool>) run;
  final AdminApi admin;

  @override
  Widget build(BuildContext context) {
    final VersionDetail d = detail;
    final List<SubtitleFile> files = d.subtitleFiles;
    final List<SubtitleRow> rows = <SubtitleRow>[
      for (final SubtitleTrack s in d.subtitles)
        (
          format: s.format.name,
          language: s.language,
          flags: <String>[
            if (s.forced) Strings.subtitleForced,
            if (s.isDefault) Strings.subtitleDefault,
          ].join(' '),
          file: null,
        ),
      for (final SubtitleFile f in files)
        (
          format: f.format.name,
          language: f.language,
          flags: <String>[
            f.source.name,
            if (f.label != null) f.label!,
          ].join(' · '),
          file: f,
        ),
    ];
    return SectionBlock(
      title: Strings.subtitlesHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.merge_outlined,
          label: Strings.combineSubtitles,
          onPressed: files.length < 2
              ? null
              : () => run(
                  showCombineDialog(
                    context,
                    admin: admin,
                    versionId: d.id,
                    subs: files,
                  ),
                ),
        ),
        PageAction(
          icon: Icons.search,
          label: Strings.searchSubtitles,
          onPressed: () => run(
            showSubtitleSearch(
              context,
              admin: admin,
              versionId: d.id,
              existing: files,
            ),
          ),
        ),
      ],
      child: AdminTable<SubtitleRow>(
        minWidth: 420,
        rows: rows,
        emptyText: Strings.emptySubtitles,
        columns: <AdminColumn<SubtitleRow>>[
          AdminColumn<SubtitleRow>(
            label: Strings.columnFormat,
            size: AdminColumnSize.small,
            essential: true,
            cell: (BuildContext context, SubtitleRow s) =>
                Text(s.format.toUpperCase()),
          ),
          AdminColumn<SubtitleRow>(
            label: Strings.columnLanguage,
            size: AdminColumnSize.small,
            essential: true,
            cell: (BuildContext context, SubtitleRow s) =>
                Text(s.language?.toUpperCase() ?? '-'),
          ),
          AdminColumn<SubtitleRow>(
            label: Strings.columnFlags,
            cell: (BuildContext context, SubtitleRow s) =>
                Text(s.flags.isEmpty ? '-' : s.flags),
          ),
          AdminColumn<SubtitleRow>(
            label: Strings.columnActions,
            align: AdminColumnAlign.end,
            fixedWidth: _kFourIconActions,
            essential: true,
            cell: (BuildContext context, SubtitleRow s) =>
                _rowActions(context, d.id, s.file),
          ),
        ],
      ),
    );
  }

  Widget _rowActions(BuildContext context, String versionId, SubtitleFile? f) {
    if (f == null) {
      return const SizedBox.shrink();
    }
    return Wrap(
      alignment: WrapAlignment.end,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: <Widget>[
        IconButton(
          tooltip: Strings.viewText,
          visualDensity: VisualDensity.compact,
          onPressed: () => showSubtitleText(
            context,
            admin: admin,
            versionId: versionId,
            sub: f,
          ),
          icon: const Icon(Icons.subject),
        ),
        IconButton(
          tooltip: Strings.translate,
          visualDensity: VisualDensity.compact,
          onPressed: () => run(
            showTranslateDialog(
              context,
              admin: admin,
              versionId: versionId,
              source: f,
            ),
          ),
          icon: const Icon(Icons.translate),
        ),
        IconButton(
          tooltip: Strings.rename,
          visualDensity: VisualDensity.compact,
          onPressed: () => run(
            renameSubtitleFile(
              context,
              admin: admin,
              versionId: versionId,
              sub: f,
            ),
          ),
          icon: const Icon(Icons.drive_file_rename_outline),
        ),
        DangerIconButton(
          icon: Icons.delete_outline,
          tooltip: Strings.delete,
          compact: true,
          onPressed: () => run(
            deleteSubtitleFile(
              context,
              admin: admin,
              versionId: versionId,
              sub: f,
            ),
          ),
        ),
      ],
    );
  }
}

class _Extras extends StatelessWidget {
  const _Extras({required this.detail, required this.playback});

  final VersionDetail detail;
  final PlaybackApi playback;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final VersionDetail d = detail;
    final List<String> markers = <String>[
      for (final Marker m in d.markers?.intro ?? const <Marker>[])
        Strings.markerRange(
          Strings.markerIntro,
          clock(m.startMs),
          clock(m.endMs),
        ),
      for (final Marker m in d.markers?.credits ?? const <Marker>[])
        Strings.markerRange(
          Strings.markerCredits,
          clock(m.startMs),
          clock(m.endMs),
        ),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (d.chapters.isNotEmpty)
          SectionBlock(
            title: Strings.chaptersHeading,
            child: _lines(context, t, <String>[
              for (final Chapter c in d.chapters)
                '${clock(c.startMs)} · ${c.title}',
            ]),
          ),
        if (markers.isNotEmpty)
          SectionBlock(
            title: Strings.markersHeading,
            child: _lines(context, t, markers),
          ),
        if (d.trickplay.isNotEmpty)
          SectionBlock(
            title: Strings.versionTrickplay,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                _lines(context, t, <String>[
                  for (final TrickplayRef r in d.trickplay)
                    '${r.tileWidth}×${r.tileHeight} · ${r.columns}×${r.rows} · '
                        '${r.sheets} sheets · ${r.intervalMs}ms',
                ]),
                const SizedBox(height: Space.s3),
                TrickplaySheets(
                  api: playback,
                  versionId: d.id,
                  refs: d.trickplay,
                ),
              ],
            ),
          ),
      ],
    );
  }

  Widget _lines(BuildContext context, Tokens t, List<String> lines) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: <Widget>[
      for (final String line in lines)
        Padding(
          padding: const EdgeInsets.only(bottom: Space.s1),
          child: Text(
            line,
            style: monoStyle.copyWith(color: t.text, fontSize: 12),
          ),
        ),
    ],
  );
}
