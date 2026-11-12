import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/api/playlist_api.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/form_dialog.dart';
import 'package:shadowmask/components/admin/labelled_field.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/playlist/playlist.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/failure_reason.dart';

Future<String?> showPlaylistNameDialog(
  BuildContext context, {
  required String title,
  required String submitLabel,
  String initial = '',
}) => showDialog<String>(
  context: context,
  builder: (BuildContext _) => _PlaylistNameDialog(
    title: title,
    submitLabel: submitLabel,
    initial: initial,
  ),
);

class _PlaylistNameDialog extends StatefulWidget {
  const _PlaylistNameDialog({
    required this.title,
    required this.submitLabel,
    required this.initial,
  });

  final String title;
  final String submitLabel;
  final String initial;

  @override
  State<_PlaylistNameDialog> createState() => _PlaylistNameDialogState();
}

class _PlaylistNameDialogState extends State<_PlaylistNameDialog> {
  late final TextEditingController _name = TextEditingController(
    text: widget.initial,
  );

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  String get _trimmed => _name.text.trim();

  void _submit() {
    if (_trimmed.isNotEmpty) {
      Navigator.of(context).pop(_trimmed);
    }
  }

  @override
  Widget build(BuildContext context) => FormDialog(
    title: widget.title,
    submitLabel: widget.submitLabel,
    onSubmit: _trimmed.isEmpty ? null : _submit,
    child: LabelledTextField(
      controller: _name,
      label: Strings.playlistName,
      onChanged: (String _) => setState(() {}),
      onSubmitted: (String _) => _submit(),
    ),
  );
}

Future<bool> addToPlaylist(
  BuildContext context, {
  required PlaylistApi playlists,
  required String userId,
  required TitleRef title,
}) async {
  final (Playlist, bool)? picked = await showDialog<(Playlist, bool)>(
    context: context,
    builder: (BuildContext _) =>
        _PlaylistPicker(playlists: playlists, userId: userId),
  );
  if (picked == null || !context.mounted) {
    return false;
  }
  final (Playlist chosen, bool made) = picked;
  try {
    final int added = await playlists.add(userId, chosen.id, title);
    if (context.mounted) {
      Toasts.of(context).success(
        added == 0
            ? Strings.toastAlreadyInPlaylist(chosen.name)
            : Strings.toastAddedToPlaylist(added, chosen.name),
        emphasis: chosen.name,
      );
    }
    return true;
  } catch (e) {
    if (context.mounted) {
      Toasts.of(context).error(failureText(Strings.errorAction, e));
    }
    return made;
  }
}

class _PlaylistPicker extends StatefulWidget {
  const _PlaylistPicker({required this.playlists, required this.userId});

  final PlaylistApi playlists;
  final String userId;

  @override
  State<_PlaylistPicker> createState() => _PlaylistPickerState();
}

class _PlaylistPickerState extends State<_PlaylistPicker> {
  late final Future<List<Playlist>> _future = widget.playlists.playlists(
    widget.userId,
  );
  bool _creating = false;

  Future<void> _create() async {
    final String? name = await showPlaylistNameDialog(
      context,
      title: Strings.newPlaylist,
      submitLabel: Strings.create,
    );
    if (name == null || !mounted) {
      return;
    }
    setState(() => _creating = true);
    try {
      final Playlist made = await widget.playlists.create(widget.userId, name);
      if (mounted) {
        Navigator.of(context).pop((made, true));
      }
    } catch (e) {
      if (mounted) {
        setState(() => _creating = false);
        Toasts.of(context).error(failureText(Strings.errorAction, e));
      }
    }
  }

  @override
  Widget build(BuildContext context) => DialogShell(
    title: Strings.addToPlaylist,
    footer: Align(
      alignment: Alignment.centerLeft,
      child: TextButton.icon(
        onPressed: _creating ? null : _create,
        icon: const Icon(Icons.add, size: 18),
        label: const Text(Strings.newPlaylistEllipsis),
      ),
    ),
    child: FutureBuilder<List<Playlist>>(
      future: _future,
      builder: (BuildContext context, AsyncSnapshot<List<Playlist>> snapshot) {
        if (snapshot.hasError) {
          return Text(
            Strings.couldNotLoadPlaylists,
            style: TextStyle(color: context.tokens.muted),
          );
        }
        final List<Playlist>? found = snapshot.data;
        if (found == null) {
          return const Center(
            child: Padding(
              padding: EdgeInsets.all(Space.s4),
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
          );
        }
        if (found.isEmpty) {
          return Text(
            Strings.playlistsEmpty,
            style: TextStyle(color: context.tokens.muted),
          );
        }
        return Material(
          type: MaterialType.transparency,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              for (final Playlist p in found)
                ListTile(
                  leading: const Icon(Icons.playlist_add),
                  title: Text(p.name),
                  subtitle: Text(Strings.playlistItemCount(p.itemCount)),
                  enabled: !_creating,
                  onTap: () => Navigator.of(context).pop((p, false)),
                ),
            ],
          ),
        );
      },
    ),
  );
}
