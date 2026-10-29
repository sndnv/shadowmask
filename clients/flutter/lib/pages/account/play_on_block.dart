import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:shadowmask/components/play_on_host.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/setting_help.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/play_on_receiver.dart';

class PlayOnBlock extends StatelessWidget {
  const PlayOnBlock({super.key});

  @override
  Widget build(BuildContext context) {
    final PlayOnReceiver? receiver = PlayOnScope.maybeOf(context);
    if (receiver == null || !receiver.linked) {
      return const SizedBox.shrink();
    }
    final Tokens t = context.tokens;
    return SectionBlock(
      title: Strings.playOnHeading,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Material(
            color: Colors.transparent,
            child: SwitchListTile(
              contentPadding: kSettingRowPadding,
              minVerticalPadding: kSettingRowVerticalPadding,
              minTileHeight: kSettingRowMinHeight,
              value: receiver.enabled,
              onChanged: receiver.loaded
                  ? (bool on) => _setEnabled(context, receiver, on)
                  : null,
              title: const Text(
                Strings.playOnAllow,
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              subtitle: SettingHelp(_status(receiver)),
            ),
          ),
          if (receiver.enabled) ...<Widget>[
            Divider(color: t.border, height: 1),
            _PortRow(receiver: receiver),
          ],
        ],
      ),
    );
  }

  void _setEnabled(BuildContext context, PlayOnReceiver receiver, bool on) {
    receiver.setEnabled(on);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  String _status(PlayOnReceiver r) => switch (r.status) {
    PlayOnStatus.off => Strings.playOnAllowHelp,
    PlayOnStatus.listening => Strings.playOnListening(
      r.listeningOn,
      r.boundPort ?? 0,
    ),
    PlayOnStatus.noNetwork => Strings.playOnNoNetwork,
    PlayOnStatus.bindFailed => Strings.playOnBindFailed(r.port),
    PlayOnStatus.reportFailed => Strings.playOnReportFailed,
  };
}

class _PortRow extends StatefulWidget {
  const _PortRow({required this.receiver});

  final PlayOnReceiver receiver;

  @override
  State<_PortRow> createState() => _PortRowState();
}

class _PortRowState extends State<_PortRow> {
  late final TextEditingController _text = TextEditingController(
    text: widget.receiver.port?.toString() ?? '',
  );
  bool _invalid = false;

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  void _save() {
    final String text = _text.text.trim();
    final int? port = text.isEmpty ? null : int.parse(text);
    if (port != null && (port < kMinListenPort || port > kMaxListenPort)) {
      setState(() => _invalid = true);
      return;
    }
    setState(() => _invalid = false);
    widget.receiver.setPort(port);
    Toasts.of(context).success(Strings.toastSettingSaved);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    return Material(
      color: Colors.transparent,
      child: ListTile(
        contentPadding: kSettingRowPadding,
        minVerticalPadding: kSettingRowVerticalPadding,
        minTileHeight: kSettingRowMinHeight,
        title: const Text(
          Strings.playOnPort,
          style: TextStyle(fontWeight: FontWeight.w600),
        ),
        subtitle: SettingHelp(
          _invalid ? Strings.playOnPortInvalid : Strings.playOnPortHelp,
          color: _invalid ? t.danger : t.muted,
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            SizedBox(
              width: 110,
              child: Semantics(
                label: Strings.playOnPort,
                child: TextField(
                  controller: _text,
                  keyboardType: TextInputType.number,
                  inputFormatters: <TextInputFormatter>[
                    FilteringTextInputFormatter.digitsOnly,
                    LengthLimitingTextInputFormatter(5),
                  ],
                  onSubmitted: (_) => _save(),
                  decoration: const InputDecoration(
                    isDense: true,
                    hintText: Strings.playOnPortAutomatic,
                  ),
                ),
              ),
            ),
            const SizedBox(width: Space.s2),
            OutlinedButton(onPressed: _save, child: const Text(Strings.save)),
          ],
        ),
      ),
    );
  }
}
