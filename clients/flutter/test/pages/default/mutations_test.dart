import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/toast_host.dart';
import 'package:shadowmask/pages/default/mutations.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';

class _Host extends StatefulWidget {
  const _Host({required this.action, this.itemKey});

  final Future<void> Function() action;
  final Object? itemKey;

  @override
  State<_Host> createState() => _HostState();
}

class _HostState extends State<_Host> with Mutations<_Host> {
  int done = 0;

  @override
  Widget build(BuildContext context) => Column(
    children: <Widget>[
      TextButton(
        onPressed: busy(widget.itemKey)
            ? null
            : () => mutate(
                key: widget.itemKey,
                widget.action,
                successText: 'Saved.',
                errorText: 'Could not save.',
                then: () => done++,
              ),
        child: const Text('go'),
      ),
      Text('done $done'),
    ],
  );
}

Future<_HostState> _pump(
  WidgetTester tester,
  Future<void> Function() action, {
  Object? itemKey,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: ToastHost(
          child: _Host(action: action, itemKey: itemKey),
        ),
      ),
    ),
  );
  return tester.state<_HostState>(find.byType(_Host));
}

void main() {
  testWidgets('the control is dead while its request is in flight', (
    WidgetTester tester,
  ) async {
    final Completer<void> gate = Completer<void>();
    int calls = 0;
    await _pump(tester, () {
      calls++;
      return gate.future;
    });

    await tester.tap(find.text('go'));
    await tester.pump();

    expect(
      tester.widget<TextButton>(find.byType(TextButton)).onPressed,
      isNull,
    );

    // A second click on a slow request is the whole defect: one toast says it
    // worked and the other says it failed.
    await tester.tap(find.text('go'), warnIfMissed: false);
    await tester.pump();
    expect(calls, 1);

    gate.complete();
    await tester.pump();

    expect(
      tester.widget<TextButton>(find.byType(TextButton)).onPressed,
      isNotNull,
    );
    expect(find.text('Saved.'), findsOneWidget);
    expect(find.text('done 1'), findsOneWidget);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a failure toasts the reason and skips the follow-up', (
    WidgetTester tester,
  ) async {
    await _pump(tester, () async {
      await Future<void>.delayed(Duration.zero);
      throw Exception('nope');
    });

    await tester.tap(find.text('go'));
    await tester.pumpAndSettle();

    expect(find.textContaining('Could not save.'), findsOneWidget);
    expect(find.text('done 0'), findsOneWidget);
    expect(
      tester.widget<TextButton>(find.byType(TextButton)).onPressed,
      isNotNull,
      reason: 'a failed mutation has to leave the control usable',
    );

    await tester.pump(kErrorToastDuration + const Duration(milliseconds: 100));
  });

  testWidgets('a keyed mutation only blocks its own row', (
    WidgetTester tester,
  ) async {
    final Completer<void> gate = Completer<void>();
    final _HostState state = await _pump(
      tester,
      () => gate.future,
      itemKey: 'row-1',
    );

    await tester.tap(find.text('go'));
    await tester.pump();

    expect(state.busy('row-1'), isTrue);
    expect(state.busy('row-2'), isFalse);
    expect(state.busy(), isTrue, reason: 'the block as a whole is mutating');

    gate.complete();
    await tester.pump();
    expect(state.busy(), isFalse);

    await tester.pump(kToastDuration + const Duration(milliseconds: 100));
  });
}
