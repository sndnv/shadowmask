import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/setting_help.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_size.dart';

class PlaybackBlock extends StatefulWidget {
  const PlaybackBlock({super.key, this.store = const PlayerPrefsStore()});

  final PlayerPrefsStore store;

  @override
  State<PlaybackBlock> createState() => _PlaybackBlockState();
}

class _PlaybackBlockState extends State<PlaybackBlock> {
  DeliveryPreference _delivery = const LocalSettings().defaultDelivery;
  bool _downmix = const LocalSettings().defaultDownmix;
  bool _normalize = const LocalSettings().defaultNormalize;
  DialogueLift _dialogue = const LocalSettings().defaultDialogue;
  NightMode _night = const LocalSettings().defaultNight;
  SubtitleSize _subtitleSize = const LocalSettings().subtitleSize;
  int _autoplaySeconds = const LocalSettings().autoplayNextS;
  bool _diagnostics = const LocalSettings().diagnostics;
  bool _loaded = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final LocalSettings prefs = await widget.store.load().catchError(
      (Object _) => const LocalSettings(),
    );
    if (!mounted) {
      return;
    }
    setState(() {
      _delivery = prefs.defaultDelivery;
      _downmix = prefs.defaultDownmix;
      _normalize = prefs.defaultNormalize;
      _dialogue = prefs.defaultDialogue;
      _night = prefs.defaultNight;
      _subtitleSize = prefs.subtitleSize;
      _autoplaySeconds = prefs.autoplayNextS;
      _diagnostics = prefs.diagnostics;
      _loaded = true;
    });
  }

  void _setDelivery(DeliveryPreference delivery) {
    setState(() => _delivery = delivery);
    widget.store.saveDelivery(delivery);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setDownmix(bool on) {
    setState(() => _downmix = on);
    widget.store.saveDownmix(on);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setNormalize(bool on) {
    setState(() => _normalize = on);
    widget.store.saveNormalize(on);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setDialogue(DialogueLift lift) {
    setState(() => _dialogue = lift);
    widget.store.saveDialogue(lift);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setNight(NightMode night) {
    setState(() => _night = night);
    widget.store.saveNight(night);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setSubtitleSize(SubtitleSize size) {
    setState(() => _subtitleSize = size);
    widget.store.saveSubtitleSize(size);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setAutoplay(int seconds) {
    setState(() => _autoplaySeconds = seconds);
    widget.store.saveAutoplaySeconds(seconds);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  void _setDiagnostics(bool on) {
    setState(() => _diagnostics = on);
    widget.store.saveDiagnostics(on);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  Widget _switchRow(
    String label,
    String help,
    bool value,
    ValueChanged<bool> onChanged,
  ) => Material(
    color: Colors.transparent,
    child: SwitchListTile(
      contentPadding: kSettingRowPadding,
      minVerticalPadding: kSettingRowVerticalPadding,
      minTileHeight: kSettingRowMinHeight,
      value: value,
      onChanged: _loaded ? onChanged : null,
      title: Text(label, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: SettingHelp(help),
    ),
  );

  Widget _choiceRow<T>(
    String label,
    String help,
    T value,
    List<(T, String)> items,
    ValueChanged<T> onChanged,
  ) => Material(
    color: Colors.transparent,
    child: ListTile(
      contentPadding: kSettingRowPadding,
      minVerticalPadding: kSettingRowVerticalPadding,
      minTileHeight: kSettingRowMinHeight,
      title: Text(label, style: const TextStyle(fontWeight: FontWeight.w600)),
      subtitle: SettingHelp(help),
      trailing: AppDropdown<T>(
        value: value,
        label: label,
        enabled: _loaded,
        items: items,
        onChanged: onChanged,
      ),
    ),
  );

  Widget _section(String title, Tokens t, List<Widget> rows) => SectionBlock(
    title: title,
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (int i = 0; i < rows.length; i++) ...<Widget>[
          if (i > 0) Divider(color: t.border, height: 1),
          rows[i],
        ],
      ],
    ),
  );

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _section(Strings.accountPlaybackHeading, t, <Widget>[
          _choiceRow<int>(
            Strings.playerAutoplayNext,
            Strings.playerAutoplayNextHelp,
            _autoplaySeconds,
            <(int, String)>[
              for (final int value in kAutoplayDelays)
                (value, autoplayLabel(value)),
            ],
            _setAutoplay,
          ),
          _switchRow(
            Strings.playerDiagnostics,
            Strings.playerDiagnosticsHelp,
            _diagnostics,
            _setDiagnostics,
          ),
        ]),
        _section(Strings.videoHeading, t, <Widget>[
          _choiceRow<DeliveryPreference>(
            Strings.playerDelivery,
            Strings.playerDeliveryHelp,
            _delivery,
            const <(DeliveryPreference, String)>[
              (DeliveryPreference.auto, Strings.playerDeliveryAuto),
              (DeliveryPreference.never, Strings.playerDeliveryNever),
              (DeliveryPreference.always, Strings.playerDeliveryAlways),
            ],
            _setDelivery,
          ),
        ]),
        _section(Strings.subtitlesHeading, t, <Widget>[
          _choiceRow<SubtitleSize>(
            Strings.subtitleSize,
            Strings.subtitleSizeHelp,
            _subtitleSize,
            <(SubtitleSize, String)>[
              for (final SubtitleSize size in SubtitleSize.values)
                (size, size.label),
            ],
            _setSubtitleSize,
          ),
        ]),
        _section(Strings.soundHeading, t, <Widget>[
          _switchRow(
            Strings.playerStereoDownmix,
            Strings.playerStereoDownmixHelp,
            _downmix,
            _setDownmix,
          ),
          _switchRow(
            Strings.playerPreventDistortion,
            Strings.playerPreventDistortionHelp,
            _normalize,
            _setNormalize,
          ),
          _choiceRow<DialogueLift>(
            Strings.playerDialogue,
            Strings.playerDialogueHelp,
            _dialogue,
            <(DialogueLift, String)>[
              for (final DialogueLift lift in DialogueLift.values)
                (lift, lift.label),
            ],
            _setDialogue,
          ),
          _choiceRow<NightMode>(
            Strings.playerNightMode,
            Strings.playerNightModeHelp,
            _night,
            <(NightMode, String)>[
              for (final NightMode mode in NightMode.values) (mode, mode.label),
            ],
            _setNight,
          ),
        ]),
      ],
    );
  }
}
