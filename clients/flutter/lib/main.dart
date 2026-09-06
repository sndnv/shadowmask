import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_dotenv/flutter_dotenv.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/api/server_probe.dart';
import 'package:shadowmask/api/server_scope.dart';
import 'package:shadowmask/api/server_store.dart';
import 'package:shadowmask/api/token_store.dart';
import 'package:shadowmask/app_router.dart';
import 'package:shadowmask/components/remote_image.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/components/tooltip_dismisser.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/pages/entry/server_page.dart';
import 'package:shadowmask/player/player_controller.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/theme_store.dart';
import 'package:shadowmask/util/url_strategy.dart';
import 'package:shadowmask/util/window.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await dotenv.load(fileName: '.env', isOptional: true);
  usePathUrlStrategy();
  remoteImage = kIsWeb ? NetworkImage.new : CachedNetworkImageProvider.new;
  await configureWindow();
  await initializePlayer();
  const ThemeStore themeStore = ThemeStore();
  const ServerStore serverStore = ServerStore();
  final AppThemeVariant variant = await themeStore.load();
  final bool? highContrast = await themeStore.loadHighContrast();
  final String? stored = await serverStore.load();
  runApp(
    ShadowmaskApp(
      initialServer: stored ?? _resolveApiBase(),
      initialVariant: variant,
      initialHighContrast: highContrast,
      themeStore: themeStore,
      serverStore: serverStore,
    ),
  );
}

String? _resolveApiBase() {
  final String? value = dotenv.maybeGet('SHADOWMASK_API_BASE');
  return value != null && value.isNotEmpty ? value : null;
}

ApiClient _defaultClient(String? baseUrl) => ApiClient(baseUrl: baseUrl);

class ShadowmaskApp extends StatefulWidget {
  const ShadowmaskApp({
    super.key,
    this.initialServer,
    this.clientFactory = _defaultClient,
    this.probe = serverAnswers,
    this.initialVariant = AppThemeVariant.dark,
    this.initialHighContrast,
    this.themeStore = const ThemeStore(),
    this.serverStore = const ServerStore(),
  });

  final String? initialServer;
  final ApiClient Function(String? baseUrl) clientFactory;
  final Future<bool> Function(String address) probe;
  final AppThemeVariant initialVariant;
  final bool? initialHighContrast;
  final ThemeStore themeStore;
  final ServerStore serverStore;

  @override
  State<ShadowmaskApp> createState() => _ShadowmaskAppState();
}

class _ShadowmaskAppState extends State<ShadowmaskApp>
    with WidgetsBindingObserver {
  late AppThemeVariant _variant = widget.initialVariant;
  late bool? _highContrast = widget.initialHighContrast;
  late String? _server = widget.initialServer;
  late ApiClient _api = widget.clientFactory(_server);
  late AppRouter _router = AppRouter(_api);
  late bool _systemHighContrast = WidgetsBinding
      .instance
      .platformDispatcher
      .accessibilityFeatures
      .highContrast;

  bool get _contrast => _highContrast ?? _systemHighContrast;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
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

  bool get _needsServer => !kIsWeb && _server == null;

  Future<void> _setServer(String address) async {
    await widget.serverStore.save(address);
    await TokenStore().clear();
    if (!mounted) {
      return;
    }
    setState(() {
      _server = address;
      _api = widget.clientFactory(address);
      _router = AppRouter(_api);
    });
  }

  @override
  Widget build(BuildContext context) {
    final Widget app = _needsServer ? _entry() : _app();
    return ThemeScope(
      variant: _variant,
      setVariant: _setVariant,
      highContrast: _contrast,
      setHighContrast: _setHighContrast,
      child: kIsWeb
          ? app
          : ServerScope(
              address: _server,
              setAddress: _setServer,
              probe: widget.probe,
              child: app,
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

  Widget _app() => MaterialApp(
    key: ValueKey<String?>(_server),
    title: Strings.appTitle,
    debugShowCheckedModeBanner: false,
    theme: buildTheme(_variant, highContrast: _contrast),
    builder: (BuildContext context, Widget? child) => TooltipDismisser(
      child: ToastHost(child: child ?? const SizedBox.shrink()),
    ),
    navigatorObservers: <NavigatorObserver>[appRouteObserver],
    onGenerateRoute: _router.router.generator,
    onGenerateInitialRoutes: _initialRoutes,
  );

  List<Route<dynamic>> _initialRoutes(String initialRoute) {
    final Route<dynamic> route =
        _router.router.generator(RouteSettings(name: initialRoute)) ??
        _router.router.generator(const RouteSettings(name: '/'))!;
    return <Route<dynamic>>[route];
  }
}
