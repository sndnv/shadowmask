import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shadowmask/api/api_client.dart';
import 'package:shadowmask/components/admin/admin_table.dart';
import 'package:shadowmask/components/admin/danger_icon_button.dart';
import 'package:shadowmask/components/timestamp_text.dart';
import 'package:shadowmask/model/user/account_profile.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/pages/admin/users_page.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/theme_scope.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../support/finders.dart';

Map<String, dynamic> _user(
  String id,
  String name, {
  String role = 'user',
  bool active = true,
}) => <String, dynamic>{
  'id': id,
  'username': name,
  'role': role,
  'active': active,
  'created_at': '2026-08-17T07:24:11Z',
  'updated_at': '2026-08-17T07:24:11Z',
};

Future<void> _pump(WidgetTester tester, ApiClient api) async {
  tester.view.physicalSize = const Size(1600, 1200);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ThemeScope(
      variant: AppThemeVariant.dark,
      setVariant: (_) {},
      child: MaterialApp(
        theme: buildTheme(AppThemeVariant.dark),
        builder: (BuildContext context, Widget? child) =>
            ToastHost(child: child ?? const SizedBox.shrink()),
        onGenerateRoute: (_) =>
            MaterialPageRoute<void>(builder: (_) => UsersPage(api: api)),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

ApiClient _api({
  void Function(Map<String, dynamic>)? onCreate,
  List<Map<String, dynamic>>? listed,
  int total = 1,
  String? createFailsWith,
  void Function(String)? onActive,
}) => ApiClient(
  baseUrl: 'http://test',
  httpClient: MockClient((http.Request req) async {
    if (req.url.path.endsWith('/active') && req.method == 'PUT') {
      final String id = req.url.pathSegments[req.url.pathSegments.length - 2];
      final Map<String, dynamic> body =
          jsonDecode(req.body) as Map<String, dynamic>;
      onActive?.call('$id:${body['active']}');
      return http.Response(jsonEncode(_user(id, 'sam')), 200);
    }
    if (req.url.path == '/api/v1/users/self') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'id': 'u1',
          'username': 'pat',
          'role': 'admin',
        }),
        200,
      );
    }
    if (req.url.path == '/api/v1/users' && req.method == 'POST') {
      onCreate?.call(jsonDecode(req.body) as Map<String, dynamic>);
      if (createFailsWith != null) {
        return http.Response(
          jsonEncode(<String, dynamic>{
            'error': <String, dynamic>{
              'code': createFailsWith,
              'message': 'from the server, for an operator',
            },
          }),
          409,
        );
      }
      return http.Response(jsonEncode(_user('u2', 'sam')), 201);
    }
    if (req.url.path == '/api/v1/users') {
      return http.Response(
        jsonEncode(<String, dynamic>{
          'items': listed ?? <dynamic>[_user('u1', 'pat')],
          'total': total,
          'offset': 0,
          'limit': 20,
        }),
        200,
      );
    }
    return http.Response('{}', 200);
  }),
);

Finder _deleteIn(String username) => find.descendant(
  of: find
      .ancestor(
        of: find.descendant(
          of: find.byType(AdminTable<AccountProfile>),
          matching: find.text(username),
        ),
        matching: find.byType(DecoratedBox),
      )
      .first,
  matching: find.byType(DangerIconButton),
);

Finder _activeIn(String username) => find
    .descendant(
      of: find
          .ancestor(
            of: find.descendant(
              of: find.byType(AdminTable<AccountProfile>),
              matching: find.text(username),
            ),
            matching: find.byType(DecoratedBox),
          )
          .first,
      matching: find.byType(IconButton),
    )
    .first;

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  testWidgets('the created column is a formatted local timestamp', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api());

    expect(find.byType(TimestampText), findsOneWidget);
    expect(find.textContaining('2026-08-17'), findsOneWidget);
    expect(find.text('2026-08-17T07:24:11Z'), findsNothing);
  });

  testWidgets('the crumb counts every user, not the rows on this page', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[_user('u1', 'pat'), _user('u2', 'sam')],
        total: 7,
      ),
    );

    expect(
      find.text(Strings.countLabel(Strings.adminUsers, 7)),
      findsOneWidget,
    );
    expect(find.text(Strings.adminUsers), findsNothing);
  });

  testWidgets('users arrive sorted by username without a header tap', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[_user('u1', 'zoe'), _user('u2', 'ada')],
        total: 2,
      ),
    );

    final double ada = tester.getTopLeft(find.text('ada')).dy;
    final double zoe = tester.getTopLeft(find.text('zoe')).dy;
    expect(ada, lessThan(zoe));
  });

  testWidgets('a duplicate username reports inline, not as a toast', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(createFailsWith: 'username_taken'));

    await tester.tap(find.widgetWithText(FilledButton, Strings.createUser));
    await tester.pumpAndSettle();

    await tester.enterText(fieldNamed(Strings.fieldUsername), 'pat');
    await tester.enterText(fieldNamed(Strings.fieldPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldRepeatPassword), 'hunter2');
    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(find.text(Strings.reasonUsernameTaken), findsOneWidget);
    expect(find.text(Strings.errorCreate), findsNothing);
    expect(find.textContaining('for an operator'), findsNothing);

    await tester.enterText(fieldNamed(Strings.fieldUsername), 'sam');
    await tester.pumpAndSettle();
    expect(find.text(Strings.reasonUsernameTaken), findsNothing);
  });

  testWidgets('a failure that is not field-scoped toasts with its reason', (
    WidgetTester tester,
  ) async {
    await _pump(tester, _api(createFailsWith: 'feature_disabled'));

    await tester.tap(find.widgetWithText(FilledButton, Strings.createUser));
    await tester.pumpAndSettle();

    await tester.enterText(fieldNamed(Strings.fieldUsername), 'sam');
    await tester.enterText(fieldNamed(Strings.fieldPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldRepeatPassword), 'hunter2');
    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(
      find.text('${Strings.errorCreate} ${Strings.reasonFeatureDisabled}'),
      findsOneWidget,
    );
    expect(find.textContaining('for an operator'), findsNothing);

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('the row tint marks admins and players, not plain users', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[
          _user('u1', 'ada', role: 'admin'),
          _user('u2', 'sam', role: 'player'),
          _user('u3', 'kim'),
        ],
      ),
    );

    expect(rowTintOf(tester, 'ada'), Tokens.dark.rowDanger);
    expect(rowTintOf(tester, 'sam'), Tokens.dark.rowWarn);
    expect(rowTintOf(tester, 'kim'), isNull);
  });

  testWidgets('an admin cannot delete the account they are signed in with', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[
          _user('u1', 'pat', role: 'admin'),
          _user('u2', 'sam'),
        ],
        total: 2,
      ),
    );

    final DangerIconButton mine = tester.widget<DangerIconButton>(
      _deleteIn('pat'),
    );
    final DangerIconButton theirs = tester.widget<DangerIconButton>(
      _deleteIn('sam'),
    );

    expect(mine.onPressed, isNull);
    expect(mine.tooltip, Strings.cannotDeleteSelf);
    expect(theirs.onPressed, isNotNull);
    expect(theirs.tooltip, Strings.deleteUser);
  });

  testWidgets('an admin cannot deactivate themselves but can others', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[
          _user('u1', 'pat', role: 'admin'),
          _user('u2', 'sam'),
        ],
        total: 2,
      ),
    );

    final IconButton mine = tester.widget<IconButton>(_activeIn('pat'));
    final IconButton theirs = tester.widget<IconButton>(_activeIn('sam'));

    expect(mine.onPressed, isNull);
    expect(mine.tooltip, Strings.cannotDeactivateSelf);
    expect(theirs.onPressed, isNotNull);
    expect(theirs.tooltip, Strings.deactivateUser);
  });

  testWidgets('an inactive account shows its status and offers activation', (
    WidgetTester tester,
  ) async {
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[
          _user('u1', 'pat', role: 'admin'),
          _user('u2', 'sam', active: false),
        ],
        total: 2,
      ),
    );

    expect(find.text(Strings.statusInactive), findsOneWidget);
    expect(find.text(Strings.statusActive), findsOneWidget);

    final IconButton theirs = tester.widget<IconButton>(_activeIn('sam'));
    expect(theirs.tooltip, Strings.activateUser);
    expect(theirs.onPressed, isNotNull);
  });

  testWidgets('deactivating asks first and then calls the server', (
    WidgetTester tester,
  ) async {
    final List<String> calls = <String>[];
    await _pump(
      tester,
      _api(
        listed: <Map<String, dynamic>>[
          _user('u1', 'pat', role: 'admin'),
          _user('u2', 'sam'),
        ],
        total: 2,
        onActive: calls.add,
      ),
    );

    await tester.tap(_activeIn('sam'));
    await tester.pumpAndSettle();

    expect(find.text(Strings.confirmDeactivateUser('sam')), findsOneWidget);
    expect(calls, isEmpty);

    await tester.tap(find.text(Strings.deactivate).last);
    await tester.pumpAndSettle();

    expect(calls, <String>['u2:false']);
    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a mismatched confirmation blocks the create, inline', (
    WidgetTester tester,
  ) async {
    Map<String, dynamic>? created;
    await _pump(
      tester,
      _api(onCreate: (Map<String, dynamic> b) => created = b),
    );

    await tester.tap(find.widgetWithText(FilledButton, Strings.createUser));
    await tester.pumpAndSettle();

    await tester.enterText(fieldNamed(Strings.fieldUsername), 'sam');
    await tester.enterText(fieldNamed(Strings.fieldPassword), 'hunter2');
    await tester.enterText(fieldNamed(Strings.fieldRepeatPassword), 'hunter3');
    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(find.text(Strings.passwordsDoNotMatch), findsOneWidget);
    expect(created, isNull);

    await tester.enterText(fieldNamed(Strings.fieldRepeatPassword), 'hunter2');
    await tester.pumpAndSettle();
    expect(find.text(Strings.passwordsDoNotMatch), findsNothing);

    await tester.tap(find.widgetWithText(FilledButton, Strings.create));
    await tester.pumpAndSettle();

    expect(created?['username'], 'sam');
    expect(created?['password'], 'hunter2');

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
