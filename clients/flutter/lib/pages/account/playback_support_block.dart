import 'package:flutter/material.dart';

import 'package:shadowmask/api/capability_scope.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/pages/account/playback_support_dialog.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class PlaybackSupportBlock extends StatelessWidget {
  const PlaybackSupportBlock({super.key});

  Future<void> _edit(BuildContext context, CapabilityScope scope) async {
    final ValueChanged<CapabilityOverrides>? set = scope.setOverrides;
    if (set == null) {
      return;
    }
    final CapabilityOverrides? next = await PlaybackSupportDialog.show(
      context,
      measured: scope.measured,
      overrides: scope.overrides,
    );
    if (next != null) {
      set(next);
    }
  }

  @override
  Widget build(BuildContext context) {
    final CapabilityScope? scope = CapabilityScope.of(context);
    if (scope == null) {
      return const SizedBox.shrink();
    }
    final ClientDecoding? effective = scope.decoding;
    return SectionBlock(
      title: Strings.playbackSupportHeading,
      actions: <Widget>[if (!scope.overrides.isEmpty) const _ChangedChip()],
      actionItems: <PageAction>[
        if (scope.setOverrides != null)
          PageAction(
            icon: Icons.edit_outlined,
            label: Strings.editPlaybackSupport,
            onPressed: () => _edit(context, scope),
          ),
      ],
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          _Entry(
            label: Strings.playbackSupportDeviceType,
            values: <String>[scope.platform],
          ),
          if (effective != null) ...<Widget>[
            if (_picture(effective) != null)
              _Entry(
                label: Strings.playbackSupportLargestPicture,
                values: <String>[_picture(effective)!],
              ),
            _Entry(
              label: Strings.playbackSupportVideo,
              values: <String>[
                for (final VideoCodecCap codec in effective.video)
                  Strings.playbackSupportVideoCodec(
                    codec.codec,
                    codec.maxBitDepth,
                    smooth: codec.smooth,
                  ),
              ],
            ),
            _Entry(
              label: Strings.playbackSupportAudio,
              values: <String>[
                for (final AudioCodecCap codec in effective.audio)
                  Strings.playbackSupportAudioCodec(
                    codec.codec,
                    codec.maxChannels,
                  ),
              ],
            ),
            _Entry(
              label: Strings.playbackSupportHdr,
              values: effective.hdr ?? const <String>[],
            ),
          ],
        ],
      ),
    );
  }

  static String? _picture(ClientDecoding decoding) =>
      decoding.ceiling ??
      (decoding.maxHeight == null
          ? null
          : Strings.playbackSupportUpTo(decoding.maxHeight!));
}

class _ChangedChip extends StatelessWidget {
  const _ChangedChip();

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: Space.s2, vertical: 2),
      decoration: BoxDecoration(
        color: t.dangerBg,
        borderRadius: const BorderRadius.all(Radii.pill),
        border: Border.all(color: t.danger),
      ),
      child: Text(
        Strings.playbackSupportChanged,
        style: TextStyle(
          color: t.danger,
          fontSize: 12,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _Entry extends StatelessWidget {
  const _Entry({required this.label, required this.values});

  final String label;
  final List<String> values;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.s3),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: t.muted),
          ),
          const SizedBox(height: Space.s1),
          for (final String value in values)
            SelectableText(
              value,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
          if (values.isEmpty)
            const SelectableText(
              Strings.playbackSupportNone,
              style: TextStyle(fontWeight: FontWeight.w600),
            ),
        ],
      ),
    );
  }
}
