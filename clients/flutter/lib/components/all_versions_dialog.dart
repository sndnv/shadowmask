import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/playback_api.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/version_picker.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version.dart';

class AllVersionsDialog extends StatelessWidget {
  const AllVersionsDialog({
    super.key,
    required this.catalog,
    required this.playback,
    required this.userId,
    required this.versions,
    this.onProgressCleared,
  });

  final CatalogApi catalog;
  final PlaybackApi playback;
  final String userId;
  final List<Version> versions;
  final VoidCallback? onProgressCleared;

  static Future<void> show(
    BuildContext context, {
    required CatalogApi catalog,
    required PlaybackApi playback,
    required String userId,
    required List<Version> versions,
    VoidCallback? onProgressCleared,
  }) => showDialog<void>(
    context: context,
    builder: (BuildContext context) => AllVersionsDialog(
      catalog: catalog,
      playback: playback,
      userId: userId,
      versions: versions,
      onProgressCleared: onProgressCleared,
    ),
  );

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.countLabel(Strings.versionsHeading, versions.length),
      width: 760,
      child: VersionPicker(
        catalog: catalog,
        playback: playback,
        userId: userId,
        versions: versions,
        onProgressCleared: onProgressCleared,
        showHeading: false,
      ),
    );
  }
}
