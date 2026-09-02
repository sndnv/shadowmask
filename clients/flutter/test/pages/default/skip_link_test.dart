import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/model/user/self_user.dart';
import 'package:shadowmask/nav/nav_section.dart';
import 'package:shadowmask/pages/default/shell_scaffold.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';

ApiClient _api() => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request _) async => http.Response('{}', 200)),
);

const SelfUser _user = SelfUser(id: 'u1', username: 'pat', role: UserRole.user);

Future<void> _pump(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        home: ShellScaffold(
          api: _api(),
          current: NavSection.movies,
          user: _user,
          body: Column(
            children: <Widget>[
              TextButton(onPressed: () {}, child: const Text('first in body')),
              TextButton(onPressed: () {}, child: const Text('second in body')),
            ],
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _tab(WidgetTester tester) async {
  await tester.sendKeyEvent(LogicalKeyboardKey.tab);
  await tester.pumpAndSettle();
}

String? _focusedText() {
  final BuildContext? context = FocusManager.instance.primaryFocus?.context;
  if (context == null) {
    return null;
  }
  final Iterable<Text> texts = context.widget is Text
      ? <Text>[context.widget as Text]
      : const <Text>[];
  if (texts.isNotEmpty) {
    return texts.first.data;
  }
  final List<String> found = <String>[];
  void visit(Element e) {
    if (e.widget is Text) {
      found.add((e.widget as Text).data ?? '');
    }
    e.visitChildren(visit);
  }

  (context as Element).visitChildren(visit);
  return found.isEmpty ? null : found.first;
}

void main() {
  testWidgets('the first tab stop is the skip link, ahead of the brand', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _tab(tester);

    expect(_focusedText(), Strings.skipToContent);
  });

  testWidgets('it stays out of sight until it is focused', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    expect(
      tester.getRect(find.text(Strings.skipToContent)).bottom,
      lessThan(0),
      reason: 'parked off the top of the page, still focusable',
    );

    await _tab(tester);

    expect(
      tester.getRect(find.text(Strings.skipToContent)).top,
      greaterThan(0),
    );
  });

  testWidgets('taking it lands the next tab inside the body', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _tab(tester);
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    // Without this a keyboard user tabs the brand and every nav item on
    // every page load before reaching anything they came for.
    await _tab(tester);

    expect(_focusedText(), 'first in body');
  });

  testWidgets('ignoring it leaves the nav in its usual order', (
    WidgetTester tester,
  ) async {
    await _pump(tester);

    await _tab(tester);
    await _tab(tester);

    expect(_focusedText(), isNot('first in body'));
    expect(_focusedText(), isNot(Strings.skipToContent));
  });
}
