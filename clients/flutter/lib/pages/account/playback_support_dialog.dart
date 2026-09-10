import 'package:flutter/material.dart';

import 'package:shadowmask/components/admin/dialog_shell.dart';
import 'package:shadowmask/components/admin/field_help.dart';
import 'package:shadowmask/components/app_dropdown.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/theme/space.dart';

class PlaybackSupportDialog extends StatefulWidget {
  const PlaybackSupportDialog({
    super.key,
    required this.measured,
    required this.overrides,
  });

  final ClientDecoding? measured;
  final CapabilityOverrides overrides;

  static Future<CapabilityOverrides?> show(
    BuildContext context, {
    required ClientDecoding? measured,
    required CapabilityOverrides overrides,
  }) => showDialog<CapabilityOverrides>(
    context: context,
    builder: (BuildContext context) =>
        PlaybackSupportDialog(measured: measured, overrides: overrides),
  );

  @override
  State<PlaybackSupportDialog> createState() => _PlaybackSupportDialogState();
}

class _PlaybackSupportDialogState extends State<PlaybackSupportDialog> {
  late CapabilityOverrides _overrides = widget.overrides;

  void _set(CapabilityOverrides next) => setState(() => _overrides = next);

  @override
  Widget build(BuildContext context) {
    final ClientDecoding? found = widget.measured;
    return DialogShell(
      title: Strings.playbackSupportHeading,
      footer: Row(
        mainAxisAlignment: MainAxisAlignment.end,
        spacing: Space.s3,
        children: <Widget>[
          if (!_overrides.isEmpty)
            TextButton(
              onPressed: () => _set(const CapabilityOverrides()),
              child: const Text(Strings.playbackSupportReset),
            ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(_overrides),
            child: const Text(Strings.save),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: Space.s4,
        children: <Widget>[
          FieldLabel(
            label: Strings.playbackSupportLargestPicture,
            help: Strings.playbackSupportPictureHelp,
            child: AppDropdown<int>(
              value: _overrides.maxHeight ?? 0,
              width: double.infinity,
              items: <(int, String)>[
                (0, _detected(found?.ceiling)),
                for (final int height in kPictureHeights)
                  (height, Strings.playbackSupportUpTo(height)),
              ],
              onChanged: (int value) =>
                  _set(_overrides.withMaxHeight(value == 0 ? null : value)),
            ),
          ),
          FieldLabel(
            label: Strings.playbackSupportFrameRate,
            help: Strings.playbackSupportFrameRateHelp,
            child: AppDropdown<int>(
              value: _overrides.maxFrameRate ?? 0,
              width: double.infinity,
              items: <(int, String)>[
                (
                  0,
                  _detected(
                    found?.maxFrameRate == null
                        ? null
                        : Strings.playbackSupportUpToRate(found!.maxFrameRate!),
                  ),
                ),
                for (final int rate in kFrameRateCaps)
                  (rate, Strings.playbackSupportUpToRate(rate)),
              ],
              onChanged: (int value) =>
                  _set(_overrides.withMaxFrameRate(value == 0 ? null : value)),
            ),
          ),
          if (found != null)
            for (final String codec in kKnownVideoCodecs)
              FieldLabel(
                label: codec,
                help: Strings.playbackSupportCodecHelp,
                child: AppDropdown<CodecSupport>(
                  value: _overrides.supportFor(codec),
                  width: double.infinity,
                  items: <(CodecSupport, String)>[
                    (CodecSupport.auto, _detected(_codecLabel(found, codec))),
                    (CodecSupport.hardware, Strings.playbackSupportHardware),
                    (CodecSupport.software, Strings.playbackSupportSoftware),
                    (
                      CodecSupport.unsupported,
                      Strings.playbackSupportUnsupported,
                    ),
                  ],
                  onChanged: (CodecSupport value) =>
                      _set(_overrides.withCodec(codec, value)),
                ),
              ),
          FieldLabel(
            label: Strings.playbackSupportHdr,
            help: Strings.playbackSupportHdrHelp,
            child: AppDropdown<HdrChoice>(
              value: _overrides.hdr,
              width: double.infinity,
              items: <(HdrChoice, String)>[
                (HdrChoice.auto, _detected(_hdrLabel(found))),
                (HdrChoice.allow, Strings.playbackSupportAllow),
                (HdrChoice.deny, Strings.playbackSupportDeny),
              ],
              onChanged: (HdrChoice value) => _set(_overrides.withHdr(value)),
            ),
          ),
        ],
      ),
    );
  }

  static String _detected(String? value) => value == null
      ? Strings.playbackSupportAuto
      : Strings.playbackSupportDetectedAs(value);

  static String _codecLabel(ClientDecoding measured, String codec) {
    for (final VideoCodecCap cap in measured.video) {
      if (cap.codec == codec) {
        return cap.smooth
            ? Strings.playbackSupportHardware
            : Strings.playbackSupportSoftware;
      }
    }
    return Strings.playbackSupportNone;
  }

  static String? _hdrLabel(ClientDecoding? measured) {
    final List<String>? hdr = measured?.hdr;
    if (hdr == null) {
      return null;
    }
    return hdr.isEmpty ? Strings.playbackSupportNone : hdr.join(', ');
  }
}
