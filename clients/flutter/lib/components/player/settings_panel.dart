import 'package:flutter/material.dart';

import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/languages.dart';

const List<int?> _qualityRungs = <int?>[null, 1080, 720, 480, 320];
const List<double> _speeds = <double>[0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const double _kControlColumn = 168;
const double _kDenseControlColumn = 132;
const double _kDenseRowPad = 3;

const Object kPlayerPanelGroup = Object();

enum PlayerPanel {
  video(Strings.playerVideo, Icons.video_settings),
  audio(Strings.playerAudio, Icons.audiotrack),
  subtitles(Strings.playerSubtitles, Icons.subtitles),
  settings(Strings.playerSettings, Icons.settings);

  const PlayerPanel(this.title, this.icon);

  final String title;
  final IconData icon;
}

class PlayerSettingsPanel extends StatefulWidget {
  const PlayerSettingsPanel({
    super.key,
    required this.panel,
    required this.controls,
    required this.version,
    required this.speed,
    required this.diagnostics,
    required this.onControls,
    required this.onSpeed,
    required this.onDiagnostics,
    required this.onClose,
    this.mode,
    this.subtitleDelivery,
    this.pictureInPicture,
    this.onPictureInPicture,
    this.autoplaySeconds,
    this.onAutoplaySeconds,
    this.onShortcuts,
    this.dense = false,
  });

  final PlayerPanel panel;
  final PlaybackControls controls;
  final VersionDetail version;
  final double speed;
  final bool diagnostics;
  final String? mode;
  final String? subtitleDelivery;
  final bool? pictureInPicture;
  final VoidCallback? onPictureInPicture;
  final int? autoplaySeconds;
  final ValueChanged<int>? onAutoplaySeconds;
  final VoidCallback? onShortcuts;
  final ValueChanged<PlaybackControls> onControls;
  final ValueChanged<double> onSpeed;
  final ValueChanged<bool> onDiagnostics;
  final VoidCallback onClose;
  final bool dense;

  @override
  State<PlayerSettingsPanel> createState() => _PlayerSettingsPanelState();
}

class _PlayerSettingsPanelState extends State<PlayerSettingsPanel> {
  late int? _height = widget.controls.height;
  late int? _audio = widget.controls.audioTrack;
  late String _sub = widget.controls.subtitle?.toWire() ?? '';
  late final TextEditingController _offset = TextEditingController(
    text: widget.controls.offsetMs == 0
        ? ''
        : widget.controls.offsetMs.toString(),
  );
  late bool _burn = widget.controls.burn;
  late bool _downmix = widget.controls.downmix;
  late double _speed = widget.speed;
  late bool _diag = widget.diagnostics;
  late int _autoplay =
      widget.autoplaySeconds ?? kDefaultPlayerPrefs.autoplaySeconds;

  @override
  void dispose() {
    _offset.dispose();
    super.dispose();
  }

  void _apply() {
    widget.onControls(
      PlaybackControls(
        height: _height,
        burn: _burn,
        downmix: _downmix,
        audioTrack: _audio,
        subtitle: SubtitleSelection.fromWire(_sub.isEmpty ? null : _sub),
        offsetMs: int.tryParse(_offset.text) ?? 0,
      ),
    );
  }

  List<Widget> _rows(BuildContext context, Tokens t) {
    final VersionDetail v = widget.version;
    final bool hasSubs = v.subtitles.isNotEmpty || v.subtitleFiles.isNotEmpty;
    final bool burnedIn = widget.subtitleDelivery == 'burned';
    switch (widget.panel) {
      case PlayerPanel.video:
        return <Widget>[
          _row(
            Strings.playerQuality,
            AppDropdown<int?>(
              value: _height,
              width: double.infinity,
              tapGroupId: kPlayerPanelGroup,
              items: <(int?, String)>[
                for (final int? rung in _qualityRungs)
                  (
                    rung,
                    rung == null
                        ? Strings.playerOriginal
                        : Strings.qualityRung(rung),
                  ),
              ],
              onChanged: (int? value) {
                setState(() => _height = value);
                _apply();
              },
            ),
          ),
          _row(
            Strings.playerSpeed,
            AppDropdown<double>(
              value: _speed,
              width: double.infinity,
              tapGroupId: kPlayerPanelGroup,
              items: <(double, String)>[
                for (final double rate in _speeds)
                  (rate, rate == 1.0 ? Strings.playerNormalSpeed : '${rate}x'),
              ],
              onChanged: (double value) {
                setState(() => _speed = value);
                widget.onSpeed(value);
              },
            ),
          ),
        ];
      case PlayerPanel.audio:
        return <Widget>[
          _row(
            Strings.playerAudio,
            AppDropdown<int?>(
              value: _audio,
              width: double.infinity,
              tapGroupId: kPlayerPanelGroup,
              items: v.audio.isEmpty
                  ? <(int?, String)>[(null, Strings.playerNoAudio)]
                  : <(int?, String)>[
                      for (final AudioTrack a in v.audio)
                        (a.index, _audioLabel(a)),
                    ],
              onChanged: (int? value) {
                setState(() => _audio = value);
                _apply();
              },
            ),
          ),
          _switch(Strings.playerStereoDownmix, _downmix, (bool value) {
            setState(() => _downmix = value);
            _apply();
          }),
        ];
      case PlayerPanel.subtitles:
        return <Widget>[
          _row(
            Strings.playerSubtitles,
            AppDropdown<String>(
              value: _sub,
              width: double.infinity,
              tapGroupId: kPlayerPanelGroup,
              items: <(String, String)>[
                ('', Strings.playerNoSubtitles),
                for (final SubtitleTrack s in v.subtitles)
                  ('embedded:${s.index}', _embeddedLabel(s)),
                for (final SubtitleFile f in v.subtitleFiles)
                  ('file:${f.id}', _fileLabel(f)),
              ],
              onChanged: (String value) {
                setState(() => _sub = value);
                _apply();
              },
            ),
          ),
          if (_sub.isNotEmpty && !burnedIn)
            _row(
              Strings.playerOffset,
              SizedBox(
                height: kControlHeight,
                child: TextField(
                  controller: _offset,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    isDense: true,
                    hintText: '0',
                    suffixText: 'ms',
                    contentPadding: EdgeInsets.symmetric(
                      horizontal: Space.s2,
                      vertical: Space.s2,
                    ),
                  ),
                  onSubmitted: (_) => _apply(),
                  onTapOutside: (_) => _apply(),
                ),
              ),
            ),
          if (_sub.isNotEmpty && hasSubs)
            _switch(Strings.playerBurnIn, _burn, (bool value) {
              setState(() => _burn = value);
              _apply();
            }),
          if (_sub.isNotEmpty && burnedIn && !_burn)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: Space.s2),
              child: Text(
                Strings.playerSubtitlesImageTrack,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: t.muted),
              ),
            ),
        ];
      case PlayerPanel.settings:
        return <Widget>[
          if (widget.autoplaySeconds != null)
            _row(
              Strings.playerAutoplayNext,
              AppDropdown<int>(
                value: _autoplay,
                width: double.infinity,
                tapGroupId: kPlayerPanelGroup,
                items: <(int, String)>[
                  for (final int seconds in kAutoplayDelays)
                    (
                      seconds,
                      seconds == 0
                          ? Strings.playerAutoplayOff
                          : Strings.playerAutoplayDelay(seconds),
                    ),
                ],
                onChanged: (int value) {
                  setState(() => _autoplay = value);
                  widget.onAutoplaySeconds?.call(value);
                },
              ),
            ),
          if (widget.pictureInPicture != null)
            _switch(
              Strings.playerPictureInPicture,
              widget.pictureInPicture!,
              (_) => widget.onPictureInPicture?.call(),
            ),
          _switch(Strings.playerDiagnostics, _diag, (bool value) {
            setState(() => _diag = value);
            widget.onDiagnostics(value);
          }),
          if (widget.onShortcuts != null)
            _row(
              Strings.shortcutsHeading,
              Align(
                alignment: Alignment.centerRight,
                child: IconButton(
                  onPressed: widget.onShortcuts,
                  iconSize: 18,
                  visualDensity: VisualDensity.compact,
                  tooltip: Strings.shortcutsHeading,
                  icon: Icon(Icons.keyboard, color: t.muted),
                ),
              ),
            ),
          if (widget.mode != null)
            _row(
              Strings.playerModeLabel,
              Text(
                widget.mode!,
                textAlign: TextAlign.right,
                style: TextStyle(color: t.text, fontSize: 13),
              ),
            ),
        ];
    }
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Material(
      color: t.surface,
      clipBehavior: Clip.antiAlias,
      shape: RoundedRectangleBorder(
        borderRadius: const BorderRadius.all(Radii.md),
        side: BorderSide(color: t.border),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _header(t),
          Flexible(
            child: SingleChildScrollView(
              padding: EdgeInsets.all(widget.dense ? Space.s2 : Space.s4),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: _rows(context, t),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _header(Tokens t) {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: t.border)),
      ),
      child: Padding(
        padding: EdgeInsets.fromLTRB(
          widget.dense ? Space.s2 : Space.s4,
          widget.dense ? 0 : Space.s2,
          Space.s2,
          widget.dense ? 0 : Space.s2,
        ),
        child: Row(
          children: <Widget>[
            Icon(widget.panel.icon, size: 18, color: t.muted),
            const SizedBox(width: Space.s2),
            Expanded(
              child: Text(
                widget.panel.title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            IconButton(
              onPressed: widget.onClose,
              iconSize: 18,
              visualDensity: VisualDensity.compact,
              icon: Icon(Icons.close, color: t.muted),
              tooltip: Strings.close,
            ),
          ],
        ),
      ),
    );
  }

  Widget _row(String label, Widget control, {bool nameControl = true}) {
    final Tokens t = context.tokens;
    final bool dense = widget.dense;
    return Padding(
      padding: EdgeInsets.symmetric(vertical: dense ? _kDenseRowPad : Space.s1),
      child: SizedBox(
        height: dense ? kControlHeight : kControlHeight + Space.s1,
        child: Row(
          children: <Widget>[
            Expanded(
              child: Text(
                label,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: t.muted, fontSize: dense ? 13 : null),
              ),
            ),
            SizedBox(width: dense ? Space.s2 : Space.s3),
            SizedBox(
              width: dense ? _kDenseControlColumn : _kControlColumn,
              child: nameControl
                  ? Semantics(label: label, child: control)
                  : control,
            ),
          ],
        ),
      ),
    );
  }

  Widget _switch(String label, bool value, ValueChanged<bool> onChanged) =>
      InkWell(
        onTap: () => onChanged(!value),
        child: _row(
          label,
          Align(
            alignment: Alignment.centerRight,
            child: IgnorePointer(
              child: Switch(value: value, onChanged: onChanged),
            ),
          ),
          nameControl: false,
        ),
      );

  String _audioLabel(AudioTrack a) => <String>[
    '${Strings.playerAudio} ${a.index}',
    a.codec,
    if (a.language != null) languageLabel(a.language!),
  ].join(' ');

  String _embeddedLabel(SubtitleTrack s) => <String>[
    Strings.subtitleTrackLabel(s.index),
    s.format.name,
    if (s.language != null) languageLabel(s.language!),
  ].join(' ');

  String _fileLabel(SubtitleFile f) => <String>[
    f.language == null ? '?' : languageLabel(f.language!),
    f.format.name,
    '(${f.source.name})',
    if (f.label != null) f.label!,
  ].join(' ');
}
