import 'package:flutter/material.dart';
import 'package:flutter_dotenv/flutter_dotenv.dart';
import 'package:flutter_web_plugins/url_strategy.dart';

import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/app_router.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/nav/route_observer.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/theme_store.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await dotenv.load(fileName: '.env', isOptional: true);
  usePathUrlStrategy();
  const ThemeStore themeStore = ThemeStore();
  final AppThemeVariant variant = await themeStore.load();
  final bool? highContrast = await themeStore.loadHighContrast();
  runApp(
    ShadowmaskApp(
      api: ApiClient(baseUrl: _resolveApiBase()),
      initialVariant: variant,
      initialHighContrast: highContrast,
      themeStore: themeStore,
    ),
  );
}

String? _resolveApiBase() {
  final String? value = dotenv.maybeGet('SHADOWMASK_API_BASE');
  return value != null && value.isNotEmpty ? value : null;
}

class ShadowmaskApp extends StatefulWidget {
  const ShadowmaskApp({
    super.key,
    required this.api,
    this.initialVariant = AppThemeVariant.dark,
    this.initialHighContrast,
    this.themeStore = const ThemeStore(),
  });

  final ApiClient api;
  final AppThemeVariant initialVariant;
  final bool? initialHighContrast;
  final ThemeStore themeStore;

  @override
  State<ShadowmaskApp> createState() => _ShadowmaskAppState();
}

class _ShadowmaskAppState extends State<ShadowmaskApp>
    with WidgetsBindingObserver {
  late AppThemeVariant _variant = widget.initialVariant;
  late bool? _highContrast = widget.initialHighContrast;
  late final AppRouter _router = AppRouter(widget.api);
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

  @override
  Widget build(BuildContext context) {
    return ThemeScope(
      variant: _variant,
      setVariant: _setVariant,
      highContrast: _contrast,
      setHighContrast: _setHighContrast,
      child: MaterialApp(
        title: Strings.appTitle,
        debugShowCheckedModeBanner: false,
        theme: buildTheme(_variant, highContrast: _contrast),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        navigatorObservers: <NavigatorObserver>[appRouteObserver],
        onGenerateRoute: _router.router.generator,
        onGenerateInitialRoutes: _initialRoutes,
      ),
    );
  }

  List<Route<dynamic>> _initialRoutes(String initialRoute) {
    final Route<dynamic> route =
        _router.router.generator(RouteSettings(name: initialRoute)) ??
        _router.router.generator(const RouteSettings(name: '/'))!;
    return <Route<dynamic>>[route];
  }
}
