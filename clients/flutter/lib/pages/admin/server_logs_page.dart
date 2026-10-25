import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_admin_api.dart';
import 'package:shadowmask/components/admin/log_level_dropdown.dart';
import 'package:shadowmask/components/admin/log_line.dart';
import 'package:shadowmask/components/breadcrumbs.dart';
import 'package:shadowmask/components/crumb.dart';
import 'package:shadowmask/components/muted_note.dart';
import 'package:shadowmask/components/page_actions.dart';
import 'package:shadowmask/components/section_block.dart';
import 'package:shadowmask/components/status_text.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/server/server_log_page.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/pages/default/section_page.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const int kServerLogPage = 200;
const Duration kServerLogPoll = Duration(seconds: 2);
const int _kReadsPerPoll = 10;
const double _kOlderWithin = 160;
const double _kBottomSlack = 24;

class ServerLogsPage extends StatelessWidget {
  const ServerLogsPage({super.key, required this.api});

  final ApiClient api;

  @override
  Widget build(BuildContext context) {
    return SectionPage(
      api: api,
      section: NavSection.admin,
      fullWidth: true,
      bodyBuilder: (BuildContext context, SelfUser user) => user.isAdmin
          ? _LogsBody(server: ServerAdminApi(api))
          : const StatusText(Strings.notAuthorized),
    );
  }
}

class _LogsBody extends StatefulWidget {
  const _LogsBody({required this.server});

  final ServerAdminApi server;

  @override
  State<_LogsBody> createState() => _LogsBodyState();
}

class _LogsBodyState extends State<_LogsBody> with RouteAware {
  final ScrollController _scroll = ScrollController();
  final Key _center = UniqueKey();
  final List<String> _older = <String>[];
  final List<String> _newer = <String>[];
  String _level = kDefaultLogLevel;
  String? _before;
  String? _after;
  bool _loading = true;
  bool _failed = false;
  bool _loadingOlder = false;
  bool _olderFailed = false;
  bool _polling = false;
  bool _pollFailed = false;
  bool _paused = false;
  int _generation = 0;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_onScroll);
    _follow();
    _load();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ModalRoute<dynamic>? route = ModalRoute.of(context);
    if (route is PageRoute<dynamic>) {
      appRouteObserver.subscribe(this, route);
    }
  }

  @override
  void didPushNext() => _timer?.cancel();

  @override
  void didPopNext() {
    _follow();
    _poll();
  }

  void _follow() {
    _timer?.cancel();
    _timer = Timer.periodic(kServerLogPoll, (_) => _poll());
  }

  @override
  void dispose() {
    appRouteObserver.unsubscribe(this);
    _timer?.cancel();
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final int generation = ++_generation;
    setState(() {
      _older.clear();
      _newer.clear();
      _before = null;
      _after = null;
      _loading = true;
      _loadingOlder = false;
      _failed = false;
      _olderFailed = false;
      _pollFailed = false;
    });
    try {
      final ServerLogPage page = await widget.server.logs(
        limit: kServerLogPage,
        level: _level,
      );
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        _newer.addAll(page.entries);
        _before = page.before;
        _after = page.after;
        _loading = false;
      });
      _toBottom();
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() {
          _loading = false;
          _failed = true;
        });
      }
    }
  }

  Future<void> _poll() async {
    if (_paused || _loading || _failed || _polling) {
      return;
    }
    _polling = true;
    final int generation = _generation;
    try {
      for (int reads = 0; reads < _kReadsPerPoll; reads++) {
        final ServerLogPage page = await widget.server.logs(
          limit: kServerLogPage,
          level: _level,
          after: _after,
        );
        if (!mounted || generation != _generation) {
          return;
        }
        final bool following = _atBottom();
        setState(() {
          _newer.addAll(page.entries);
          _after = page.after ?? _after;
          _pollFailed = false;
        });
        if (following && page.entries.isNotEmpty) {
          _toBottom();
        }
        if (!page.more) {
          break;
        }
      }
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _pollFailed = true);
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _loadOlder() async {
    if (_before == null || _loading || _loadingOlder || _olderFailed) {
      return;
    }
    final int generation = _generation;
    setState(() => _loadingOlder = true);
    try {
      final ServerLogPage page = await widget.server.logs(
        limit: kServerLogPage,
        level: _level,
        before: _before,
      );
      if (mounted && generation == _generation) {
        setState(() {
          _older.addAll(page.entries.reversed);
          _before = page.before;
        });
      }
    } catch (_) {
      if (mounted && generation == _generation) {
        setState(() => _olderFailed = true);
      }
    } finally {
      if (mounted && generation == _generation) {
        setState(() => _loadingOlder = false);
      }
    }
  }

  void _retryOlder() {
    setState(() => _olderFailed = false);
    _loadOlder();
  }

  void _onScroll() {
    final ScrollPosition position = _scroll.position;
    final bool scrollingUp =
        position.userScrollDirection == ScrollDirection.forward;
    if (scrollingUp &&
        position.pixels <= position.minScrollExtent + _kOlderWithin) {
      _loadOlder();
    }
  }

  bool _atBottom() =>
      !_scroll.hasClients ||
      _scroll.position.pixels >=
          _scroll.position.maxScrollExtent - _kBottomSlack;

  void _toBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _scroll.hasClients) {
        _scroll.jumpTo(_scroll.position.maxScrollExtent);
      }
    });
  }

  void _setLevel(String level) {
    _level = level;
    _load();
  }

  void _togglePause() => setState(() => _paused = !_paused);

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Breadcrumbs(<Crumb>[
          Crumb(Strings.adminHeading, route: adminRoute()),
          Crumb(Strings.adminServer, route: adminServerRoute()),
          const Crumb(Strings.serverLogsCrumb),
        ]),
        const SizedBox(height: Space.s4),
        SectionBlock(
          title: Strings.adminServerLogs,
          actions: <Widget>[
            LogLevelDropdown(value: _level, onChanged: _setLevel),
          ],
          actionItems: <PageAction>[
            PageAction(
              icon: _paused ? Icons.play_arrow : Icons.pause,
              label: _paused ? Strings.resumeLogs : Strings.pauseLogs,
              onPressed: _togglePause,
            ),
          ],
          child: _body(context),
        ),
      ],
    );
  }

  Widget _body(BuildContext context) {
    if (_loading) {
      return const StatusText(Strings.loadingLogs);
    }
    if (_failed) {
      return const StatusText(Strings.couldNotLoadLogs);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        if (_older.isEmpty && _newer.isEmpty)
          const StatusText(Strings.emptyLogs)
        else
          _entries(context),
        if (_pollFailed) const MutedNote(Strings.couldNotRefreshLogs),
      ],
    );
  }

  Widget _entries(BuildContext context) {
    final Tokens t = context.tokens;
    final double height = (MediaQuery.sizeOf(context).height * 0.7).clamp(
      320.0,
      900.0,
    );
    return Container(
      height: height,
      width: double.infinity,
      padding: const EdgeInsets.all(Space.s3),
      decoration: BoxDecoration(
        color: t.bg,
        borderRadius: const BorderRadius.all(Radii.sm),
        border: Border.all(color: t.border),
      ),
      child: SelectionArea(
        child: CustomScrollView(
          controller: _scroll,
          center: _center,
          slivers: <Widget>[
            SliverList.builder(
              itemCount: _older.length + 1,
              itemBuilder: (BuildContext context, int index) =>
                  index == _older.length ? _top() : LogLine(_older[index]),
            ),
            SliverList.builder(
              key: _center,
              itemCount: _newer.length,
              itemBuilder: (BuildContext context, int index) =>
                  LogLine(_newer[index]),
            ),
          ],
        ),
      ),
    );
  }

  Widget _top() {
    if (_loadingOlder) {
      return const MutedNote(Strings.loadingOlderLogs);
    }
    if (_olderFailed) {
      return Align(
        alignment: Alignment.centerLeft,
        child: TextButton(
          onPressed: _retryOlder,
          child: const Text(Strings.couldNotLoadOlderLogs),
        ),
      );
    }
    if (_before == null) {
      return const MutedNote(Strings.startOfLogs);
    }
    return const SizedBox.shrink();
  }
}
