import 'package:flutter/material.dart';

import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/bare_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/util/server_address.dart';

class ServerPage extends StatefulWidget {
  const ServerPage({super.key, this.initialAddress});

  final String? initialAddress;

  @override
  State<ServerPage> createState() => _ServerPageState();
}

class _ServerPageState extends State<ServerPage> {
  late final TextEditingController _address = TextEditingController(
    text: widget.initialAddress ?? '',
  );
  String? _error;
  bool _busy = false;

  @override
  void dispose() {
    _address.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final ServerScope? scope = ServerScope.of(context);
    if (scope == null) {
      return;
    }
    final String? address = normalizeServerAddress(_address.text);
    if (address == null) {
      setState(() => _error = Strings.serverAddressInvalid);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    final bool answers = await scope.probe(address);
    if (!mounted) {
      return;
    }
    if (!answers) {
      setState(() {
        _busy = false;
        _error = Strings.serverAddressUnanswered;
      });
      return;
    }
    if (address == scope.address) {
      Navigator.of(context).pop();
      return;
    }
    await scope.setAddress(address);
  }

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final bool canCancel = Navigator.of(context).canPop();
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
                  Strings.serverHeading,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
                const SizedBox(height: Space.s3),
                Text(
                  Strings.serverAddressHelp,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: t.muted),
                ),
                const SizedBox(height: Space.s4),
                TextField(
                  controller: _address,
                  autofocus: true,
                  keyboardType: TextInputType.url,
                  textInputAction: TextInputAction.done,
                  decoration: const InputDecoration(
                    labelText: Strings.serverAddress,
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
                  child: Text(_busy ? Strings.connecting : Strings.connect),
                ),
                if (canCancel) ...<Widget>[
                  const SizedBox(height: Space.s2),
                  OutlinedButton(
                    onPressed: _busy ? null : () => Navigator.of(context).pop(),
                    child: const Text(Strings.cancel),
                  ),
                ],
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
