import 'dart:async';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_filter_field.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/start_ellipsis_text.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/pagination.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/library/library.dart';
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
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/page.dart';

typedef _VersionsData = ({
  Paged<Version> page,
  Map<String, String> titles,
  Map<String, String> libraries,
});

class VersionsPage extends StatelessWidget {
  const VersionsPage({super.key, required this.api, this.offset = 0});

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
          ? _VersionsBody(
              admin: AdminApi(api),
              catalog: CatalogApi(api),
              libraries: LibraryApi(api),
              offset: offset,
            )
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _VersionsBody extends StatefulWidget {
  const _VersionsBody({
    required this.admin,
    required this.catalog,
    required this.libraries,
    required this.offset,
  });

  final AdminApi admin;
  final CatalogApi catalog;
  final LibraryApi libraries;
  final int offset;

  @override
  State<_VersionsBody> createState() => _VersionsBodyState();
}

class _VersionsBodyState extends State<_VersionsBody>
    with Mutations<_VersionsBody> {
  late int _offset = widget.offset;
  late Future<_VersionsData> _future = _load();
  final TextEditingController _filter = TextEditingController();
  Timer? _debounce;
  String _needle = '';
  int? _total;

  @override
  void dispose() {
    _debounce?.cancel();
    _filter.dispose();
    super.dispose();
  }

  Future<_VersionsData> _load() {
    final Future<_VersionsData> pending = _fetch();
    pending.then((_VersionsData data) {
      if (!mounted ||
          !identical(_future, pending) ||
          data.page.total == _total) {
        return;
      }
      setState(() => _total = data.page.total);
    }, onError: (Object _, StackTrace _) {});
    return pending;
  }

  Future<_VersionsData> _fetch() async {
    final Future<Map<String, String>> pendingLibraries = widget.libraries
        .libraries()
        .then(
          (List<Library> items) => <String, String>{
            for (final Library l in items) l.id: l.name,
          },
        )
        .catchError((Object _) => <String, String>{});
    final Paged<Version> page = await widget.admin.versions(
      offset: _offset,
      filter: _needle,
    );
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
          titles[card.ref.id] = card.listName;
        }
      } catch (_) {}
    }
    return (page: page, titles: titles, libraries: await pendingLibraries);
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

  void _onFilterChanged(String value) {
    _debounce?.cancel();
    _debounce = Timer(kFilterDebounce, () {
      if (!mounted) {
        return;
      }
      setState(() {
        _needle = value.trim();
        _offset = 0;
        _future = _load();
      });
    });
  }

  Future<void> _remove(Version v) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.removeVersion,
      message: Strings.confirmRemoveVersion(v.path ?? v.id),
      confirmLabel: Strings.remove,
    );
    if (!ok) {
      return;
    }
    await mutate(
      key: v.id,
      () => widget.admin.deleteVersion(v.id),
      successText: Strings.toastVersionRemoved,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  String _titleOf(Map<String, String> titles, Version v) {
    final String? name = titles[v.title.id];
    return name == null || name.isEmpty ? v.title.id : name;
  }

  String _libraryOf(Map<String, String> libraries, Version v) =>
      libraries[v.libraryId] ?? v.libraryId;

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
                ? Strings.adminVersions
                : Strings.countLabel(Strings.adminVersions, total),
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
        AdminFilterField(
          controller: _filter,
          hintText: Strings.filterVersions,
          onChanged: _onFilterChanged,
        ),
        const SizedBox(height: Space.s3),
        buildBlock<_VersionsData>(
          future: _future,
          errorText: Strings.couldNotLoadVersions,
          builder: _table,
        ),
      ],
    );
  }

  Widget _table(BuildContext context, _VersionsData data) {
    final Tokens t = context.tokens;
    final Paged<Version> page = data.page;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        AdminTable<Version>(
          rows: page.items,
          emptyText: _needle.isEmpty
              ? Strings.emptyVersions
              : Strings.noMatchingVersions,
          minWidth: 1080,
          initialSortColumn: 0,
          onRowTap: (Version v) =>
              Navigator.of(context).pushNamed(versionRoute(v.id)),
          rowLabel: Strings.openVersion,
          rowColor: (Version v) => v.available ? t.rowOk : t.rowDanger,
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
              narrowCell: (BuildContext context, Version v) =>
                  StartEllipsisText.middle(
                    p.basename(v.path ?? v.id),
                    tooltip: v.path ?? v.id,
                    style: monoStyle.copyWith(color: t.text, fontSize: 12),
                  ),
            ),
            AdminColumn<Version>(
              label: Strings.columnTitle,
              sortKey: (Version v) => _titleOf(data.titles, v),
              cell: (BuildContext context, Version v) => Text(
                _titleOf(data.titles, v),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            AdminColumn<Version>(
              label: Strings.columnLibrary,
              sortKey: (Version v) => _libraryOf(data.libraries, v),
              cell: (BuildContext context, Version v) => Text(
                _libraryOf(data.libraries, v),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            AdminColumn<Version>(
              label: Strings.columnQuality,
              size: AdminColumnSize.small,
              sortKey: (Version v) => v.quality.label,
              cell: (BuildContext context, Version v) => Text(v.quality.label),
            ),
            AdminColumn<Version>(
              label: Strings.versionContainer,
              size: AdminColumnSize.small,
              sortKey: (Version v) => v.container,
              cell: (BuildContext context, Version v) => Text(v.container),
            ),
            AdminColumn<Version>(
              label: Strings.columnSize,
              size: AdminColumnSize.small,
              align: AdminColumnAlign.end,
              sortKey: (Version v) => v.sizeBytes,
              cell: (BuildContext context, Version v) =>
                  Text(gigabytes(v.sizeBytes)),
            ),
            AdminColumn<Version>(
              label: Strings.columnActions,
              size: AdminColumnSize.small,
              align: AdminColumnAlign.end,
              cell: (BuildContext context, Version v) => DangerIconButton(
                icon: Icons.delete_outline,
                tooltip: Strings.removeVersion,
                onPressed: busy(v.id) ? null : () => _remove(v),
              ),
            ),
          ],
        ),
        const SizedBox(height: Space.s4),
        Pagination(
          basePath: adminVersionsRoute(),
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
