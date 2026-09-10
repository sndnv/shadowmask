import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/components/link_code_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/bare_page.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/client_platform.dart';
import 'package:shadowmask/util/device_name.dart';

class LinkCodePage extends StatefulWidget {
  const LinkCodePage({
    super.key,
    required this.api,
    this.defaultDeviceName = deviceDisplayName,
  });

  final ApiClient api;
  final Future<String?> Function() defaultDeviceName;

  @override
  State<LinkCodePage> createState() => _LinkCodePageState();
}

class _LinkCodePageState extends State<LinkCodePage> {
  final TextEditingController _code = TextEditingController();
  final TextEditingController _device = TextEditingController(
    text: Strings.deviceNameFallback,
  );
  final FocusNode _deviceFocus = FocusNode();
  String? _error;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _prefillDeviceName();
  }

  Future<void> _prefillDeviceName() async {
    final String? name = await widget.defaultDeviceName();
    if (name != null && mounted && _device.text == Strings.deviceNameFallback) {
      _device.text = name;
    }
  }

  @override
  void dispose() {
    _code.dispose();
    _device.dispose();
    _deviceFocus.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final String code = _code.text.replaceAll(' ', '');
    final String device = _device.text.trim();
    if (code.isEmpty) {
      setState(() => _error = Strings.linkCodeRequired);
      return;
    }
    if (device.isEmpty) {
      setState(() => _error = Strings.deviceNameRequired);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.api.redeemLinkCode(
        code,
        deviceName: device,
        platform: clientPlatform(),
      );
      if (mounted) {
        Navigator.of(context).pushReplacementNamed('/home');
      }
    } on http.ClientException {
      if (mounted) {
        setState(() => _error = Strings.linkFailed(Strings.serverUnreachable));
      }
    } catch (e) {
      if (mounted) {
        setState(() => _error = Strings.linkFailed(e.toString()));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  void _changeServer(ServerScope scope) {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (BuildContext context) =>
            ServerPage(initialAddress: scope.address),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final ServerScope? scope = ServerScope.of(context);
    return BarePage(
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 360),
          child: Padding(
            padding: const EdgeInsets.all(Space.s4),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text(
                  Strings.linkDeviceTitle,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
                const SizedBox(height: Space.s3),
                Text(
                  Strings.linkDeviceHelp,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: t.muted),
                ),
                const SizedBox(height: Space.s4),
                TextField(
                  controller: _code,
                  autofocus: true,
                  autocorrect: false,
                  enableSuggestions: false,
                  textCapitalization: TextCapitalization.characters,
                  textInputAction: TextInputAction.next,
                  inputFormatters: <TextInputFormatter>[_LinkCodeFormatter()],
                  style: monoStyle.copyWith(
                    fontSize: 20,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 3,
                  ),
                  decoration: const InputDecoration(
                    labelText: Strings.linkCodeLabel,
                  ),
                  onSubmitted: (_) => _deviceFocus.requestFocus(),
                ),
                const SizedBox(height: Space.s3),
                TextField(
                  controller: _device,
                  focusNode: _deviceFocus,
                  textInputAction: TextInputAction.done,
                  decoration: const InputDecoration(
                    labelText: Strings.deviceNameLabel,
                  ),
                  onSubmitted: (_) => _submit(),
                ),
                if (_error != null) ...<Widget>[
                  const SizedBox(height: Space.s3),
                  SelectableText(
                    _error!,
                    style: Theme.of(
                      context,
                    ).textTheme.bodyMedium?.copyWith(color: t.danger),
                  ),
                ],
                const SizedBox(height: Space.s4),
                FilledButton(
                  onPressed: _busy ? null : _submit,
                  child: Text(
                    _busy ? Strings.linking : Strings.linkDeviceAction,
                  ),
                ),
                const SizedBox(height: Space.s2),
                TextButton(
                  onPressed: _busy
                      ? null
                      : () => Navigator.of(context).pushReplacementNamed('/'),
                  child: const Text(Strings.usePassword),
                ),
                if (scope != null)
                  TextButton(
                    onPressed: _busy ? null : () => _changeServer(scope),
                    child: const Text(Strings.changeServer),
                  ),
                const SizedBox(height: Space.s6),
                const Center(child: BrandMark(size: 40)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _LinkCodeFormatter extends TextInputFormatter {
  @override
  TextEditingValue formatEditUpdate(TextEditingValue _, TextEditingValue next) {
    final String stripped = next.text.toUpperCase().replaceAll(
      RegExp('[^A-Z0-9]'),
      '',
    );
    final String grouped = groupedLinkCode(stripped);
    return TextEditingValue(
      text: grouped,
      selection: TextSelection.collapsed(offset: grouped.length),
    );
  }
}
