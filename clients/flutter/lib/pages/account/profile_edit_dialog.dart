import 'package:flutter/material.dart';

import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/language_dropdown.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/server/rating_system.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/view/failure_reason.dart';

const List<int> kStreamOptions = <int>[1, 2, 3, 4, 5, 6, 8, 10];
const List<int> kBitrateOptions = <int>[2, 4, 8, 12, 20, 40];

class ProfileEditDialog extends StatefulWidget {
  const ProfileEditDialog({
    super.key,
    required this.catalog,
    required this.userId,
    required this.profile,
    required this.ratingSystems,
  });

  final CatalogApi catalog;
  final String userId;
  final AccountProfile profile;
  final List<RatingSystem> ratingSystems;

  static Future<bool> show(
    BuildContext context, {
    required CatalogApi catalog,
    required String userId,
    required AccountProfile profile,
    required List<RatingSystem> ratingSystems,
  }) async {
    final bool? saved = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) => ProfileEditDialog(
        catalog: catalog,
        userId: userId,
        profile: profile,
        ratingSystems: ratingSystems,
      ),
    );
    return saved ?? false;
  }

  @override
  State<ProfileEditDialog> createState() => _ProfileEditDialogState();
}

class _ProfileEditDialogState extends State<ProfileEditDialog> {
  late String _audio = widget.profile.preferredAudio.firstOrNull ?? '';
  late String _subtitle = widget.profile.preferredSubtitle.firstOrNull ?? '';
  late String _system = widget.profile.maxContentRating?.system ?? '';
  late String _code = widget.profile.maxContentRating?.code ?? '';
  late int _streams = widget.profile.concurrentStreamLimit ?? 0;
  late int _bitrate = _mbpsOf(widget.profile.bitrateCap);
  bool _saving = false;

  static int _mbpsOf(int? bps) => bps == null ? 0 : (bps / 1000000).round();

  List<String> get _codes => widget.ratingSystems
      .where((RatingSystem s) => s.system == _system)
      .expand((RatingSystem s) => s.codes)
      .toList();

  void _pickSystem(String system) {
    setState(() {
      _system = system;
      _code = '';
    });
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    final Map<String, dynamic> body = <String, dynamic>{
      'preferred_audio': _audio.isEmpty ? <String>[] : <String>[_audio],
      'preferred_subtitle': _subtitle.isEmpty
          ? <String>[]
          : <String>[_subtitle],
      'max_content_rating': _system.isEmpty || _code.isEmpty
          ? null
          : <String, String>{'system': _system, 'code': _code},
      'concurrent_stream_limit': _streams == 0 ? null : _streams,
      'bitrate_cap': _bitrate == 0 ? null : _bitrate * 1000000,
    };
    try {
      await widget.catalog.updateProfile(widget.userId, body);
      if (!mounted) {
        return;
      }
      Toasts.of(context).success(Strings.toastProfileSaved);
      Navigator.of(context).pop(true);
    } catch (e) {
      if (mounted) {
        Toasts.of(context).error(failureText(Strings.errorSave, e));
        setState(() => _saving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return DialogShell(
      title: Strings.editProfile,
      enableClose: !_saving,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        children: <Widget>[
          FilledButton(
            onPressed: _saving ? null : _save,
            child: const Text(Strings.save),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: Space.s4,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(
                child: FieldLabel(
                  label: Strings.fieldPreferredAudio,
                  help: Strings.preferredAudioHelp,
                  child: LanguageDropdown(
                    value: _audio,
                    emptyLabel: Strings.optionNoPreference,
                    enabled: !_saving,
                    onChanged: (String v) => setState(() => _audio = v),
                  ),
                ),
              ),
              const SizedBox(width: Space.s3),
              Expanded(
                child: FieldLabel(
                  label: Strings.fieldPreferredSubtitle,
                  help: Strings.preferredSubtitleHelp,
                  child: LanguageDropdown(
                    value: _subtitle,
                    emptyLabel: Strings.optionNoPreference,
                    enabled: !_saving,
                    onChanged: (String v) => setState(() => _subtitle = v),
                  ),
                ),
              ),
            ],
          ),
          FieldLabel(
            label: Strings.fieldMaximumRating,
            help: Strings.maximumRatingHelp,
            child: Row(
              children: <Widget>[
                Expanded(
                  child: AppDropdown<String>(
                    value: _system,
                    items: <(String, String)>[
                      ('', Strings.optionNoLimit),
                      for (final RatingSystem s in widget.ratingSystems)
                        (s.system, s.system.toUpperCase()),
                    ],
                    onChanged: _saving ? (_) {} : _pickSystem,
                  ),
                ),
                const SizedBox(width: Space.s3),
                Expanded(
                  child: AppDropdown<String>(
                    value: _code,
                    items: <(String, String)>[
                      ('', Strings.optionNoLimit),
                      for (final String code in _codes)
                        (code, code.toUpperCase()),
                    ],
                    onChanged: _saving
                        ? (_) {}
                        : (String v) => setState(() => _code = v),
                  ),
                ),
              ],
            ),
          ),
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(
                child: FieldLabel(
                  label: Strings.fieldConcurrentStreams,
                  help: Strings.concurrentStreamsHelp,
                  child: AppDropdown<int>(
                    value: _streams,
                    items: <(int, String)>[
                      (0, Strings.optionUnlimited),
                      for (final int n in kStreamOptions)
                        (n, Strings.streamCount(n)),
                    ],
                    onChanged: _saving
                        ? (_) {}
                        : (int v) => setState(() => _streams = v),
                  ),
                ),
              ),
              const SizedBox(width: Space.s3),
              Expanded(
                child: FieldLabel(
                  label: Strings.fieldBitrateCap,
                  help: Strings.bitrateCapHelp,
                  child: AppDropdown<int>(
                    value: _bitrate,
                    items: <(int, String)>[
                      (0, Strings.optionUnlimited),
                      for (final int mbps in kBitrateOptions)
                        (mbps, Strings.megabitsPerSecond(mbps)),
                    ],
                    onChanged: _saving
                        ? (_) {}
                        : (int v) => setState(() => _bitrate = v),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
