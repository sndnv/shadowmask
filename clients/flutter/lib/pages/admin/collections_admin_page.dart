import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/admin_column.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/admin/match_row.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/collection.dart';
import 'package:shadowmask/model/catalog/movie.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/page.dart';

class CollectionsAdminPage extends StatelessWidget {
  const CollectionsAdminPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      loading: const SkeletonPage(child: SkeletonRows()),
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _CollectionsBody(catalog: CatalogApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _CollectionsBody extends StatefulWidget {
  const _CollectionsBody({required this.catalog});

  final CatalogApi catalog;

  @override
  State<_CollectionsBody> createState() => _CollectionsBodyState();
}

class _CollectionsBodyState extends State<_CollectionsBody>
    with Mutations<_CollectionsBody> {
  late Future<Paged<Collection>> _future = widget.catalog.collections();

  void _reload() {
    setState(() {
      _future = widget.catalog.collections();
    });
  }

  Future<void> _openEditor(Collection? existing) async {
    final bool? saved = await showDialog<bool>(
      context: context,
      builder: (BuildContext ctx) =>
          _CollectionEditorDialog(catalog: widget.catalog, existing: existing),
    );
    if (saved ?? false) {
      if (mounted) {
        Toasts.of(context).success(
          existing == null
              ? Strings.toastCollectionCreated
              : Strings.toastCollectionSaved,
        );
      }
      _reload();
    }
  }

  Future<void> _edit(Collection c) async {
    final Collection full;
    try {
      full = await widget.catalog.collection(c.id);
    } catch (e) {
      if (mounted) {
        Toasts.of(
          context,
        ).error(failureText(Strings.couldNotLoadCollection, e));
      }
      return;
    }
    if (mounted) {
      await _openEditor(full);
    }
  }

  Future<void> _delete(Collection c) async {
    final bool ok = await confirmDialog(
      context,
      title: Strings.deleteCollection,
      message: Strings.confirmDeleteCollection(c.name),
    );
    if (!ok) {
      return;
    }
    await mutate(
      key: c.id,
      () => widget.catalog.deleteCollection(c.id),
      successText: Strings.toastCollectionDeleted,
      errorText: Strings.errorDelete,
      then: _reload,
    );
  }

  @override
  Widget build(BuildContext context) {
    return buildBlock<Paged<Collection>>(
      future: _future,
      errorText: Strings.couldNotLoadCollections,
      builder: (BuildContext context, Paged<Collection> page) => Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Breadcrumbs(<Crumb>[
            Crumb(Strings.adminHeading, route: adminRoute()),
            Crumb(Strings.countLabel(Strings.adminCollections, page.total)),
          ]),
          PageActions(<PageAction>[
            PageAction(
              icon: Icons.refresh,
              label: Strings.refresh,
              onPressed: _reload,
            ),
            PageAction(
              icon: Icons.add,
              label: Strings.createCollection,
              primary: true,
              onPressed: () => _openEditor(null),
            ),
          ]),
          const SizedBox(height: Space.s4),
          AdminTable<Collection>(
            rows: page.items,
            emptyText: Strings.noCollectionsFound,
            minWidth: 720,
            initialSortColumn: 0,
            columns: <AdminColumn<Collection>>[
              AdminColumn<Collection>(
                label: Strings.columnName,
                size: AdminColumnSize.large,
                essential: true,
                sortKey: (Collection c) => c.name,
                cell: (BuildContext c, Collection col) =>
                    Text(col.name, overflow: TextOverflow.ellipsis),
              ),
              AdminColumn<Collection>(
                label: Strings.columnActions,
                fixedWidth: adminActionsWidth(2),
                align: AdminColumnAlign.end,
                essential: true,
                cell: (BuildContext c, Collection col) => Wrap(
                  alignment: WrapAlignment.end,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: <Widget>[
                    IconButton(
                      tooltip: Strings.editCollection,
                      onPressed: () => _edit(col),
                      icon: const Icon(Icons.edit_outlined),
                    ),
                    DangerIconButton(
                      icon: Icons.delete_outline,
                      tooltip: Strings.deleteCollection,
                      onPressed: busy(col.id) ? null : () => _delete(col),
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

class _Member {
  const _Member(this.id, this.title);

  final String id;
  final String title;
}

class _CollectionEditorDialog extends StatefulWidget {
  const _CollectionEditorDialog({required this.catalog, this.existing});

  final CatalogApi catalog;
  final Collection? existing;

  @override
  State<_CollectionEditorDialog> createState() =>
      _CollectionEditorDialogState();
}

class _CollectionEditorDialogState extends State<_CollectionEditorDialog> {
  late final TextEditingController _name = TextEditingController(
    text: widget.existing?.name ?? '',
  );
  late final TextEditingController _overview = TextEditingController(
    text: widget.existing?.overview ?? '',
  );
  final TextEditingController _query = TextEditingController();
  late final List<_Member> _members = <_Member>[
    for (final Movie m in widget.existing?.items ?? const <Movie>[])
      _Member(m.id, m.title),
  ];
  Future<Paged<CatalogCard>>? _results;
  bool _submitting = false;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    _overview.dispose();
    _query.dispose();
    super.dispose();
  }

  void _search() {
    final String q = _query.text.trim();
    if (q.isEmpty) {
      return;
    }
    setState(() {
      _results = widget.catalog.search(q: q, type: 'movie', limit: 20);
    });
  }

  void _add(CatalogCard card) {
    if (_members.any((_Member m) => m.id == card.ref.id)) {
      return;
    }
    setState(() => _members.add(_Member(card.ref.id, card.title)));
  }

  void _remove(_Member member) =>
      setState(() => _members.removeWhere((_Member m) => m.id == member.id));

  Future<void> _submit() async {
    if (_name.text.trim().isEmpty) {
      setState(() => _error = Strings.requiredName);
      return;
    }
    final String overview = _overview.text.trim();
    final Map<String, dynamic> body = <String, dynamic>{
      'name': _name.text.trim(),
      'overview': overview.isEmpty ? null : overview,
      'movies': _members.map((_Member m) => m.id).toList(),
    };
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final Collection? existing = widget.existing;
      if (existing == null) {
        await widget.catalog.createCollection(body);
      } else {
        await widget.catalog.updateCollection(existing.id, body);
      }
      if (mounted) {
        Navigator.of(context).pop(true);
      }
    } catch (_) {
      if (mounted) {
        setState(() {
          _error = Strings.errorSave;
          _submitting = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: widget.existing == null
          ? Strings.createCollection
          : Strings.editCollection,
      width: 640,
      enableClose: !_submitting,
      footer: Align(
        alignment: Alignment.centerRight,
        child: FilledButton(
          onPressed: _submitting ? null : _submit,
          child: _submitting
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text(Strings.save),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          LabelledTextField(controller: _name, label: Strings.fieldName),
          const SizedBox(height: Space.s3),
          LabelledTextField(
            controller: _overview,
            label: Strings.fieldOverview,
            maxLines: 3,
          ),
          const Divider(height: Space.s6),
          Text(
            Strings.membersHeading,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: Space.s2),
          if (_members.isEmpty)
            Text(
              Strings.emptyMoviesInCollection,
              style: TextStyle(color: context.tokens.muted),
            )
          else
            Wrap(
              spacing: Space.s2,
              runSpacing: Space.s2,
              children: <Widget>[
                for (final _Member m in _members)
                  InputChip(label: Text(m.title), onDeleted: () => _remove(m)),
              ],
            ),
          const SizedBox(height: Space.s4),
          Text(
            Strings.addMembers,
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: Space.s2),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: <Widget>[
              Expanded(
                child: LabelledTextField(
                  controller: _query,
                  label: Strings.fieldSearchQuery,
                  onSubmitted: (_) => _search(),
                ),
              ),
              const SizedBox(width: Space.s3),
              FilledButton(
                onPressed: _search,
                style: kFieldButtonStyle,
                child: const Text(Strings.searchAction),
              ),
            ],
          ),
          if (_results != null) ...<Widget>[
            const SizedBox(height: Space.s3),
            _searchResults(),
          ],
          if (_error != null) ...<Widget>[
            const SizedBox(height: Space.s3),
            Text(_error!, style: TextStyle(color: context.tokens.danger)),
          ],
        ],
      ),
    );
  }

  Widget _searchResults() {
    return FutureBuilder<Paged<CatalogCard>>(
      future: _results,
      builder:
          (BuildContext context, AsyncSnapshot<Paged<CatalogCard>> snapshot) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const SkeletonLines(lines: 2, padded: false);
            }
            final List<CatalogCard> results =
                snapshot.data?.items ?? const <CatalogCard>[];
            if (snapshot.hasError || results.isEmpty) {
              return Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.s2,
                  vertical: Space.s2,
                ),
                child: Text(
                  Strings.noResultsFound,
                  style: TextStyle(color: context.tokens.muted),
                ),
              );
            }
            return ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 220),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: results.length,
                itemBuilder: (BuildContext context, int i) => MatchRow(
                  label: catalogCardLabel(results[i]),
                  tooltip: Strings.addToCollection,
                  onTap: () => _add(results[i]),
                ),
              ),
            );
          },
    );
  }
}
