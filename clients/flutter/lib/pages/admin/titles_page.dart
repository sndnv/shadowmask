import 'dart:async';

import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/all_versions_dialog.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/title_row.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/view/page.dart';
import 'package:shadowmask/view/title_actions.dart';
import 'package:shadowmask/view/version_order.dart';

const List<TitleSort> _columnSorts = <TitleSort>[
  TitleSort.title,
  TitleSort.kind,
  TitleSort.year,
  TitleSort.versions,
  TitleSort.added,
];

String _name(TitleRow r) => switch (r.kind) {
  TitleKind.season => r.title ?? Strings.seasonLabel(r.seasonNumber ?? 0),
  TitleKind.episode =>
    '${Strings.episodeCode(r.seasonNumber, r.episodeNumber ?? 0)} '
            '${r.title ?? ''}'
        .trim(),
  _ => r.title ?? r.id,
};

String? _context(TitleRow r) => switch ((r.kind, r.seriesTitle)) {
  (TitleKind.season, final String series) => series,
  (TitleKind.episode, final String series) =>
    '$series · ${Strings.seasonLabel(r.seasonNumber ?? 0)}',
  _ => null,
};

String _fullName(TitleRow r) => switch ((r.kind, r.seriesTitle)) {
  (TitleKind.season || TitleKind.episode, final String series) =>
    '$series · ${_name(r)}',
  _ => _name(r),
};

String _kind(TitleKind kind) => switch (kind) {
  TitleKind.movie => Strings.kindMovie,
  TitleKind.series => Strings.kindSeries,
  TitleKind.season => Strings.kindSeason,
  _ => Strings.kindEpisode,
};

String _versions(TitleRow r) => switch (r.episodes) {
  null => Strings.versionCountLabel(r.files),
  0 => Strings.noEpisodes,
  final int episodes => Strings.episodesWithFilesLabel(r.files, episodes),
};

TitleAvailability _availability(TitleRow r) => r.available
    ? TitleAvailability.available
    : r.files == 0
    ? TitleAvailability.noVersion
    : TitleAvailability.unavailable;

String? _deleteBlock(TitleRow r) => switch (r.kind) {
  TitleKind.season => deleteBlockForEpisodes(r.episodes ?? 0),
  TitleKind.series => deleteBlockForSeasons(r.seasons ?? 0),
  _ => deleteBlockForVersions(r.files),
};

String _deleteLabel(TitleKind kind) => switch (kind) {
  TitleKind.movie => Strings.deleteMovie,
  TitleKind.series => Strings.deleteSeries,
  TitleKind.season => Strings.deleteSeason,
  _ => Strings.deleteEpisode,
};

bool _listsVersions(TitleRow r) =>
    r.readable && (r.kind == TitleKind.movie || r.kind == TitleKind.episode);

class TitlesPage extends StatelessWidget {
  const TitlesPage({super.key, required this.api, this.offset = 0});

  final ApiClient api;
  final int offset;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      loading: const SkeletonPage(child: SkeletonRows()),
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _TitlesBody(
              admin: AdminApi(api),
              catalog: CatalogApi(api),
              playback: PlaybackApi(api),
              userId: user.id,
              offset: offset,
            )
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _TitlesBody extends StatefulWidget {
  const _TitlesBody({
    required this.admin,
    required this.catalog,
    required this.playback,
    required this.userId,
    required this.offset,
  });

  final AdminApi admin;
  final CatalogApi catalog;
  final PlaybackApi playback;
  final String userId;
  final int offset;

  @override
  State<_TitlesBody> createState() => _TitlesBodyState();
}

class _TitlesBodyState extends State<_TitlesBody> with Mutations<_TitlesBody> {
  late int _offset = widget.offset;
  late Future<Paged<TitleRow>> _future = _load();
  final TextEditingController _filter = TextEditingController();
  Timer? _debounce;
  String _needle = '';
  TitleKind? _kindFilter;
  TitleAvailability? _availabilityFilter;
  TitleSort _sort = TitleSort.title;
  bool _descending = false;
  int? _total;

  @override
  void dispose() {
    _debounce?.cancel();
    _filter.dispose();
    super.dispose();
  }

  Future<Paged<TitleRow>> _load() {
    final Future<Paged<TitleRow>> pending = widget.admin.titles(
      offset: _offset,
      filter: _needle,
      kind: _kindFilter,
      availability: _availabilityFilter,
      sort: _sort,
      descending: _descending,
    );
    pending.then((Paged<TitleRow> page) {
      if (!mounted || !identical(_future, pending)) {
        return;
      }
      final int last = page.total == 0 || page.limit <= 0
          ? 0
          : (page.total - 1) ~/ page.limit * page.limit;
      if (page.items.isEmpty && last < _offset) {
        _goTo(last);
        return;
      }
      if (page.total != _total) {
        setState(() => _total = page.total);
      }
    }, onError: (Object _, StackTrace _) {});
    return pending;
  }

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  void _goTo(int offset) {
    setState(() {
      _offset = offset;
      _future = _load();
    });
  }

  void _restart(VoidCallback change) {
    setState(() {
      change();
      _offset = 0;
      _future = _load();
    });
  }

  void _onFilterChanged(String value) {
    _debounce?.cancel();
    _debounce = Timer(kFilterDebounce, () {
      if (mounted) {
        _restart(() => _needle = value.trim());
      }
    });
  }

  void _onSort(int column, bool ascending) => _restart(() {
    _sort = _columnSorts[column];
    _descending = !ascending;
  });

  Future<void> _deleteCall(TitleRow r) => switch (r.kind) {
    TitleKind.movie => widget.admin.deleteMovie(r.id),
    TitleKind.series => widget.admin.deleteSeries(r.id),
    TitleKind.season => widget.admin.deleteSeason(r.id),
    _ => widget.admin.deleteEpisode(r.id),
  };

  Future<void> _delete(TitleRow r) async {
    final bool ok = await confirmDialog(
      context,
      title: _deleteLabel(r.kind),
      message: Strings.confirmDeleteTitle(_fullName(r)),
      confirmLabel: Strings.delete,
    );
    if (!ok || !mounted) {
      return;
    }
    await mutate(
      key: r.id,
      () => _deleteCall(r),
      successText: Strings.toastDeleted,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  Future<Paged<Version>> _versionsOf(TitleRow r) => r.kind == TitleKind.movie
      ? widget.catalog.movieVersions(r.id)
      : widget.catalog.episodeVersions(
          r.id,
          series: r.seriesId,
          season: r.seasonId,
        );

  Future<void> _showVersions(TitleRow r) async {
    List<Version>? found;
    await mutate(key: 'versions:${r.id}', () async {
      found = (await _versionsOf(r)).items;
    }, errorText: Strings.couldNotLoadVersions);
    final List<Version>? versions = found;
    if (versions == null || !mounted) {
      return;
    }
    await AllVersionsDialog.show(
      context,
      catalog: widget.catalog,
      playback: widget.playback,
      userId: widget.userId,
      versions: orderedVersions(versions),
      admin: widget.admin,
      adminPages: true,
    );
  }

  @override
  Widget build(BuildContext context) {
    final int? total = _total;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(<Crumb>[
          Crumb(Strings.adminHeading, route: adminRoute()),
          Crumb(
            total == null
                ? Strings.adminTitles
                : Strings.countLabel(Strings.adminTitles, total),
          ),
        ]),
        PageActions(<PageAction>[
          PageAction(
            icon: Icons.refresh,
            label: Strings.refresh,
            onPressed: _reload,
          ),
        ]),
        const SizedBox(height: Space.s4),
        Wrap(
          spacing: Space.s3,
          runSpacing: Space.s2,
          children: <Widget>[
            AdminFilterField(
              controller: _filter,
              hintText: Strings.filterTitles,
              onChanged: _onFilterChanged,
            ),
            AppDropdown<TitleKind?>(
              value: _kindFilter,
              label: Strings.columnKind,
              width: 160,
              items: const <(TitleKind?, String)>[
                (null, Strings.availabilityAll),
                (TitleKind.movie, Strings.typeMovie),
                (TitleKind.series, Strings.typeSeries),
                (TitleKind.season, Strings.typeSeason),
                (TitleKind.episode, Strings.typeEpisode),
              ],
              onChanged: (TitleKind? kind) =>
                  _restart(() => _kindFilter = kind),
            ),
            AppDropdown<TitleAvailability?>(
              value: _availabilityFilter,
              label: Strings.availability,
              width: 160,
              items: const <(TitleAvailability?, String)>[
                (null, Strings.availabilityAll),
                (TitleAvailability.available, Strings.availabilityAvailable),
                (
                  TitleAvailability.unavailable,
                  Strings.availabilityUnavailable,
                ),
                (TitleAvailability.noVersion, Strings.availabilityNoVersion),
              ],
              onChanged: (TitleAvailability? availability) =>
                  _restart(() => _availabilityFilter = availability),
            ),
          ],
        ),
        const SizedBox(height: Space.s3),
        buildBlock<Paged<TitleRow>>(
          future: _future,
          errorText: Strings.couldNotLoadTitles,
          builder: _table,
        ),
      ],
    );
  }

  Color _tint(Tokens t, TitleRow r) => switch (_availability(r)) {
    TitleAvailability.available => t.rowOk,
    TitleAvailability.unavailable => t.rowDanger,
    TitleAvailability.noVersion => t.rowWarn,
  };

  Widget _text(String value) =>
      Text(value, maxLines: 1, overflow: TextOverflow.ellipsis);

  Widget _named(BuildContext context, Tokens t, String name, String? below) {
    if (below == null) {
      return _text(name);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _text(name),
        Text(
          below,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(
            context,
          ).textTheme.bodySmall?.copyWith(color: t.muted),
        ),
      ],
    );
  }

  Widget _actions(TitleRow r) {
    final String? block = _deleteBlock(r);
    final bool enabled = block == null && !busy(r.id);
    final String loading = 'versions:${r.id}';
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (_listsVersions(r))
          GestureDetector(
            excludeFromSemantics: true,
            onTap: busy(loading) ? () {} : null,
            child: IconButton(
              icon: const Icon(Icons.layers),
              tooltip: Strings.versionsHeading,
              onPressed: busy(loading) ? null : () => _showVersions(r),
            ),
          )
        else
          const SizedBox(width: kMinInteractiveDimension),
        GestureDetector(
          excludeFromSemantics: true,
          onTap: enabled ? null : () {},
          child: DangerIconButton(
            icon: Icons.delete_outline,
            tooltip: block ?? _deleteLabel(r.kind),
            onPressed: enabled ? () => _delete(r) : null,
          ),
        ),
      ],
    );
  }

  Widget _table(BuildContext context, Paged<TitleRow> page) {
    final Tokens t = context.tokens;
    final bool filtered =
        _needle.isNotEmpty ||
        _kindFilter != null ||
        _availabilityFilter != null;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        AdminTable<TitleRow>(
          rows: page.items,
          emptyText: filtered ? Strings.noMatchingTitles : Strings.emptyTitles,
          minWidth: 1080,
          onRowTap: (TitleRow r) => Navigator.of(context).pushNamed(
            titleRoute(r.kind, r.id, series: r.seriesId, season: r.seasonId),
          ),
          canTapRow: (TitleRow r) => r.readable,
          rowLabel: Strings.openTitle,
          rowColor: (TitleRow r) => _tint(t, r),
          initialSortColumn: _columnSorts.indexOf(_sort),
          initialSortAscending: !_descending,
          onSort: _onSort,
          columns: <AdminColumn<TitleRow>>[
            AdminColumn<TitleRow>(
              label: Strings.columnTitle,
              size: AdminColumnSize.large,
              essential: true,
              sortable: true,
              cell: (BuildContext context, TitleRow r) =>
                  _named(context, t, _name(r), _context(r)),
              narrowCell: (BuildContext context, TitleRow r) => _named(
                context,
                t,
                _name(r),
                '${_context(r) ?? _kind(r.kind)} · ${_versions(r)}',
              ),
            ),
            AdminColumn<TitleRow>(
              label: Strings.columnKind,
              size: AdminColumnSize.small,
              sortable: true,
              cell: (BuildContext context, TitleRow r) => _text(_kind(r.kind)),
            ),
            AdminColumn<TitleRow>(
              label: Strings.columnYear,
              size: AdminColumnSize.small,
              sortable: true,
              cell: (BuildContext context, TitleRow r) =>
                  _text(r.year?.toString() ?? ''),
            ),
            AdminColumn<TitleRow>(
              label: Strings.versionsHeading,
              sortable: true,
              cell: (BuildContext context, TitleRow r) => _text(_versions(r)),
            ),
            AdminColumn<TitleRow>(
              label: Strings.columnAdded,
              size: AdminColumnSize.small,
              sortable: true,
              cell: (BuildContext context, TitleRow r) =>
                  _text(dateText(r.addedAt) ?? ''),
            ),
            AdminColumn<TitleRow>(
              label: Strings.columnActions,
              align: AdminColumnAlign.end,
              essential: true,
              fixedWidth: adminActionsWidth(2),
              cell: (BuildContext context, TitleRow r) => _actions(r),
            ),
          ],
        ),
        const SizedBox(height: Space.s4),
        Pagination(
          basePath: adminTitlesRoute(),
          params: const <String, String?>{},
          total: page.total,
          offset: page.offset,
          limit: page.limit,
          count: page.items.length,
          onOffset: _goTo,
        ),
      ],
    );
  }
}
