import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/start_ellipsis_text.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/library_form_dialog.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/library/duplicate_candidate.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/model/library/resolve_candidate.dart';
import 'package:shadowmask/model/library/scan_state.dart';
import 'package:shadowmask/model/library/unmatched_file.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/format.dart';
import 'package:shadowmask/util/library_labels.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/page.dart';

class LibraryPage extends StatelessWidget {
  const LibraryPage({super.key, required this.api, this.libraryId});

  final ApiClient api;
  final String? libraryId;

  @override
  Widget build(BuildContext context) {
    final String? id = libraryId ?? Uri.base.queryParameters['id'];
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) {
        if (!user.isAdmin) {
          return const StatusText(Strings.notAuthorized);
        }
        if (id == null || id.isEmpty) {
          return const StatusText(Strings.couldNotLoadLibrary);
        }
        return _LibraryBody(api: api, id: id);
      },
    );
  }
}

class _LibraryBody extends StatefulWidget {
  const _LibraryBody({required this.api, required this.id});

  final ApiClient api;
  final String id;

  @override
  State<_LibraryBody> createState() => _LibraryBodyState();
}

class _LibraryBodyState extends State<_LibraryBody>
    with Mutations<_LibraryBody> {
  late final LibraryApi _libraries = LibraryApi(widget.api);
  late final CatalogApi _catalog = CatalogApi(widget.api);
  late Future<Library> _future = _libraries.library(widget.id);

  void _reload() {
    setState(() {
      _future = _libraries.library(widget.id);
    });
  }

  Future<void> _edit(Library library) async {
    final bool saved = await showLibraryFormDialog(
      context,
      libraries: _libraries,
      existing: library,
    );
    if (saved) {
      _reload();
    }
  }

  Future<void> _delete(Library library) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteLibrary,
      message: Strings.confirmDeleteLibrary(library.name),
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => _libraries.deleteLibrary(library.id),
      successText: Strings.toastLibraryDeleted,
      errorText: Strings.errorDelete,
      then: () =>
          Navigator.of(context).pushReplacementNamed(adminLibrariesRoute()),
    );
  }

  Future<void> _refreshMetadata() async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.refreshMetadata,
      message: Strings.confirmRefreshLibraryBody,
      confirmLabel: Strings.refreshMetadata,
      danger: false,
    );
    if (!ok) {
      return;
    }
    await mutate(
      () => _libraries.refreshMetadata(widget.id),
      successText: Strings.toastMetadataQueued,
      errorText: Strings.errorRefresh,
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<Library>(
      future: _future,
      errorText: Strings.couldNotLoadLibrary,
      builder: (BuildContext context, Library library) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(<Crumb>[
            Crumb(Strings.adminHeading, route: adminRoute()),
            Crumb(Strings.adminLibraries, route: adminLibrariesRoute()),
            Crumb(library.name),
          ]),
          const SizedBox(height: Space.s4),
          SectionBlock(
            title: library.name,
            actionItems: <PageAction>[
              PageAction(
                icon: Icons.delete_outline,
                label: Strings.delete,
                danger: true,
                onPressed: busy() ? null : () => _delete(library),
              ),
              PageAction(
                icon: Icons.refresh,
                label: Strings.refreshMetadata,
                onPressed: busy() ? null : _refreshMetadata,
              ),
              PageAction(
                icon: Icons.edit_outlined,
                label: Strings.edit,
                onPressed: () => _edit(library),
              ),
            ],
            child: _Facts(library: library),
          ),
          _ScanBlock(libraries: _libraries, id: widget.id),
          _DuplicatesBlock(libraries: _libraries, id: widget.id),
          _UnmatchedBlock(libraries: _libraries, id: widget.id),
          _VersionsBlock(
            libraries: _libraries,
            catalog: _catalog,
            id: widget.id,
          ),
        ],
      ),
    );
  }
}

class _Facts extends StatelessWidget {
  const _Facts({required this.library});

  final Library library;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        _kv(context, Strings.columnKind, libraryKindLabel(library.kind)),
        _kv(context, Strings.fieldOrigin, libraryOriginLabel(library.origin)),
        _kv(context, Strings.fieldWatcher, watcherLabel(library.watcher)),
        _kv(context, Strings.fieldRoots, library.roots.join(', ')),
        if (library.scanSchedule != null)
          _kv(context, Strings.fieldScanSchedule, library.scanSchedule!),
        _kv(
          context,
          Strings.fieldMetadataSources,
          library.metadataSources.join(', '),
        ),
      ],
    );
  }
}

Widget _kv(BuildContext context, String label, String value) {
  final Tokens t = context.tokens;
  return Padding(
    padding: const EdgeInsets.symmetric(vertical: Space.s1),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        SizedBox(
          width: 140,
          child: Text(
            label,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: t.muted),
          ),
        ),
        Expanded(child: Text(value.isEmpty ? '-' : value)),
      ],
    ),
  );
}

class _ScanBlock extends StatefulWidget {
  const _ScanBlock({required this.libraries, required this.id});

  final LibraryApi libraries;
  final String id;

  @override
  State<_ScanBlock> createState() => _ScanBlockState();
}

class _ScanBlockState extends State<_ScanBlock> with Mutations<_ScanBlock> {
  late Future<ScanState> _future = widget.libraries.scanState(widget.id);

  void _reload() {
    setState(() {
      _future = widget.libraries.scanState(widget.id);
    });
  }

  Future<void> _trigger() => mutate(
    () => widget.libraries.triggerScan(widget.id),
    successText: Strings.toastScanQueued,
    errorText: Strings.errorScan,
    then: _reload,
  );

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.scanHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.refresh,
          label: Strings.refresh,
          onPressed: _reload,
        ),
        PageAction(
          icon: Icons.play_arrow,
          label: Strings.scan,
          primary: true,
          onPressed: busy() ? null : _trigger,
        ),
      ],
      child: buildBlock<ScanState>(
        future: _future,
        builder: (BuildContext context, ScanState scan) => Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            _kv(context, Strings.columnStatus, scanStatusLabel(scan.status)),
            _kv(
              context,
              Strings.columnProgress,
              '${(scan.progress * 100).round()}%',
            ),
            if (scan.lastScannedAt != null)
              _kv(context, Strings.factUpdated, scan.lastScannedAt!),
            if (scan.error != null)
              _kv(context, Strings.jobFactError, scan.error!),
          ],
        ),
      ),
    );
  }
}

typedef _VersionsData = ({List<Version> versions, Map<String, String> titles});

class _VersionsBlock extends StatefulWidget {
  const _VersionsBlock({
    required this.libraries,
    required this.catalog,
    required this.id,
  });

  final LibraryApi libraries;
  final CatalogApi catalog;
  final String id;

  @override
  State<_VersionsBlock> createState() => _VersionsBlockState();
}

class _VersionsBlockState extends State<_VersionsBlock> {
  late Future<_VersionsData> _future = _load();
  int? _total;

  void _reload() {
    setState(() {
      _future = _load();
    });
  }

  Future<_VersionsData> _load() async {
    final Paged<Version> page = await widget.libraries.versions(widget.id);
    final Map<String, String> titles = <String, String>{};
    final List<TitleRef> refs = <TitleRef>[];
    for (final Version v in page.items) {
      if (!titles.containsKey(v.title.id)) {
        titles[v.title.id] = '';
        refs.add(v.title);
      }
    }
    if (refs.isNotEmpty) {
      try {
        for (final CatalogCard card in await widget.catalog.titleCards(refs)) {
          titles[card.ref.id] = card.title;
        }
      } catch (_) {}
    }
    if (mounted) {
      setState(() {
        _total = page.total;
      });
    }
    return (versions: page.items, titles: titles);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final int? total = _total;
    return SectionBlock(
      title: total == null
          ? Strings.adminVersions
          : Strings.versionsWithCount(total),
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.refresh,
          label: Strings.refresh,
          onPressed: _reload,
        ),
      ],
      child: buildBlock<_VersionsData>(
        future: _future,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, _VersionsData data) =>
            AdminTable<Version>(
              rows: data.versions,
              emptyText: Strings.emptyVersions,
              onRowTap: (Version v) =>
                  Navigator.of(context).pushNamed(versionRoute(v.id)),
              rowLabel: Strings.openVersion,
              columns: <AdminColumn<Version>>[
                AdminColumn<Version>(
                  label: Strings.columnPath,
                  size: AdminColumnSize.large,
                  essential: true,
                  sortKey: (Version v) => v.path ?? '',
                  cell: (BuildContext context, Version v) => StartEllipsisText(
                    v.path ?? v.id,
                    style: monoStyle.copyWith(color: t.text, fontSize: 12),
                  ),
                ),
                AdminColumn<Version>(
                  label: Strings.columnTitle,
                  sortKey: (Version v) => data.titles[v.title.id] ?? '',
                  cell: (BuildContext context, Version v) => Text(
                    _titleOf(data.titles, v),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                AdminColumn<Version>(
                  label: Strings.columnQuality,
                  size: AdminColumnSize.small,
                  sortKey: (Version v) => v.quality.label,
                  cell: (BuildContext context, Version v) =>
                      Text(v.quality.label),
                ),
                AdminColumn<Version>(
                  label: Strings.versionContainer,
                  size: AdminColumnSize.small,
                  cell: (BuildContext context, Version v) => Text(v.container),
                ),
                AdminColumn<Version>(
                  label: Strings.columnSize,
                  size: AdminColumnSize.small,
                  align: AdminColumnAlign.end,
                  sortKey: (Version v) => v.sizeBytes,
                  cell: (BuildContext context, Version v) =>
                      Text(megabytes(v.sizeBytes)),
                ),
                AdminColumn<Version>(
                  label: Strings.columnAvailable,
                  size: AdminColumnSize.small,
                  essential: true,
                  cell: (BuildContext context, Version v) => Text(
                    v.available ? Strings.yes : Strings.no,
                    style: TextStyle(color: v.available ? t.text : t.muted),
                  ),
                ),
              ],
            ),
      ),
    );
  }

  String _titleOf(Map<String, String> titles, Version v) {
    final String? name = titles[v.title.id];
    return name == null || name.isEmpty ? v.title.id : name;
  }
}

class _DuplicatesBlock extends StatefulWidget {
  const _DuplicatesBlock({required this.libraries, required this.id});

  final LibraryApi libraries;
  final String id;

  @override
  State<_DuplicatesBlock> createState() => _DuplicatesBlockState();
}

class _DuplicatesBlockState extends State<_DuplicatesBlock>
    with Mutations<_DuplicatesBlock> {
  late Future<Paged<DuplicateCandidate>> _future = widget.libraries.duplicates(
    widget.id,
  );

  void _reload() {
    setState(() {
      _future = widget.libraries.duplicates(widget.id);
    });
  }

  Future<void> _dismiss(DuplicateCandidate d) => mutate(
    key: d.id,
    () => widget.libraries.dismissDuplicate(widget.id, d.id),
    successText: Strings.toastDismissed,
    errorText: Strings.errorAction,
    then: _reload,
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.duplicatesHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.refresh,
          label: Strings.refresh,
          onPressed: _reload,
        ),
      ],
      child: buildBlock<Paged<DuplicateCandidate>>(
        future: _future,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, Paged<DuplicateCandidate> page) =>
            page.items.isEmpty
            ? const StatusText(Strings.emptyDuplicates)
            : Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: Space.s3,
                children: <Widget>[
                  for (final DuplicateCandidate d in page.items)
                    Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            d.paths.join('\n'),
                            style: monoStyle.copyWith(
                              color: t.text,
                              fontSize: 12,
                            ),
                          ),
                        ),
                        TextButton(
                          onPressed: busy(d.id) ? null : () => _dismiss(d),
                          child: const Text(Strings.dismiss),
                        ),
                      ],
                    ),
                ],
              ),
      ),
    );
  }
}

class _UnmatchedBlock extends StatefulWidget {
  const _UnmatchedBlock({required this.libraries, required this.id});

  final LibraryApi libraries;
  final String id;

  @override
  State<_UnmatchedBlock> createState() => _UnmatchedBlockState();
}

class _UnmatchedBlockState extends State<_UnmatchedBlock> {
  late Future<Paged<UnmatchedFile>> _future = widget.libraries.unmatched(
    widget.id,
  );

  void _reload() {
    setState(() {
      _future = widget.libraries.unmatched(widget.id);
    });
  }

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.unmatchedHeading,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.refresh,
          label: Strings.refresh,
          onPressed: _reload,
        ),
      ],
      child: buildBlock<Paged<UnmatchedFile>>(
        future: _future,
        loading: const SkeletonRows(rows: 3),
        builder: (BuildContext context, Paged<UnmatchedFile> page) =>
            page.items.isEmpty
            ? const StatusText(Strings.emptyUnmatched)
            : Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: Space.s3,
                children: <Widget>[
                  for (final UnmatchedFile f in page.items)
                    _UnmatchedTile(
                      libraries: widget.libraries,
                      libraryId: widget.id,
                      file: f,
                      onResolved: _reload,
                    ),
                ],
              ),
      ),
    );
  }
}

class _UnmatchedTile extends StatefulWidget {
  const _UnmatchedTile({
    required this.libraries,
    required this.libraryId,
    required this.file,
    required this.onResolved,
  });

  final LibraryApi libraries;
  final String libraryId;
  final UnmatchedFile file;
  final VoidCallback onResolved;

  @override
  State<_UnmatchedTile> createState() => _UnmatchedTileState();
}

class _UnmatchedTileState extends State<_UnmatchedTile>
    with Mutations<_UnmatchedTile> {
  Future<List<ResolveCandidate>>? _candidates;

  void _loadCandidates() {
    setState(() {
      _candidates = widget.libraries.unmatchedCandidates(
        widget.libraryId,
        widget.file.id,
      );
    });
  }

  Map<String, dynamic> _targetOf(ResolveTarget target) =>
      target.kind == 'existing'
      ? <String, dynamic>{'kind': 'existing', 'title': target.title!.toJson()}
      : <String, dynamic>{
          'kind': 'provider',
          'source': target.source,
          'value': target.value,
        };

  Future<void> _resolve(ResolveCandidate candidate) => mutate(
    () => widget.libraries.resolveUnmatched(
      widget.libraryId,
      widget.file.id,
      _targetOf(candidate.target),
    ),
    successText: Strings.toastResolved,
    errorText: Strings.errorAction,
    then: widget.onResolved,
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                widget.file.path,
                style: monoStyle.copyWith(color: t.text, fontSize: 12),
              ),
            ),
            TextButton(
              onPressed: _candidates == null ? _loadCandidates : null,
              child: const Text(Strings.resolve),
            ),
          ],
        ),
        if (_candidates != null)
          FutureBuilder<List<ResolveCandidate>>(
            future: _candidates,
            builder:
                (
                  BuildContext context,
                  AsyncSnapshot<List<ResolveCandidate>> snapshot,
                ) {
                  if (snapshot.connectionState != ConnectionState.done) {
                    return const StatusText(Strings.loading);
                  }
                  final List<ResolveCandidate> candidates =
                      snapshot.data ?? const <ResolveCandidate>[];
                  if (candidates.isEmpty) {
                    return const StatusText(Strings.emptyCandidates);
                  }
                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      for (final ResolveCandidate c in candidates)
                        Padding(
                          padding: const EdgeInsets.only(left: Space.s4),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Text(
                                  c.year == null
                                      ? '${c.title} [${c.kind}] via ${c.source}'
                                      : '${c.title} (${c.year}) [${c.kind}] via ${c.source}',
                                ),
                              ),
                              TextButton(
                                onPressed: busy() ? null : () => _resolve(c),
                                child: const Text(Strings.resolve),
                              ),
                            ],
                          ),
                        ),
                    ],
                  );
                },
          ),
      ],
    );
  }
}
