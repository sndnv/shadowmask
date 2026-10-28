import 'dart:async';

import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_dotenv/flutter_dotenv.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/capability_scope.dart';
import 'package:shadowmask/api/child_mode_scope.dart';
import 'package:shadowmask/api/child_mode_store.dart';
import 'package:shadowmask/api/server_probe.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/api/server_store.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/app_router.dart';
import 'package:shadowmask/components/backdrop_scope.dart';
import 'package:shadowmask/components/page_title.dart';
import 'package:shadowmask/components/play_on_host.dart';
import 'package:shadowmask/components/remote_image.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/tooltip_dismisser.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/api/capability_overrides_store.dart';
import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';
import 'package:shadowmask/nav/browser_history.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/theme_store.dart';
import 'package:shadowmask/util/api_base.dart';
import 'package:shadowmask/util/bundled_fonts.dart';
import 'package:shadowmask/util/bundled_licenses.dart';
import 'package:shadowmask/util/page_director.dart';
import 'package:shadowmask/util/client_capabilities.dart';
import 'package:shadowmask/util/client_platform.dart';
import 'package:shadowmask/util/desktop_entry.dart';
import 'package:shadowmask/util/url_strategy.dart';
import 'package:shadowmask/util/window.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    await BrowserContextMenu.disableContextMenu();
  }
  LicenseRegistry.addLicense(
    () => bundledLicenseEntries(bundlePlatform(), rootBundle),
  );
  await loadBundledFonts(bundlePlatform(), rootBundle);
  await dotenv.load(fileName: '.env', isOptional: true);
  usePathUrlStrategy();
  remoteImage = kIsWeb ? NetworkImage.new : CachedNetworkImageProvider.new;
  await configureWindow();
  unawaited(refreshDesktopEntry());
  await initializePlayer();
  const ThemeStore themeStore = ThemeStore();
  const ServerStore serverStore = ServerStore();
  final AppThemeVariant variant = await themeStore.load();
  final bool? highContrast = await themeStore.loadHighContrast();
  final String? stored = await serverStore.load();
  final ClientDecoding? decoding = await measureDecoding();
  const CapabilityOverridesStore capabilityStore = CapabilityOverridesStore();
  final CapabilityOverrides overrides = await capabilityStore.load();
  const ChildModeStore childModeStore = ChildModeStore();
  final int? childAge = await childModeStore.load();
  runApp(
    ShadowmaskApp(
      initialServer: stored ?? _resolveApiBase(),
      initialVariant: variant,
      initialHighContrast: highContrast,
      themeStore: themeStore,
      serverStore: serverStore,
      capabilityStore: capabilityStore,
      decoding: decoding,
      initialOverrides: overrides,
      initialChildAge: childAge,
      childModeStore: childModeStore,
    ),
  );
}

String? _resolveApiBase() => apiBaseFrom(
  define: const String.fromEnvironment('SHADOWMASK_API_BASE'),
  env: dotenv.maybeGet('SHADOWMASK_API_BASE'),
);

ApiClient _defaultClient(String? baseUrl) => ApiClient(baseUrl: baseUrl);

bool _androidApp() => !kIsWeb && clientPlatform() == 'android';

class ShadowmaskApp extends StatefulWidget {
  const ShadowmaskApp({
    super.key,
    this.initialServer,
    this.clientFactory = _defaultClient,
    this.probe = serverAnswers,
    this.initialVariant = AppThemeVariant.retro,
    this.initialHighContrast,
    this.themeStore = const ThemeStore(),
    this.serverStore = const ServerStore(),
    this.capabilityStore = const CapabilityOverridesStore(),
    this.decoding,
    this.initialOverrides = const CapabilityOverrides(),
    this.receivesCommands = _androidApp,
    this.initialChildAge,
    this.childModeStore = const ChildModeStore(),
  });

  final String? initialServer;
  final ApiClient Function(String? baseUrl) clientFactory;
  final Future<bool> Function(String address) probe;
  final AppThemeVariant initialVariant;
  final bool? initialHighContrast;
  final ThemeStore themeStore;
  final ServerStore serverStore;
  final CapabilityOverridesStore capabilityStore;
  final ClientDecoding? decoding;
  final CapabilityOverrides initialOverrides;
  final bool Function() receivesCommands;
  final int? initialChildAge;
  final ChildModeStore childModeStore;

  @override
  State<ShadowmaskApp> createState() => _ShadowmaskAppState();
}

class _ShadowmaskAppState extends State<ShadowmaskApp>
    with WidgetsBindingObserver {
  late AppThemeVariant _variant = widget.initialVariant;
  late bool? _highContrast = widget.initialHighContrast;
  late String? _server = widget.initialServer;
  late CapabilityOverrides _overrides = widget.initialOverrides;
  late int? _childAge = widget.initialChildAge;
  int _reloads = 0;
  late ApiClient _api = _clientFor(_server);
  late AppRouter _router = AppRouter(_api);
  GlobalKey<NavigatorState> _navigator = GlobalKey<NavigatorState>();
  late BrowserHistoryProvider _routeInformation = BrowserHistoryProvider();
  late BrowserHistory _history = _historyFor(_navigator);
  late bool _systemHighContrast = WidgetsBinding
      .instance
      .platformDispatcher
      .accessibilityFeatures
      .highContrast;
  late PageDirector<String?> _backdrops = _backdropDirector();
  PageDirector<String?> _titles = PageDirector<String?>(null);

  bool get _contrast => _highContrast ?? _systemHighContrast;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _backdrops.dispose();
    _titles.dispose();
    super.dispose();
  }

  @override
  void didChangeAccessibilityFeatures() {
    final bool value = WidgetsBinding
        .instance
        .platformDispatcher
        .accessibilityFeatures
        .highContrast;
    if (value != _systemHighContrast) {
      setState(() => _systemHighContrast = value);
    }
  }

  void _setVariant(AppThemeVariant variant) {
    if (variant == _variant) {
      return;
    }
    setState(() => _variant = variant);
    widget.themeStore.save(variant);
  }

  void _setHighContrast(bool value) {
    if (value == _highContrast) {
      return;
    }
    setState(() => _highContrast = value);
    widget.themeStore.saveHighContrast(value);
  }

  void _setOverrides(CapabilityOverrides overrides) {
    if (overrides == _overrides) {
      return;
    }
    setState(() => _overrides = overrides);
    widget.capabilityStore.save(overrides);
  }

  ApiClient _clientFor(String? server) =>
      widget.clientFactory(server)..childAge = _childAge;

  Future<void> _setChildAge(int? age) async {
    if (age == _childAge) {
      return;
    }
    await widget.childModeStore.save(age);
    if (!mounted) {
      return;
    }
    setState(() {
      _childAge = age;
      _api.childAge = age;
      _reloads += 1;
      _navigator = GlobalKey<NavigatorState>();
      _routeInformation = BrowserHistoryProvider(initialRoute: '/');
      _renewPages();
      _history = _historyFor(_navigator);
    });
    final GlobalKey<NavigatorState> reloaded = _navigator;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final BuildContext? context = reloaded.currentContext;
      if (context != null) {
        Toasts.of(context).success(childModeToast(age));
      }
    });
  }

  bool get _needsServer => !kIsWeb && _server == null;

  Future<void> _setServer(String address) async {
    await widget.serverStore.save(address);
    await TokenStore().clear();
    if (!mounted) {
      return;
    }
    setState(() {
      _server = address;
      _api = _clientFor(address);
      _router = AppRouter(_api);
      _navigator = GlobalKey<NavigatorState>();
      _routeInformation = BrowserHistoryProvider();
      _renewPages();
      _history = _historyFor(_navigator);
    });
  }

  PageDirector<String?> _backdropDirector() => PageDirector<String?>(
    null,
    prepare: (String? url) => precacheBackdrop(url, context),
  );

  void _renewPages() {
    final PageDirector<String?> backdrops = _backdrops;
    final PageDirector<String?> titles = _titles;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      backdrops.dispose();
      titles.dispose();
    });
    _backdrops = _backdropDirector();
    _titles = PageDirector<String?>(null);
  }

  BrowserHistory _historyFor(GlobalKey<NavigatorState> navigator) =>
      BrowserHistory(
        navigatorKey: navigator,
        onGenerateRoute: (RouteSettings settings) =>
            _router.router.generator(settings),
        onGenerateInitialRoutes: _initialRoutes,
        observers: <NavigatorObserver>[
          appRouteObserver,
          _backdrops.observer,
          _titles.observer,
        ],
      );

  @override
  Widget build(BuildContext context) {
    final Widget app = _needsServer ? _entry() : _app();
    return ThemeScope(
      variant: _variant,
      setVariant: _setVariant,
      highContrast: _contrast,
      setHighContrast: _setHighContrast,
      child: CapabilityScope(
        platform: clientPlatform(),
        measured: widget.decoding,
        overrides: _overrides,
        setOverrides: _setOverrides,
        child: BackdropScope(
          backdrops: _backdrops,
          child: PageTitleScope(
            titles: _titles,
            child: ChildModeScope(
              age: _childAge,
              setAge: _setChildAge,
              child: kIsWeb
                  ? app
                  : ServerScope(
                      address: _server,
                      setAddress: _setServer,
                      probe: widget.probe,
                      child: app,
                    ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _entry() => MaterialApp(
    title: Strings.appTitle,
    debugShowCheckedModeBanner: false,
    theme: buildTheme(_variant, highContrast: _contrast),
    builder: (BuildContext context, Widget? child) => TooltipDismisser(
      child: ToastHost(child: child ?? const SizedBox.shrink()),
    ),
    home: const ServerPage(),
  );

  Widget _app() => MaterialApp.router(
    key: ValueKey<(String?, int)>((_server, _reloads)),
    title: Strings.appTitle,
    debugShowCheckedModeBanner: false,
    theme: buildTheme(_variant, highContrast: _contrast),
    builder: (BuildContext context, Widget? child) => TabTitle(
      titles: _titles,
      child: TooltipDismisser(
        child: ToastHost(child: _commands(child ?? const SizedBox.shrink())),
      ),
    ),
    routerDelegate: _history,
    routeInformationParser: const BrowserHistoryParser(),
    routeInformationProvider: _routeInformation,
  );

  Widget _commands(Widget child) => widget.receivesCommands()
      ? PlayOnHost(api: _api, navigator: _navigator, child: child)
      : child;

  List<Route<dynamic>> _initialRoutes(String initialRoute) {
    final Route<dynamic> route =
        _router.router.generator(RouteSettings(name: initialRoute)) ??
        _router.router.generator(const RouteSettings(name: '/'))!;
    return <Route<dynamic>>[route];
  }
}
