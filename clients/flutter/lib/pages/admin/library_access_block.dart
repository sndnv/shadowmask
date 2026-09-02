import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/library_api.dart';
import 'package:shadowmask/api/user_admin_api.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/library/library.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/view/failure_reason.dart';

class LibraryAccessBlock extends StatefulWidget {
  const LibraryAccessBlock({
    super.key,
    required this.api,
    required this.userId,
  });

  final ApiClient api;
  final String userId;

  @override
  State<LibraryAccessBlock> createState() => _LibraryAccessBlockState();
}

class _LibraryAccessBlockState extends State<LibraryAccessBlock> {
  late final LibraryApi _libraries = LibraryApi(widget.api);
  late final UserAdminApi _users = UserAdminApi(widget.api);
  late final Future<_AccessData> _future = _load();
  Set<String>? _granted;
  bool _saving = false;

  Future<_AccessData> _load() async {
    final List<Library> libraries = await _libraries.libraries();
    final List<String> granted = await _users.libraryAccess(widget.userId);
    return _AccessData(libraries, granted);
  }

  Future<void> _save() async {
    final Set<String> granted = _granted ?? <String>{};
    setState(() => _saving = true);
    try {
      await _users.setLibraryAccess(widget.userId, granted.toList());
      if (mounted) {
        Toasts.of(context).success(Strings.toastAccessSaved);
      }
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorSave, e));
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return SectionBlock(
      title: Strings.libraryAccess,
      actionItems: <PageAction>[
        PageAction(
          icon: Icons.save_outlined,
          label: Strings.save,
          primary: true,
          onPressed: _saving ? null : _save,
        ),
      ],
      child: buildBlock<_AccessData>(
        future: _future,
        errorText: Strings.couldNotLoadLibraries,
        builder: (BuildContext context, _AccessData data) {
          if (data.libraries.isEmpty) {
            return const StatusText(Strings.emptyLibraries);
          }
          final Set<String> granted = _granted ??= data.granted.toSet();
          return Material(
            type: MaterialType.transparency,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                for (final Library l in data.libraries)
                  CheckboxListTile(
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    value: granted.contains(l.id),
                    title: Text(l.name),
                    onChanged: (bool? v) => setState(() {
                      if (v ?? false) {
                        granted.add(l.id);
                      } else {
                        granted.remove(l.id);
                      }
                    }),
                  ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _AccessData {
  const _AccessData(this.libraries, this.granted);

  final List<Library> libraries;
  final List<String> granted;
}
