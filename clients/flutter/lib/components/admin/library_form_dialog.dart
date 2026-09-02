import 'package:flutter/material.dart';

import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/components/admin/admin_field_row.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/admin/scan_schedule_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/util/library_labels.dart';
import 'package:shadowmask/view/failure_reason.dart';

const List<String> _defaultArticles = <String>['the', 'a', 'an'];

Future<bool> showLibraryFormDialog(
  BuildContext context, {
  required LibraryApi libraries,
  Library? existing,
}) async {
  final bool? saved = await showDialog<bool>(
    context: context,
    builder: (BuildContext _) =>
        LibraryFormDialog(libraries: libraries, existing: existing),
  );
  return saved ?? false;
}

class LibraryFormDialog extends StatefulWidget {
  const LibraryFormDialog({super.key, required this.libraries, this.existing});

  final LibraryApi libraries;
  final Library? existing;

  @override
  State<LibraryFormDialog> createState() => _LibraryFormDialogState();
}

class _LibraryFormDialogState extends State<LibraryFormDialog> {
  late final TextEditingController _name = TextEditingController(
    text: widget.existing?.name ?? '',
  );
  late final TextEditingController _roots = TextEditingController(
    text: widget.existing?.roots.join(', ') ?? '',
  );
  late String _schedule = widget.existing?.scanSchedule ?? '';
  late final TextEditingController _sources = TextEditingController(
    text: widget.existing?.metadataSources.join(', ') ?? '',
  );
  late final TextEditingController _articles = TextEditingController(
    text: (widget.existing?.sortArticles ?? _defaultArticles).join(', '),
  );
  late LibraryKind _kind = widget.existing?.kind ?? LibraryKind.movie;
  late LibraryOrigin _origin = widget.existing?.origin ?? LibraryOrigin.local;
  late WatcherStrategy _watcher =
      widget.existing?.watcher ?? WatcherStrategy.local;
  bool _submitting = false;
  String? _nameError;

  void _clearNameError(String _) {
    if (_nameError != null) {
      setState(() => _nameError = null);
    }
  }

  @override
  void dispose() {
    _name.dispose();
    _roots.dispose();
    _sources.dispose();
    _articles.dispose();
    super.dispose();
  }

  List<String> _csv(String raw) => raw
      .split(',')
      .map((String s) => s.trim())
      .where((String s) => s.isNotEmpty)
      .toList();

  Future<void> _submit() async {
    if (_name.text.trim().isEmpty) {
      setState(() => _nameError = Strings.requiredName);
      return;
    }
    final String? schedule = _schedule.isEmpty ? null : _schedule;
    final Map<String, dynamic> body = <String, dynamic>{
      'name': _name.text.trim(),
      'kind': libraryKindWire(_kind),
      'roots': _csv(_roots.text),
      'watcher': watcherWire(_watcher),
      'scan_schedule': schedule,
      'metadata_sources': _csv(_sources.text),
      'sort_articles': _csv(_articles.text),
    };
    final Library? existing = widget.existing;
    if (existing == null) {
      body['origin'] = libraryOriginWire(_origin);
    }
    setState(() => _submitting = true);
    try {
      if (existing == null) {
        await widget.libraries.createLibrary(body);
      } else {
        await widget.libraries.updateLibrary(existing.id, body);
      }
      if (mounted) {
        Toasts.of(context).success(Strings.toastLibrarySaved);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorSave, e));
        setState(() => _submitting = false);
      }
    }
  }

  Widget _pair(Widget left, Widget right) => AdminFieldRow(
    crossAxisAlignment: CrossAxisAlignment.start,
    fields: <AdminField>[AdminField(flex: 1, left), AdminField(flex: 1, right)],
  );

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: widget.existing == null
          ? Strings.createLibrary
          : Strings.editLibrary,
      submitting: _submitting,
      onSubmit: _submit,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        spacing: Space.s3,
        children: <Widget>[
          LabelledTextField(
            controller: _name,
            label: Strings.fieldName,
            error: _nameError,
            onChanged: _clearNameError,
          ),
          _pair(
            LabelledDropdown<LibraryKind>(
              label: Strings.fieldKind,
              help: Strings.kindHelp,
              value: _kind,
              items: <(LibraryKind, String)>[
                for (final LibraryKind k in LibraryKind.values)
                  (k, libraryKindLabel(k)),
              ],
              onChanged: (LibraryKind v) => setState(() => _kind = v),
            ),
            LabelledDropdown<LibraryOrigin>(
              label: Strings.fieldOrigin,
              help: Strings.originHelp,
              value: _origin,
              enabled: widget.existing == null,
              items: <(LibraryOrigin, String)>[
                for (final LibraryOrigin o in LibraryOrigin.values)
                  (o, libraryOriginLabel(o)),
              ],
              onChanged: (LibraryOrigin v) => setState(() => _origin = v),
            ),
          ),
          _pair(
            LabelledDropdown<WatcherStrategy>(
              label: Strings.fieldWatcher,
              help: Strings.watcherHelp,
              value: _watcher,
              items: <(WatcherStrategy, String)>[
                for (final WatcherStrategy w in WatcherStrategy.values)
                  (w, watcherLabel(w)),
              ],
              onChanged: (WatcherStrategy v) => setState(() => _watcher = v),
            ),
            ScanScheduleField(
              value: _schedule,
              enabled: !_submitting,
              onChanged: (String v) => _schedule = v,
            ),
          ),
          LabelledTextField(
            controller: _roots,
            label: Strings.fieldRoots,
            help: Strings.rootsHelp,
            hint: Strings.commaSeparatedPaths,
          ),
          LabelledTextField(
            controller: _sources,
            label: Strings.fieldMetadataSources,
            help: Strings.metadataSourcesHelp,
            hint: Strings.commaSeparatedSources,
          ),
          LabelledTextField(
            controller: _articles,
            label: Strings.fieldSortArticles,
            help: Strings.sortArticlesHelp,
            hint: Strings.commaSeparatedArticles,
          ),
        ],
      ),
    );
  }
}
