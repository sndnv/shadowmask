import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:http/http.dart' as http;

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/components/brand_mark.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/default/bare_page.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

class SignInPage extends StatefulWidget {
  const SignInPage({super.key, required this.api});

  final ApiClient api;

  @override
  State<SignInPage> createState() => _SignInPageState();
}

class _SignInPageState extends State<SignInPage> {
  final TextEditingController _username = TextEditingController();
  final TextEditingController _password = TextEditingController();
  final FocusNode _passwordFocus = FocusNode();
  String? _error;
  bool _busy = false;
  bool _obscure = true;

  @override
  void initState() {
    super.initState();
    _redirectIfSignedIn();
  }

  Future<void> _redirectIfSignedIn() async {
    final bool signedIn = (await widget.api.currentTokens()) != null;
    if (signedIn && mounted) {
      Navigator.of(context).pushReplacementNamed('/home');
    }
  }

  @override
  void dispose() {
    _username.dispose();
    _password.dispose();
    _passwordFocus.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final String username = _username.text.trim();
    final String password = _password.text;
    if (username.isEmpty || password.isEmpty) {
      setState(() => _error = Strings.usernameAndPasswordRequired);
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.api.login(username, password);
      TextInput.finishAutofillContext();
      if (mounted) {
        Navigator.of(context).pushReplacementNamed('/home');
      }
    } on http.ClientException {
      if (mounted) {
        setState(
          () => _error = Strings.signInFailed(Strings.serverUnreachable),
        );
      }
    } catch (e) {
      if (mounted) {
        setState(() => _error = Strings.signInFailed(e.toString()));
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
            child: AutofillGroup(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Text(
                    Strings.signInTitle,
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.headlineMedium,
                  ),
                  const SizedBox(height: Space.s4),
                  TextField(
                    controller: _username,
                    autofocus: true,
                    autofillHints: const <String>[AutofillHints.username],
                    textInputAction: TextInputAction.next,
                    decoration: const InputDecoration(
                      labelText: Strings.username,
                    ),
                    onSubmitted: (_) => _passwordFocus.requestFocus(),
                  ),
                  const SizedBox(height: Space.s3),
                  TextField(
                    controller: _password,
                    focusNode: _passwordFocus,
                    obscureText: _obscure,
                    autofillHints: const <String>[AutofillHints.password],
                    textInputAction: TextInputAction.done,
                    decoration: InputDecoration(
                      labelText: Strings.password,
                      suffixIcon: IconButton(
                        onPressed: () => setState(() => _obscure = !_obscure),
                        tooltip: _obscure
                            ? Strings.showPassword
                            : Strings.hidePassword,
                        icon: Icon(
                          _obscure
                              ? Icons.visibility_outlined
                              : Icons.visibility_off_outlined,
                          color: t.muted,
                        ),
                      ),
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
                    child: Text(_busy ? Strings.loading : Strings.signInTitle),
                  ),
                  if (scope != null) ...<Widget>[
                    const SizedBox(height: Space.s2),
                    TextButton(
                      onPressed: _busy ? null : () => _changeServer(scope),
                      child: const Text(Strings.changeServer),
                    ),
                  ],
                  const SizedBox(height: Space.s6),
                  const Center(child: BrandMark(size: 40)),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
