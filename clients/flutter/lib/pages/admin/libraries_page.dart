import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/admin/library_form_dialog.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/util/library_labels.dart';

class LibrariesPage extends StatelessWidget {
  const LibrariesPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      loading: const SkeletonPage(child: SkeletonRows()),
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _LibrariesBody(libraries: LibraryApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _LibrariesBody extends StatefulWidget {
  const _LibrariesBody({required this.libraries});

  final LibraryApi libraries;

  @override
  State<_LibrariesBody> createState() => _LibrariesBodyState();
}

class _LibrariesBodyState extends State<_LibrariesBody>
    with Mutations<_LibrariesBody> {
  late Future<List<Library>> _future = widget.libraries.libraries();

  void _reload() {
    setState(() {
      _future = widget.libraries.libraries();
    });
  }

  Future<void> _openForm({Library? existing}) async {
    final bool saved = await showLibraryFormDialog(
      context,
      libraries: widget.libraries,
      existing: existing,
    );
    if (saved) {
      _reload();
    }
  }

  Future<void> _scan(Library library) => mutate(
    key: library.id,
    () => widget.libraries.triggerScan(library.id),
    successText: Strings.toastScanQueuedFor(library.name),
    errorText: Strings.errorScanFor(library.name),
  );

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
      key: library.id,
      () => widget.libraries.deleteLibrary(library.id),
      successText: Strings.toastLibraryDeleted,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<List<Library>>(
      future: _future,
      errorText: Strings.couldNotLoadLibraries,
      builder: (BuildContext context, List<Library> libraries) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(<Crumb>[
            Crumb(Strings.adminHeading, route: adminRoute()),
            Crumb(Strings.countLabel(Strings.adminLibraries, libraries.length)),
          ]),
          PageActions(<PageAction>[
            PageAction(
              icon: Icons.refresh,
              label: Strings.refresh,
              onPressed: _reload,
            ),
            PageAction(
              icon: Icons.add,
              label: Strings.createLibrary,
              primary: true,
              onPressed: () => _openForm(),
            ),
          ]),
          const SizedBox(height: Space.s4),
          AdminTable<Library>(
            rows: libraries,
            emptyText: Strings.emptyLibraries,
            minWidth: 880,
            initialSortColumn: 0,
            onRowTap: (Library l) =>
                Navigator.of(context).pushNamed(adminLibraryRoute(l.id)),
            rowLabel: Strings.openLibrary,
            columns: <AdminColumn<Library>>[
              AdminColumn<Library>(
                label: Strings.columnName,
                size: AdminColumnSize.large,
                essential: true,
                sortKey: (Library l) => l.name,
                cell: (BuildContext c, Library l) =>
                    Text(l.name, overflow: TextOverflow.ellipsis),
              ),
              AdminColumn<Library>(
                label: Strings.columnKind,
                size: AdminColumnSize.small,
                essential: true,
                sortKey: (Library l) => l.kind.index,
                cell: (BuildContext c, Library l) =>
                    Text(libraryKindLabel(l.kind)),
              ),
              AdminColumn<Library>(
                label: Strings.fieldOrigin,
                size: AdminColumnSize.small,
                sortKey: (Library l) => l.origin.index,
                cell: (BuildContext c, Library l) =>
                    Text(libraryOriginLabel(l.origin)),
              ),
              AdminColumn<Library>(
                label: Strings.fieldWatcher,
                size: AdminColumnSize.small,
                sortKey: (Library l) => l.watcher.index,
                cell: (BuildContext c, Library l) =>
                    Text(watcherLabel(l.watcher)),
              ),
              AdminColumn<Library>(
                label: Strings.fieldRoots,
                sortKey: (Library l) => l.roots.length,
                cell: (BuildContext c, Library l) =>
                    Text(l.roots.join(', '), overflow: TextOverflow.ellipsis),
              ),
              AdminColumn<Library>(
                label: Strings.columnActions,
                align: AdminColumnAlign.end,
                cell: (BuildContext c, Library l) => Row(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    DangerIconButton(
                      icon: Icons.delete_outline,
                      tooltip: Strings.deleteLibrary,
                      onPressed: busy(l.id) ? null : () => _delete(l),
                    ),
                    IconButton(
                      tooltip: Strings.editLibrary,
                      onPressed: () => _openForm(existing: l),
                      icon: const Icon(Icons.edit_outlined),
                    ),
                    IconButton(
                      tooltip: Strings.scanLibrary,
                      onPressed: busy(l.id) ? null : () => _scan(l),
                      icon: const Icon(Icons.play_arrow),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
