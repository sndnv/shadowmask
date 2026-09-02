import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/admin/admin_field_row.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/language_dropdown.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/subtitle_candidate.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/app_button.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/subtitle_labels.dart';
import 'package:shadowmask/view/failure_reason.dart';

const double _kSpinner = 16;

Future<bool> showSubtitleSearch(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required List<SubtitleFile> existing,
}) async =>
    await showDialog<bool>(
      context: context,
      builder: (BuildContext _) => _SubtitleSearchDialog(
        admin: admin,
        versionId: versionId,
        existing: existing,
      ),
    ) ??
    false;

Future<bool> renameSubtitleFile(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required SubtitleFile sub,
}) async {
  final String? language = await showDialog<String>(
    context: context,
    builder: (BuildContext _) => _RenameSubtitleDialog(initial: sub.language),
  );
  if (language == null || !context.mounted) {
    return false;
  }
  try {
    await admin.renameSubtitle(
      versionId,
      sub.id,
      language: language.isEmpty ? null : language,
    );
    if (context.mounted) {
      Toasts.of(context).success(Strings.toastRenamed);
    }
    return true;
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorRename, e));
    }
    return false;
  }
}

Future<void> showSubtitleText(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required SubtitleFile sub,
}) => showDialog<void>(
  context: context,
  builder: (BuildContext _) =>
      _SubtitleTextDialog(admin: admin, versionId: versionId, sub: sub),
);

Future<bool> deleteSubtitleFile(
  BuildContext context, {
  required AdminApi admin,
  required String versionId,
  required SubtitleFile sub,
}) async {
  final bool ok = await confirmDialog(
    context,
    title: Strings.deleteSubtitle,
    message: Strings.confirmDeleteSubtitle(subtitleFileLabel(sub)),
  );
  if (!ok || !context.mounted) {
    return false;
  }
  try {
    await admin.deleteSubtitle(versionId, sub.id);
    if (context.mounted) {
      Toasts.of(context).success(Strings.toastDeleted);
    }
    return true;
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorDelete, e));
    }
    return false;
  }
}

class _SubtitleTextDialog extends StatefulWidget {
  const _SubtitleTextDialog({
    required this.admin,
    required this.versionId,
    required this.sub,
  });

  final AdminApi admin;
  final String versionId;
  final SubtitleFile sub;

  @override
  State<_SubtitleTextDialog> createState() => _SubtitleTextDialogState();
}

class _SubtitleTextDialogState extends State<_SubtitleTextDialog> {
  late final Future<String> _text = widget.admin.subtitleText(
    widget.versionId,
    widget.sub.id,
  );

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: subtitleFileLabel(widget.sub),
      width: 640,
      child: FutureBuilder<String>(
        future: _text,
        builder: (BuildContext context, AsyncSnapshot<String> snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Padding(
              padding: EdgeInsets.all(Space.s4),
              child: Center(child: CircularProgressIndicator()),
            );
          }
          if (snapshot.hasError) {
            return Text(
              failureText(Strings.errorSubtitleText, snapshot.error!),
              style: TextStyle(color: context.tokens.danger),
            );
          }
          final String content = snapshot.data?.trim() ?? '';
          if (content.isEmpty) {
            return const StatusText(Strings.emptySubtitleText);
          }
          return Text(
            content,
            style: monoStyle.copyWith(
              color: context.tokens.text,
              fontSize: 12,
              height: 1.5,
            ),
          );
        },
      ),
    );
  }
}

class _SubtitleSearchDialog extends StatefulWidget {
  const _SubtitleSearchDialog({
    required this.admin,
    required this.versionId,
    required this.existing,
  });

  final AdminApi admin;
  final String versionId;
  final List<SubtitleFile> existing;

  @override
  State<_SubtitleSearchDialog> createState() => _SubtitleSearchDialogState();
}

class _SubtitleSearchDialogState extends State<_SubtitleSearchDialog> {
  final TextEditingController _query = TextEditingController();
  String _language = '';
  Future<List<SubtitleCandidate>>? _results;
  bool _downloaded = false;
  final Set<String> _pending = <String>{};
  late final Set<String> _held = <String>{
    for (final SubtitleFile f in widget.existing) f.id,
  };

  String _candidateId(SubtitleCandidate c) =>
      'opensubtitles:${widget.versionId}:${c.fileId}';

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  void _search() {
    setState(() {
      _results = widget.admin.subtitleSearch(
        widget.versionId,
        query: _query.text.trim(),
        language: _language,
      );
    });
  }

  Future<void> _download(SubtitleCandidate c) async {
    final String id = _candidateId(c);
    if (_pending.contains(id)) {
      return;
    }
    setState(() => _pending.add(id));
    try {
      await widget.admin.subtitleDownload(
        widget.versionId,
        fileId: c.fileId,
        language: c.language,
        releaseName: c.releaseName,
      );
      _downloaded = true;
      if (mounted) {
        setState(() {
          _pending.remove(id);
          _held.add(id);
        });
        Toasts.of(context).success(Strings.toastDownloaded);
      }
    } catch (e) {
      if (mounted) {
        setState(() => _pending.remove(id));
        Toasts.of(context).error(failureText(Strings.errorDownload, e));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.searchSubtitles,
      width: 560,
      onClose: () => Navigator.of(context).pop(_downloaded),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          AdminFieldRow(
            fields: <AdminField>[
              AdminField(
                flex: 1,
                LabelledTextField(
                  controller: _query,
                  label: Strings.fieldSearchQuery,
                  help: Strings.subtitleQueryHelp,
                  onSubmitted: (_) => _search(),
                ),
              ),
              AdminField(
                width: 160,
                LanguageDropdown(
                  value: _language,
                  emptyLabel: Strings.optionAnyLanguage,
                  onChanged: (String v) => setState(() => _language = v),
                ),
              ),
              AdminField(
                FilledButton(
                  onPressed: _search,
                  style: kFieldButtonStyle,
                  child: const Text(Strings.searchSubtitles),
                ),
              ),
            ],
          ),
          if (_results != null) ...<Widget>[
            const SizedBox(height: Space.s3),
            _searchResults(),
          ],
        ],
      ),
    );
  }

  Widget _searchResults() {
    return FutureBuilder<List<SubtitleCandidate>>(
      future: _results,
      builder:
          (
            BuildContext context,
            AsyncSnapshot<List<SubtitleCandidate>> snapshot,
          ) {
            if (snapshot.connectionState != ConnectionState.done) {
              return const Padding(
                padding: EdgeInsets.all(Space.s4),
                child: Center(child: CircularProgressIndicator()),
              );
            }
            if (snapshot.hasError) {
              return Text(
                failureText(Strings.errorSubtitleSearch, snapshot.error!),
                style: TextStyle(color: context.tokens.danger),
              );
            }
            final List<SubtitleCandidate> results =
                snapshot.data ?? const <SubtitleCandidate>[];
            if (results.isEmpty) {
              return const StatusText(Strings.emptyCandidates);
            }
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  Strings.searchResultsHeading,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: context.tokens.muted),
                ),
                const SizedBox(height: Space.s2),
                for (final SubtitleCandidate c in results)
                  Padding(
                    padding: const EdgeInsets.only(bottom: Space.s2),
                    child: Row(
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            subtitleCandidateLabel(c),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        if (_held.contains(_candidateId(c)))
                          const TextButton(
                            onPressed: null,
                            child: Text(Strings.downloaded),
                          )
                        else if (_pending.contains(_candidateId(c)))
                          TextButton(
                            onPressed: null,
                            child: Semantics(
                              label: Strings.downloading,
                              child: const SizedBox(
                                width: _kSpinner,
                                height: _kSpinner,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              ),
                            ),
                          )
                        else
                          TextButton(
                            onPressed: () => _download(c),
                            child: const Text(Strings.download),
                          ),
                      ],
                    ),
                  ),
              ],
            );
          },
    );
  }
}

class _RenameSubtitleDialog extends StatefulWidget {
  const _RenameSubtitleDialog({this.initial});

  final String? initial;

  @override
  State<_RenameSubtitleDialog> createState() => _RenameSubtitleDialogState();
}

class _RenameSubtitleDialogState extends State<_RenameSubtitleDialog> {
  late String _language = widget.initial ?? '';

  @override
  Widget build(BuildContext context) {
    return FormDialog(
      title: Strings.rename,
      onSubmit: () => Navigator.of(context).pop(_language),
      child: LanguageDropdown(
        label: Strings.fieldLanguage,
        help: Strings.subtitleLanguageHelp,
        value: _language,
        emptyLabel: Strings.optionNoLanguage,
        onChanged: (String v) => setState(() => _language = v),
      ),
    );
  }
}
