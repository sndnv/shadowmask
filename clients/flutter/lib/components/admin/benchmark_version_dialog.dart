import 'package:flutter/material.dart';

import 'package:shadowmask/api/admin_api.dart';
import 'package:shadowmask/components/action_field.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/match_row.dart';
import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/common/quality.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/page.dart';

const int kBenchmarkSearchLimit = 10;

Future<Version?> showBenchmarkVersionDialog(
  BuildContext context, {
  required AdminApi admin,
}) => showDialog<Version>(
  context: context,
  builder: (BuildContext _) => BenchmarkVersionDialog(admin: admin),
);

String benchmarkVersionLabel(Version v) =>
    '${v.quality.label} · ${v.container} · ${v.path ?? v.id}';

class BenchmarkVersionDialog extends StatefulWidget {
  const BenchmarkVersionDialog({super.key, required this.admin});

  final AdminApi admin;

  @override
  State<BenchmarkVersionDialog> createState() => _BenchmarkVersionDialogState();
}

class _BenchmarkVersionDialogState extends State<BenchmarkVersionDialog> {
  final TextEditingController _query = TextEditingController();
  List<Version> _matches = const <Version>[];
  bool _searching = false;
  String? _error;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  Future<void> _search() async {
    final String q = _query.text.trim();
    if (q.isEmpty) {
      setState(() => _error = Strings.requiredVersionSearch);
      return;
    }
    setState(() {
      _error = null;
      _searching = true;
      _matches = const <Version>[];
    });
    try {
      final Paged<Version> page = await widget.admin.versions(
        filter: q,
        limit: kBenchmarkSearchLimit,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _searching = false;
        _matches = page.items;
        _error = page.items.isEmpty ? Strings.noMatchingVersions : null;
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

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final String? error = _error;
    return DialogShell(
      title: Strings.chooseVersionHeading,
      width: 640,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          ActionField(
            controller: _query,
            enabled: !_searching,
            autofocus: true,
            label: Strings.searchLabel,
            hintText: Strings.filterVersions,
            actionIcon: Icons.search,
            actionTooltip: Strings.searchLabel,
            onSubmitted: _search,
          ),
          if (_searching) ...<Widget>[
            const SizedBox(height: Space.s3),
            const SkeletonLines(lines: 2, padded: false),
          ] else if (_matches.isNotEmpty) ...<Widget>[
            const SizedBox(height: Space.s3),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 320),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: _matches.length,
                itemBuilder: (BuildContext context, int i) => MatchRow(
                  label: benchmarkVersionLabel(_matches[i]),
                  tooltip: Strings.benchmarkAction,
                  onTap: () => Navigator.of(context).pop(_matches[i]),
                ),
              ),
            ),
          ],
          if (error != null) ...<Widget>[
            const SizedBox(height: Space.s2),
            Text(error, style: TextStyle(color: t.danger, fontSize: 12)),
          ],
        ],
      ),
    );
  }
}
