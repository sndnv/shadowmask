import 'package:flutter/material.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/catalog_api.dart';
import 'package:shadowmask/api/play_on_store.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/catalog/version_detail.dart';
import 'package:shadowmask/nav/routes.dart';
import 'package:shadowmask/util/command_listener.dart';
import 'package:shadowmask/view/play_on_receiver.dart';

class PlayOnScope extends InheritedNotifier<PlayOnReceiver> {
  const PlayOnScope({
    super.key,
    required PlayOnReceiver receiver,
    required super.child,
  }) : super(notifier: receiver);

  static PlayOnReceiver? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<PlayOnScope>()?.notifier;

  static PlayOnReceiver? read(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PlayOnScope>()?.notifier;
}

class PlayOnHost extends StatefulWidget {
  const PlayOnHost({
    super.key,
    required this.api,
    required this.navigator,
    required this.child,
    this.store = const PlayOnStore(),
    this.bind,
    this.addresses,
  });

  final ApiClient api;
  final GlobalKey<NavigatorState> navigator;
  final Widget child;
  final PlayOnStore store;
  final ListenerBinder? bind;
  final AddressLister? addresses;

  @override
  State<PlayOnHost> createState() => _PlayOnHostState();
}

class _PlayOnHostState extends State<PlayOnHost> with WidgetsBindingObserver {
  late final PlayOnReceiver _receiver = PlayOnReceiver(
    api: widget.api,
    onPlay: _open,
    onRefused: _refused,
    store: widget.store,
    bind: widget.bind,
    addresses: widget.addresses,
  );
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _receiver.load();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _receiver.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _receiver.resume();
    }
    if (state == AppLifecycleState.paused) {
      _receiver.pause();
    }
  }

  Future<void> _open(String versionId) async {
    final int generation = ++_generation;
    final VersionDetail detail;
    try {
      detail = await CatalogApi(widget.api).version(versionId);
    } catch (_) {
      if (generation == _generation) {
        _refused();
      }
      return;
    }
    if (generation != _generation) {
      return;
    }
    final NavigatorState? navigator = widget.navigator.currentState;
    if (navigator == null) {
      return;
    }
    navigator
      ..pushNamedAndRemoveUntil(homeRoute(), (Route<dynamic> _) => false)
      ..pushNamed(titleRoute(detail.title.type, detail.title.id))
      ..pushNamed(watchRoute(versionId));
  }

  void _refused() {
    if (mounted) {
      Toasts.of(context).error(Strings.playOnCommandRefused);
    }
  }

  @override
  Widget build(BuildContext context) =>
      PlayOnScope(receiver: _receiver, child: widget.child);
}
