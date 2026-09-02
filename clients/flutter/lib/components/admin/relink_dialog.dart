import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/admin_field_row.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/match_row.dart';
import 'package:shadowmask/components/action_field.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/segmented_tabs.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/catalog_card.dart';
import 'package:shadowmask/view/failure_reason.dart';
import 'package:shadowmask/view/page.dart';

const String kRelinkProvider = 'tmdb';

enum RelinkMode { move, provider }

Future<bool> showRelinkDialog(
  BuildContext context, {
  required AdminApi admin,
  required CatalogApi catalog,
  required List<String> versionIds,
}) async {
  final bool? done = await showDialog<bool>(
    context: context,
    builder: (BuildContext _) =>
        RelinkDialog(admin: admin, catalog: catalog, versionIds: versionIds),
  );
  return done ?? false;
}

class RelinkDialog extends StatefulWidget {
  const RelinkDialog({
    super.key,
    required this.admin,
    required this.catalog,
    required this.versionIds,
  });

  final AdminApi admin;
  final CatalogApi catalog;
  final List<String> versionIds;

  @override
  State<RelinkDialog> createState() => _RelinkDialogState();
}

class _RelinkDialogState extends State<RelinkDialog> {
  final TextEditingController _query = TextEditingController();
  final TextEditingController _value = TextEditingController();
  RelinkMode _mode = RelinkMode.provider;
  String _type = 'movie';
  List<CatalogCard> _matches = const <CatalogCard>[];
  bool _searching = false;
  String? _error;
  bool _submitting = false;

  @override
  void dispose() {
    _query.dispose();
    _value.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    final String q = _query.text.trim();
    if (q.isEmpty) {
      setState(() => _error = Strings.requiredSearch);
      return;
    }
    setState(() {
      _error = null;
      _searching = true;
      _matches = const <CatalogCard>[];
    });
    try {
      final Paged<CatalogCard> page = await widget.catalog.search(
        q: q,
        type: _type,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _searching = false;
        _matches = page.items;
        _error = page.items.isEmpty ? Strings.noMatches : null;
      });
    } catch (_) {
      if (!mounted) {
        return;
      }
      setState(() {
        _searching = false;
        _error = Strings.searchFailed;
      });
    }
  }

  Future<void> _move(
    Map<String, dynamic> target, {
    required String heading,
    required String question,
    required String confirmLabel,
  }) async {
    final bool ok = await confirmDialog(
      context,
      title: heading,
      message: question,
      confirmLabel: confirmLabel,
      danger: false,
    );
    if (!ok || !mounted) {
      return;
    }
    setState(() => _submitting = true);
    try {
      for (final String id in widget.versionIds) {
        await widget.admin.relink(id, target);
      }
      if (mounted) {
        Toasts.of(context).success(Strings.toastRelinkQueued);
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorRelink, e));
        setState(() => _submitting = false);
      }
    }
  }

  void _moveToProvider() {
    final String value = _value.text.trim();
    if (value.isEmpty) {
      setState(() => _error = Strings.requiredTmdbId);
      return;
    }
    setState(() => _error = null);
    _move(
      <String, dynamic>{
        'kind': 'provider',
        'source': kRelinkProvider,
        'value': value,
      },
      heading: Strings.confirmRelinkHeading,
      question: Strings.confirmRelinkBody(value),
      confirmLabel: Strings.relink,
    );
  }

  void _switchMode(RelinkMode mode) {
    if (mode == _mode) {
      return;
    }
    setState(() {
      _mode = mode;
      _error = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return DialogShell(
      title: Strings.relink,
      enableClose: !_submitting,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          SegmentedTabs<RelinkMode>(
            current: _mode,
            tabs: const <(RelinkMode, String)>[
              (RelinkMode.provider, Strings.tabRelink),
              (RelinkMode.move, Strings.tabMove),
            ],
            onChanged: _switchMode,
          ),
          const SizedBox(height: Space.s4),
          if (_mode == RelinkMode.move) ..._moveTab() else ..._relinkTab(),
          if (_error != null) ...<Widget>[
            const SizedBox(height: Space.s2),
            Text(_error!, style: TextStyle(color: t.danger, fontSize: 12)),
          ],
        ],
      ),
    );
  }

  List<Widget> _moveTab() => <Widget>[
    AdminFieldRow(
      fields: <AdminField>[
        AdminField(
          flex: 1,
          ActionField(
            controller: _query,
            enabled: !_submitting,
            autofocus: true,
            label: Strings.searchLabel,
            hintText: Strings.relinkSearchLabel,
            actionIcon: Icons.search,
            actionTooltip: Strings.searchLabel,
            onSubmitted: _search,
          ),
        ),
        AdminField(
          width: 130,
          AppDropdown<String>(
            value: _type,
            label: Strings.searchTypeLabel,
            items: const <(String, String)>[
              ('movie', Strings.typeMovie),
              ('episode', Strings.typeEpisode),
            ],
            onChanged: (String v) {
              setState(() => _type = v);
              _search();
            },
          ),
        ),
      ],
    ),
    if (_searching) ...<Widget>[
      const SizedBox(height: Space.s3),
      const SkeletonLines(lines: 2, padded: false),
    ] else if (_matches.isNotEmpty) ...<Widget>[
      const SizedBox(height: Space.s3),
      ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 220),
        child: ListView.builder(
          shrinkWrap: true,
          itemCount: _matches.length,
          itemBuilder: (BuildContext context, int i) => MatchRow(
            label: catalogCardLabel(_matches[i]),
            tooltip: Strings.moveHere,
            enabled: !_submitting,
            onTap: () => _move(
              <String, dynamic>{
                'kind': 'existing',
                'title': <String, dynamic>{
                  'type': _matches[i].ref.type.wire,
                  'id': _matches[i].ref.id,
                },
              },
              heading: Strings.confirmMoveHeading,
              question: Strings.confirmMoveBody(catalogCardLabel(_matches[i])),
              confirmLabel: Strings.tabMove,
            ),
          ),
        ),
      ),
    ],
  ];

  List<Widget> _relinkTab() => <Widget>[
    ActionField(
      controller: _value,
      enabled: !_submitting,
      autofocus: true,
      label: Strings.fieldTmdbId,
      hintText: Strings.relinkProviderHint,
      actionIcon: Icons.link,
      actionTooltip: Strings.relink,
      onSubmitted: _moveToProvider,
    ),
  ];
}
