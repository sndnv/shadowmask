import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/page_states.dart';
import 'package:shadowmask/api/authentication_failure.dart';
import 'package:shadowmask/api/authorization_failure.dart';
import 'package:shadowmask/pages/default/server_unreachable.dart';
import 'package:shadowmask/pages/default/shell_scaffold.dart';

class SectionPage extends StatefulWidget {
  const SectionPage({
    super.key,
    required this.api,
    required this.section,
    required this.bodyBuilder,
    this.errorText,
    this.fullWidth = false,
    this.fitViewport = false,
    this.loading,
  });

  final ApiClient api;
  final NavSection section;
  final Widget Function(BuildContext context, SelfUser user) bodyBuilder;
  final String? errorText;
  final bool fullWidth;
  final bool fitViewport;
  final Widget? loading;

  @override
  State<SectionPage> createState() => _SectionPageState();
}

class _SectionPageState extends State<SectionPage> {
  late Future<SelfUser> _self = widget.api.currentUser();

  void _retry() {
    setState(() {
      _self = widget.api.currentUser();
    });
  }

  bool _unreachable(AsyncSnapshot<SelfUser> snapshot) {
    final Object? error = snapshot.error;
    return snapshot.connectionState == ConnectionState.done &&
        error != null &&
        error is! AuthenticationFailure &&
        error is! AuthorizationFailure;
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<SelfUser>(
      future: _self,
      builder: (BuildContext context, AsyncSnapshot<SelfUser> snapshot) =>
          _unreachable(snapshot)
          ? ServerUnreachablePage(onRetry: _retry)
          : ShellScaffold(
              api: widget.api,
              current: widget.section,
              user: snapshot.data,
              fullWidth: widget.fullWidth,
              fitViewport: widget.fitViewport,
              body: LoadingShape(
                shape: widget.loading,
                child: buildSnapshot<SelfUser>(
                  context,
                  snapshot,
                  errorText: widget.errorText,
                  onRetry: _retry,
                  loading: widget.loading,
                  builder: widget.bodyBuilder,
                ),
              ),
            ),
    );
  }
}
