import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';
import 'package:flutter_svg/flutter_svg.dart';

import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/components/linked_nodes.dart';
import 'package:shadowmask/components/tab_heading.dart';
import 'package:shadowmask/components/title_bar.dart';
import 'package:shadowmask/components/version_label.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/player/player_prefs_store.dart';
import 'package:shadowmask/player/state/local_settings.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_choice.dart';
import 'package:shadowmask/view/subtitle_size.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/model/catalog/version.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/subtitle_labels.dart';
import 'package:shadowmask/view/version_order.dart';

const List<int?> _qualityRungs = <int?>[null, 1080, 720, 480, 320];

int _sourceHeight(VersionDetail version) =>
    version.video.isNotEmpty ? version.video.first.height : 0;

List<int?> _offeredRungs(VersionDetail version) {
  final int source = _sourceHeight(version);
  if (source <= 0) {
    return _qualityRungs;
  }
  return _qualityRungs
      .where((int? rung) => rung == null || rung < source)
      .toList();
}

String _originalLabel(VersionDetail version) {
  final int source = _sourceHeight(version);
  return source > 0 ? Strings.playerOriginalAt(source) : Strings.playerOriginal;
}

String _qualityLabel(VersionDetail version, int? rung) =>
    rung == null ? _originalLabel(version) : Strings.qualityRung(rung);

List<(int?, String)> _audioItems(VersionDetail version) {
  final List<String> labels = playerAudioLabels(version);
  return <(int?, String)>[
    for (int i = 0; i < version.audio.length; i++)
      (version.audio[i].index, labels[i]),
  ];
}

List<(String, String)> _subtitleItems(VersionDetail version) {
  final List<String> wires = <String>[
    for (final SubtitleTrack s in version.subtitles) 'embedded:${s.index}',
    for (final SubtitleFile f in version.subtitleFiles) 'file:${f.id}',
  ];
  final List<String> labels = playerSubtitleLabels(version);
  return <(String, String)>[
    ('', Strings.playerNoSubtitles),
    for (int i = 0; i < wires.length; i++) (wires[i], labels[i]),
  ];
}

String? panelSummary(
  PlayerPanel panel,
  PlaybackControls controls,
  VersionDetail version,
) {
  switch (panel) {
    case PlayerPanel.video:
      return _qualityLabel(version, controls.height);
    case PlayerPanel.audio:
      for (final (int?, String) item in _audioItems(version)) {
        if (item.$1 == controls.audioTrack) {
          return item.$2;
        }
      }
      return null;
    case PlayerPanel.subtitles:
      final String wire = controls.subtitle.selection?.toWire() ?? '';
      for (final (String, String) item in _subtitleItems(version)) {
        if (item.$1 == wire) {
          return item.$2;
        }
      }
      return null;
    case PlayerPanel.settings:
      return null;
  }
}

int steppedOffset(int ms, int delta) =>
    (ms + delta).clamp(-kOffsetBoundMs, kOffsetBoundMs);

const List<double> _speeds = <double>[0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
const double _kControlColumn = 168;
const double _kDenseControlColumn = 132;
const double _kDenseRowPad = 3;
const double kSheetRowHeight = 48;
const int kOffsetStepMs = 250;
const int kOffsetBoundMs = 30000;

class PlayerChoice {
  const PlayerChoice({
    required this.title,
    required this.labels,
    required this.selected,
    required this.onPick,
  });

  final String title;
  final List<String> labels;
  final int selected;
  final ValueChanged<int> onPick;
}

typedef ChoiceOpener = void Function(PlayerChoice choice);

const Object kPlayerPanelGroup = Object();

enum PlayerPanel {
  video(Strings.playerVideo, icon: Icons.video_settings),
  audio(Strings.playerAudio, glyph: 'assets/glyphs/icon-audio.svg'),
  subtitles(Strings.playerSubtitles, icon: Icons.subtitles),
  settings(Strings.playerSettings, icon: Icons.tune);

  const PlayerPanel(this.title, {this.icon, this.glyph});

  final String title;
  final IconData? icon;
  final String? glyph;
}

const double kPanelGlyphScale = 1.25;

class PanelIcon extends StatelessWidget {
  const PanelIcon(this.panel, {super.key, this.size, this.color});

  final PlayerPanel panel;
  final double? size;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final String? glyph = panel.glyph;
    if (glyph == null) {
      return Icon(panel.icon, size: size, color: color);
    }
    final IconThemeData theme = IconTheme.of(context);
    final Color? tint = color ?? theme.color;
    return SizedBox.square(
      dimension: size ?? theme.size ?? 24,
      child: Transform.scale(
        scale: kPanelGlyphScale,
        child: SvgPicture.asset(
          glyph,
          excludeFromSemantics: true,
          colorFilter: tint == null
              ? null
              : ColorFilter.mode(tint, BlendMode.srcIn),
        ),
      ),
    );
  }
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
    this.container,
    this.subtitleDelivery,
    this.pictureInPicture,
    this.onPictureInPicture,
    this.autoplaySeconds,
    this.onAutoplaySeconds,
    this.subtitleSize,
    this.onSubtitleSize,
    this.networkTimeoutSeconds,
    this.onNetworkTimeoutSeconds,
    this.bufferSeconds,
    this.onBufferSeconds,
    this.bufferBytes,
    this.onBufferBytes,
    this.waitForBuffer,
    this.onWaitForBuffer,
    this.onShortcuts,
    this.onShowLog,
    this.onAddSubtitles,
    this.openChoice,
    this.versions = const <Version>[],
    this.onVersion,
    this.sound = kNoSoundRows,
    this.dense = false,
    this.busy = false,
  });

  final SoundRows sound;
  final PlayerPanel panel;
  final PlaybackControls controls;
  final VersionDetail version;
  final List<Version> versions;
  final ValueChanged<String>? onVersion;
  final double speed;
  final bool diagnostics;
  final String? mode;
  final String? container;
  final String? subtitleDelivery;
  final bool? pictureInPicture;
  final VoidCallback? onPictureInPicture;
  final int? autoplaySeconds;
  final ValueChanged<int>? onAutoplaySeconds;
  final SubtitleSize? subtitleSize;
  final ValueChanged<SubtitleSize>? onSubtitleSize;
  final int? networkTimeoutSeconds;
  final ValueChanged<int>? onNetworkTimeoutSeconds;
  final int? bufferSeconds;
  final ValueChanged<int>? onBufferSeconds;
  final int? bufferBytes;
  final ValueChanged<int>? onBufferBytes;
  final bool? waitForBuffer;
  final ValueChanged<bool>? onWaitForBuffer;
  final VoidCallback? onShortcuts;
  final VoidCallback? onShowLog;
  final VoidCallback? onAddSubtitles;
  final ChoiceOpener? openChoice;
  final ValueChanged<PlaybackControls> onControls;
  final ValueChanged<double> onSpeed;
  final ValueChanged<bool> onDiagnostics;
  final VoidCallback onClose;
  final bool dense;
  final bool busy;

  @override
  State<PlayerSettingsPanel> createState() => _PlayerSettingsPanelState();
}

class _PlayerSettingsPanelState extends State<PlayerSettingsPanel> {
  late int? _height;
  late int? _audio;
  late SubtitleChoice _subtitle;
  late String _sub;
  final TextEditingController _offset = TextEditingController();
  late bool _burn;
  late bool _downmix;
  late bool _normalize;
  late DialogueLift _dialogue;
  late NightMode _night;
  late DeliveryPreference? _delivery;
  PlaybackControls? _asked;
  late double _speed = widget.speed;
  late bool _diag = widget.diagnostics;
  late int _autoplay =
      widget.autoplaySeconds ?? const LocalSettings().autoplayNextS;
  late SubtitleSize _subtitleSize =
      widget.subtitleSize ?? const LocalSettings().subtitleSize;
  late int _timeout =
      widget.networkTimeoutSeconds ?? const LocalSettings().networkTimeoutS;
  late int _buffer = widget.bufferSeconds ?? const LocalSettings().bufferS;
  late int _bufferBytes =
      widget.bufferBytes ?? const LocalSettings().bufferBytes;
  late bool _wait = widget.waitForBuffer ?? const LocalSettings().waitForBuffer;
  final Map<PlayerPanel, String> _open = <PlayerPanel, String>{};

  @override
  void initState() {
    super.initState();
    _show(widget.controls);
  }

  @override
  void didUpdateWidget(PlayerSettingsPanel old) {
    super.didUpdateWidget(old);
    final PlaybackControls? asked = _asked;
    if (widget.controls != old.controls ||
        (asked != null && widget.controls != asked)) {
      _asked = null;
      _show(widget.controls);
    }
  }

  @override
  void dispose() {
    _offset.dispose();
    super.dispose();
  }

  void _show(PlaybackControls controls) {
    _height = controls.height;
    _audio = controls.audioTrack;
    _subtitle = controls.subtitle;
    _sub = controls.subtitle.selection?.toWire() ?? '';
    _offset.text = controls.offsetMs == 0 ? '' : controls.offsetMs.toString();
    _burn = controls.burn;
    _downmix = controls.downmix ?? false;
    _normalize = controls.normalize ?? true;
    _dialogue = controls.dialogue ?? DialogueLift.none;
    _night = controls.night ?? NightMode.off;
    _delivery = controls.delivery;
  }

  Widget _stream(Widget row) => AbsorbPointer(
    absorbing: widget.busy,
    child: Opacity(opacity: widget.busy ? 0.5 : 1, child: row),
  );

  Widget _streamIf(bool stream, Widget row) => stream ? _stream(row) : row;

  void _apply() {
    final PlaybackControls asked = PlaybackControls(
      height: _height,
      burn: _burn,
      downmix: _downmix,
      normalize: _normalize,
      dialogue: _dialogue,
      night: _night,
      audioTrack: _audio,
      subtitle: _subtitle,
      offsetMs: int.tryParse(_offset.text) ?? 0,
      delivery: _delivery,
    );
    _asked = asked;
    widget.onControls(asked);
  }

  List<(String, String)> _versionItems() {
    final List<Version> all = widget.versions;
    final List<(String, String)> items = <(String, String)>[
      if (widget.onVersion != null)
        for (int i = 0; i < all.length; i++)
          if (all[i].available)
            (all[i].id, versionLabelText(all[i], versionNumberLabel(i))),
    ];
    return items.any(((String, String) item) => item.$1 == widget.version.id)
        ? items
        : const <(String, String)>[];
  }

  void _pickVersion(String id) {
    if (id != widget.version.id) {
      widget.onVersion?.call(id);
    }
  }

  List<Widget> _rows(BuildContext context, Tokens t) {
    final VersionDetail v = widget.version;
    final bool hasSubs = v.subtitles.isNotEmpty || v.subtitleFiles.isNotEmpty;
    final bool burnedIn = widget.subtitleDelivery == 'burned';
    switch (widget.panel) {
      case PlayerPanel.video:
        final List<(String, String)> versions = _versionItems();
        return <Widget>[
          if (versions.length > 1)
            _choice<String>(
              Strings.playerVersion,
              v.id,
              versions,
              _pickVersion,
              help: Strings.playerVersionHelp,
              stream: true,
            ),
          _choice<int?>(
            Strings.playerQuality,
            _height,
            <(int?, String)>[
              for (final int? rung in _offeredRungs(v))
                (rung, _qualityLabel(v, rung)),
            ],
            (int? value) {
              setState(() => _height = value);
              _apply();
            },
            help: Strings.playerQualityHelp,
            stream: true,
          ),
          _choice<double>(
            Strings.playerSpeed,
            _speed,
            <(double, String)>[
              for (final double rate in _speeds)
                (rate, rate == 1.0 ? Strings.playerNormalSpeed : '${rate}x'),
            ],
            (double value) {
              setState(() => _speed = value);
              widget.onSpeed(value);
            },
          ),
        ];
      case PlayerPanel.audio:
        return <Widget>[
          _choice<int?>(
            Strings.playerAudio,
            _audio,
            v.audio.isEmpty
                ? <(int?, String)>[(null, Strings.playerNoAudio)]
                : _audioItems(v),
            (int? value) {
              setState(() => _audio = value);
              _apply();
            },
            stream: true,
          ),
          if (widget.sound.downmix)
            _switch(
              Strings.playerStereoDownmix,
              _downmix,
              (bool value) {
                setState(() => _downmix = value);
                _apply();
              },
              help: Strings.playerStereoDownmixHelp,
              stream: true,
            ),
          if (widget.sound.normalize)
            _switch(
              Strings.playerPreventDistortion,
              _normalize,
              (bool value) {
                setState(() => _normalize = value);
                _apply();
              },
              help: Strings.playerPreventDistortionHelp,
              stream: true,
            ),
          if (widget.sound.dialogue)
            _choice<DialogueLift>(
              Strings.playerDialogue,
              _dialogue,
              <(DialogueLift, String)>[
                for (final DialogueLift lift in DialogueLift.values)
                  (lift, lift.label),
              ],
              (DialogueLift value) {
                setState(() => _dialogue = value);
                _apply();
              },
              help: Strings.playerDialogueHelp,
              stream: true,
            ),
          if (widget.sound.night)
            _choice<NightMode>(
              Strings.playerNightMode,
              _night,
              <(NightMode, String)>[
                for (final NightMode mode in NightMode.values)
                  (mode, mode.label),
              ],
              (NightMode value) {
                setState(() => _night = value);
                _apply();
              },
              help: Strings.playerNightModeHelp,
              stream: true,
            ),
        ];
      case PlayerPanel.subtitles:
        return <Widget>[
          _choice<String>(Strings.playerSubtitles, _sub, _subtitleItems(v), (
            String value,
          ) {
            final SubtitleSelection? track = SubtitleSelection.fromWire(value);
            setState(() {
              _sub = value;
              _subtitle = track == null
                  ? const SubtitleChoice.off()
                  : SubtitleChoice.on(track);
            });
            _apply();
          }, stream: true),
          if (_sub.isNotEmpty && !burnedIn && !_sheet)
            _mapped
                ? _offsetNode()
                : _stream(
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
                      help: Strings.playerOffsetHelp,
                      fills: true,
                    ),
                  ),
          if (_sub.isNotEmpty && !burnedIn && widget.onSubtitleSize != null)
            _choice<SubtitleSize>(
              Strings.playerSubtitleSize,
              _subtitleSize,
              <(SubtitleSize, String)>[
                for (final SubtitleSize size in SubtitleSize.values)
                  (size, size.label),
              ],
              (SubtitleSize value) {
                setState(() => _subtitleSize = value);
                widget.onSubtitleSize?.call(value);
              },
              help: Strings.subtitleSizeHelp,
            ),
          if (_sub.isNotEmpty && hasSubs)
            _switch(
              Strings.playerBurnIn,
              _burn,
              (bool value) {
                setState(() => _burn = value);
                _apply();
              },
              help: Strings.playerBurnInHelp,
              stream: true,
            ),
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
          if (_sub.isNotEmpty && !burnedIn && _sheet)
            _stream(_offsetStepper(t)),
          if (widget.onAddSubtitles != null) ...<Widget>[
            ..._section(t, null),
            _action(
              Strings.moreSubtitles,
              widget.onAddSubtitles,
              TextButton.icon(
                onPressed: widget.onAddSubtitles,
                icon: const Icon(Icons.add, size: 18),
                label: const Text(Strings.addSubtitles),
              ),
              help: Strings.moreSubtitlesHelp,
            ),
          ],
        ];
      case PlayerPanel.settings:
        return <Widget>[
          if (widget.autoplaySeconds != null)
            _choice<int>(
              Strings.playerAutoplayNext,
              _autoplay,
              <(int, String)>[
                for (final int value in kAutoplayDelays)
                  (value, autoplayLabel(value)),
              ],
              (int value) {
                setState(() => _autoplay = value);
                widget.onAutoplaySeconds?.call(value);
              },
              help: Strings.playerAutoplayNextHelp,
            ),
          if (widget.onNetworkTimeoutSeconds != null)
            _choice<int>(
              Strings.playerNetworkTimeout,
              _timeout,
              <(int, String)>[
                for (final int seconds in kNetworkTimeouts)
                  (seconds, Strings.playerAutoplayDelay(seconds)),
              ],
              (int value) {
                setState(() => _timeout = value);
                widget.onNetworkTimeoutSeconds?.call(value);
              },
              help: Strings.playerNetworkTimeoutHelp,
            ),
          if (widget.pictureInPicture != null)
            _switch(
              Strings.playerPictureInPicture,
              widget.pictureInPicture!,
              (_) => widget.onPictureInPicture?.call(),
            ),
          if (widget.onShortcuts != null)
            _action(
              Strings.shortcutsHeading,
              widget.onShortcuts,
              IconButton(
                onPressed: widget.onShortcuts,
                iconSize: 18,
                visualDensity: VisualDensity.compact,
                tooltip: Strings.shortcutsHeading,
                icon: Icon(Icons.keyboard, color: t.muted),
              ),
            ),
          if (widget.onBufferSeconds != null ||
              widget.onBufferBytes != null ||
              widget.onWaitForBuffer != null) ...<Widget>[
            ..._section(t, Strings.playerBufferingSection),
            if (widget.onBufferSeconds != null)
              _choice<int>(
                Strings.playerBufferTarget,
                _buffer,
                <(int, String)>[
                  for (final int seconds in kBufferTargets)
                    (seconds, Strings.playerBufferDuration(seconds)),
                ],
                (int value) {
                  setState(() => _buffer = value);
                  widget.onBufferSeconds?.call(value);
                },
                help: Strings.playerBufferHelp,
              ),
            if (widget.onBufferBytes != null)
              _choice<int>(
                Strings.playerBufferLimit,
                _bufferBytes,
                <(int, String)>[
                  for (final int bytes in kBufferSizes)
                    (bytes, Strings.playerBufferSize(bytes)),
                ],
                (int value) {
                  setState(() => _bufferBytes = value);
                  widget.onBufferBytes?.call(value);
                },
                help: Strings.playerBufferLimitHelp,
              ),
            if (widget.onWaitForBuffer != null)
              _switch(Strings.playerWaitForBuffer, _wait, (bool value) {
                setState(() => _wait = value);
                widget.onWaitForBuffer?.call(value);
              }, help: Strings.playerWaitForBufferHelp),
          ],
          ..._section(t, Strings.playerAdvanced),
          _choice<DeliveryPreference?>(
            Strings.playerDelivery,
            _delivery,
            const <(DeliveryPreference?, String)>[
              (DeliveryPreference.auto, Strings.playerDeliveryAuto),
              (DeliveryPreference.never, Strings.playerDeliveryNever),
              (DeliveryPreference.always, Strings.playerDeliveryAlways),
            ],
            (DeliveryPreference? value) {
              setState(() => _delivery = value);
              _apply();
            },
            help: Strings.playerDeliveryHelp,
            stream: true,
          ),
          _switch(Strings.playerDiagnostics, _diag, (bool value) {
            setState(() => _diag = value);
            widget.onDiagnostics(value);
          }, help: Strings.playerDiagnosticsHelp),
          if (widget.onShowLog != null)
            _action(
              Strings.playerLog,
              widget.onShowLog,
              TextButton(
                onPressed: widget.onShowLog,
                child: const Text(Strings.playerLogShow),
              ),
              help: Strings.playerLogHelp,
            ),
          if (_mapped)
            _streamFacts()
          else ...<Widget>[
            if (widget.mode != null)
              _row(
                Strings.playerModeLabel,
                Text(
                  widget.mode!,
                  textAlign: TextAlign.right,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(color: t.text, fontSize: 13),
                ),
                help: Strings.playerModeHelp,
              ),
            _row(
              Strings.playerContainerLabel,
              Text(
                widget.container ?? Strings.playerContainerNone,
                textAlign: TextAlign.right,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: t.text, fontSize: 13),
              ),
              help: Strings.playerContainerHelp,
            ),
          ],
        ];
    }
  }

  Widget? _help(String title, String? body) =>
      body == null ? null : FieldHelp(title: title, body: body);

  int _openAt(List<_Node> nodes) {
    final String? remembered = _open[widget.panel];
    int first = -1;
    for (int i = 0; i < nodes.length; i++) {
      final _Node node = nodes[i];
      if (node is! _NodeChoice && node is! _NodeFacts) {
        continue;
      }
      if (node.label == remembered) {
        return i;
      }
      if (first < 0) {
        first = i;
      }
    }
    if (first >= 0) {
      _open[widget.panel] = nodes[first].label;
    }
    return first;
  }

  Widget _left(_Node node, bool open) {
    void show() => setState(() => _open[widget.panel] = node.label);
    return switch (node) {
      _NodeChoice() => _streamIf(
        node.stream,
        NodeItem(
          label: node.label,
          value: node.current,
          selected: open,
          role: NodeRole.opens,
          help: _help(node.label, node.help),
          onTap: show,
        ),
      ),
      _NodeFacts() => NodeItem(
        label: node.label,
        value: node.value,
        selected: open,
        role: NodeRole.opens,
        onTap: show,
      ),
      _NodeAction() => NodeItem(
        label: node.label,
        icon: Icons.open_in_new,
        selected: false,
        role: NodeRole.plain,
        help: _help(node.label, node.help),
        onTap: node.onTap,
      ),
      _NodeCaption() => NodeCaption(node.label),
    };
  }

  List<Widget> _right(_Node? open, {required bool blocked}) {
    final Tokens t = context.tokens;
    return switch (open) {
      _NodeChoice() => <Widget>[
        for (int i = 0; i < open.labels.length; i++)
          NodeItem(
            label: open.labels[i],
            selected: i == open.selected,
            selectedColor: t.accent,
            role: open.steps ? NodeRole.plain : NodeRole.choice,
            enabled: !open.idle.contains(i),
            onTap: () {
              if (!blocked) {
                open.onPick(i);
              }
            },
          ),
      ],
      _NodeFacts() => <Widget>[
        for (final (String label, String value, String help) in open.facts)
          NodeItem(
            label: label,
            value: value,
            selected: false,
            role: NodeRole.plain,
            help: _help(label, help),
            onTap: null,
          ),
      ],
      _ => const <Widget>[],
    };
  }

  Widget _map(List<_Node> nodes) {
    final int at = _openAt(nodes);
    final _Node? open = at < 0 ? null : nodes[at];
    final bool blocked = open is _NodeChoice && open.stream && widget.busy;
    return LinkedNodes(
      from: math.max(0, at),
      lit: open is _NodeChoice ? open.selected : -1,
      rightLabel: open?.label,
      rightEnabled: !blocked,
      left: <Widget>[
        for (final _Node node in nodes) _left(node, identical(node, open)),
      ],
      right: _right(open, blocked: blocked),
    );
  }

  List<Widget> _section(Tokens t, String? caption) {
    if (_mapped) {
      return <Widget>[if (caption != null) _NodeCaption(label: caption)];
    }
    return <Widget>[
      Divider(color: t.border, height: Space.s5),
      if (caption != null) ...<Widget>[
        Text(
          caption,
          style: TextStyle(
            color: t.muted,
            fontSize: 13,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: Space.s2),
      ],
    ];
  }

  Widget _action(
    String label,
    VoidCallback? onTap,
    Widget control, {
    String? help,
  }) => _mapped
      ? _NodeAction(label: label, onTap: onTap, help: help)
      : _row(label, control, help: help);

  Widget _streamFacts() {
    final String container = widget.container ?? Strings.playerContainerNone;
    final String? mode = widget.mode;
    return _NodeFacts(
      label: Strings.playerStream,
      value: mode ?? container,
      facts: <(String, String, String)>[
        if (mode != null)
          (Strings.playerModeLabel, mode, Strings.playerModeHelp),
        (Strings.playerContainerLabel, container, Strings.playerContainerHelp),
      ],
    );
  }

  void _setOffset(int ms) {
    setState(() => _offset.text = ms == 0 ? '' : '$ms');
    _apply();
  }

  Widget _offsetNode() {
    final int ms = int.tryParse(_offset.text) ?? 0;
    final List<int> steps = <int>[
      steppedOffset(ms, -kOffsetStepMs),
      0,
      steppedOffset(ms, kOffsetStepMs),
    ];
    return _NodeChoice(
      label: Strings.playerOffset,
      labels: const <String>[
        Strings.playerOffsetEarlier,
        Strings.playerOffsetReset,
        Strings.playerOffsetLater,
      ],
      selected: -1,
      value: Strings.playerOffsetValue(ms),
      onPick: (int index) => _setOffset(steps[index]),
      help: Strings.playerOffsetHelp,
      stream: true,
      steps: true,
      idle: <int>{
        for (int i = 0; i < steps.length; i++)
          if (steps[i] == ms) i,
      },
    );
  }

  List<Widget> _body(BuildContext context, Tokens t) {
    final List<Widget> rows = _rows(context, t);
    if (!_mapped) {
      return rows;
    }
    final List<_Node> nodes = rows.whereType<_Node>().toList();
    final List<Widget> rest = <Widget>[
      for (final Widget row in rows)
        if (row is! _Node) row,
    ];
    return <Widget>[
      if (nodes.isNotEmpty) _map(nodes),
      if (nodes.isNotEmpty && rest.isNotEmpty) const SizedBox(height: Space.s3),
      ...rest,
    ];
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Widget rows = SingleChildScrollView(
      padding: _sheet
          ? const EdgeInsets.symmetric(horizontal: Space.s4, vertical: Space.s2)
          : EdgeInsets.all(widget.dense ? Space.s2 : Space.s4),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: _sheet
            ? CrossAxisAlignment.stretch
            : CrossAxisAlignment.start,
        children: _body(context, t),
      ),
    );
    if (_sheet) {
      return rows;
    }
    return Material(
      color: t.surface,
      clipBehavior: Clip.antiAlias,
      shape: context.shapes.border(
        Corners.panel,
        side: BorderSide(color: context.shapes.line(t.border)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _header(t),
          Flexible(child: rows),
        ],
      ),
    );
  }

  Widget _header(Tokens t) {
    if (context.shapes.cut) {
      return TitleBar(
        title: widget.panel.title,
        onClose: widget.onClose,
        dense: widget.dense,
        leading: PanelIcon(widget.panel, size: 18, color: t.accentContrast),
      );
    }
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
            PanelIcon(widget.panel, size: 18, color: t.muted),
            const SizedBox(width: Space.s2),
            Expanded(
              child: TabHeading(
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

  bool get _sheet => widget.openChoice != null;

  bool get _mapped => context.shapes.cut && !_sheet;

  Widget _row(
    String label,
    Widget control, {
    bool nameControl = true,
    String? help,
    bool fills = false,
  }) {
    final Tokens t = context.tokens;
    final bool dense = widget.dense && !_sheet;
    final double column = dense ? _kDenseControlColumn : _kControlColumn;
    final Widget named = nameControl
        ? Semantics(label: label, child: control)
        : control;
    return Padding(
      padding: EdgeInsets.symmetric(vertical: dense ? _kDenseRowPad : Space.s1),
      child: SizedBox(
        height: _sheet
            ? kSheetRowHeight
            : dense
            ? kControlHeight
            : kControlHeight + Space.s1,
        child: Row(
          children: <Widget>[
            Expanded(
              child: Row(
                children: <Widget>[
                  Flexible(
                    child: Text(
                      label,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        color: _sheet ? t.text : t.muted,
                        fontSize: _sheet ? 16 : (dense ? 13 : null),
                      ),
                    ),
                  ),
                  if (help != null) FieldHelp(title: label, body: help),
                ],
              ),
            ),
            SizedBox(width: dense ? Space.s2 : Space.s3),
            if (fills)
              SizedBox(width: column, child: named)
            else
              ConstrainedBox(
                constraints: BoxConstraints(maxWidth: column),
                child: named,
              ),
          ],
        ),
      ),
    );
  }

  Widget _switch(
    String label,
    bool value,
    ValueChanged<bool> onChanged, {
    String? help,
    bool stream = false,
  }) {
    if (_mapped) {
      return _NodeChoice(
        label: label,
        labels: const <String>[Strings.playerOn, Strings.playerOff],
        selected: value ? 0 : 1,
        onPick: (int index) {
          if ((index == 0) != value) {
            onChanged(index == 0);
          }
        },
        help: help,
        stream: stream,
      );
    }
    return _streamIf(
      stream,
      InkWell(
        onTap: () => onChanged(!value),
        child: _row(
          label,
          IgnorePointer(
            child: Switch(value: value, onChanged: onChanged),
          ),
          nameControl: help != null,
          help: help,
        ),
      ),
    );
  }

  Widget _choice<T>(
    String label,
    T value,
    List<(T, String)> items,
    ValueChanged<T> onChanged, {
    String? help,
    bool stream = false,
  }) {
    final int selected = items.indexWhere(
      ((T, String) item) => item.$1 == value,
    );
    if (_mapped) {
      return _NodeChoice(
        label: label,
        labels: <String>[for (final (T, String) item in items) item.$2],
        selected: selected,
        onPick: (int index) => onChanged(items[index].$1),
        help: help,
        stream: stream,
      );
    }
    final ChoiceOpener? open = widget.openChoice;
    if (open == null) {
      final Widget row = _row(
        label,
        AppDropdown<T>(
          value: value,
          width: double.infinity,
          tapGroupId: kPlayerPanelGroup,
          items: items,
          onChanged: onChanged,
        ),
        help: help,
        fills: true,
      );
      return _streamIf(stream, row);
    }
    final Widget drill = _drill(
      label,
      selected < 0 ? '' : items[selected].$2,
      () => open(
        PlayerChoice(
          title: label,
          labels: <String>[for (final (T, String) item in items) item.$2],
          selected: selected,
          onPick: (int index) => onChanged(items[index].$1),
        ),
      ),
      help: help,
    );
    return _streamIf(stream, drill);
  }

  Widget _drill(
    String label,
    String current,
    VoidCallback onTap, {
    String? help,
  }) {
    final Tokens t = context.tokens;
    return InkWell(
      onTap: onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: kSheetRowHeight),
        child: _apart(
          <Widget>[
            Flexible(
              child: Text(label, style: TextStyle(color: t.text, fontSize: 16)),
            ),
            if (help != null) FieldHelp(title: label, body: help),
          ],
          <Widget>[
            Flexible(
              child: Text(
                current,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(color: t.muted, fontSize: 15),
              ),
            ),
            Icon(Icons.chevron_right, color: t.muted),
          ],
        ),
      ),
    );
  }

  Widget _apart(List<Widget> start, List<Widget> end) => Row(
    mainAxisAlignment: MainAxisAlignment.spaceBetween,
    children: <Widget>[
      Flexible(
        child: Row(mainAxisSize: MainAxisSize.min, children: start),
      ),
      Flexible(
        child: Padding(
          padding: const EdgeInsets.only(left: Space.s2),
          child: Row(mainAxisSize: MainAxisSize.min, children: end),
        ),
      ),
    ],
  );

  Widget _offsetStepper(Tokens t) {
    final int ms = int.tryParse(_offset.text) ?? 0;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.s2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _apart(
            <Widget>[
              Flexible(
                child: Text(
                  Strings.playerOffset,
                  style: TextStyle(color: t.text, fontSize: 16),
                ),
              ),
              const FieldHelp(
                title: Strings.playerOffset,
                body: Strings.playerOffsetHelp,
              ),
            ],
            <Widget>[
              Flexible(
                child: Text(
                  Strings.playerOffsetValue(ms),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(color: t.muted, fontSize: 15),
                ),
              ),
            ],
          ),
          Row(
            children: <Widget>[
              IconButton(
                onPressed: () => _setOffset(steppedOffset(ms, -kOffsetStepMs)),
                tooltip: Strings.playerOffsetEarlier,
                icon: Icon(Icons.remove, color: t.text),
              ),
              Expanded(
                child: Center(
                  child: TextButton(
                    onPressed: ms == 0 ? null : () => _setOffset(0),
                    child: const Text(Strings.playerOffsetReset),
                  ),
                ),
              ),
              IconButton(
                onPressed: () => _setOffset(steppedOffset(ms, kOffsetStepMs)),
                tooltip: Strings.playerOffsetLater,
                icon: Icon(Icons.add, color: t.text),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

sealed class _Node extends StatelessWidget {
  const _Node({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}

class _NodeChoice extends _Node {
  const _NodeChoice({
    required super.label,
    required this.labels,
    required this.selected,
    required this.onPick,
    required this.stream,
    this.help,
    this.value,
    this.steps = false,
    this.idle = const <int>{},
  });

  final List<String> labels;
  final int selected;
  final ValueChanged<int> onPick;
  final bool stream;
  final String? help;
  final String? value;
  final bool steps;
  final Set<int> idle;

  String get current => value ?? (selected < 0 ? '' : labels[selected]);
}

class _NodeFacts extends _Node {
  const _NodeFacts({
    required super.label,
    required this.value,
    required this.facts,
  });

  final String value;
  final List<(String, String, String)> facts;
}

class _NodeAction extends _Node {
  const _NodeAction({required super.label, required this.onTap, this.help});

  final VoidCallback? onTap;
  final String? help;
}

class _NodeCaption extends _Node {
  const _NodeCaption({required super.label});
}
