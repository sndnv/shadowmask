import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/components/admin/confirm_dialog.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/theme/app_theme.dart';
import 'package:shadowmask/theme/app_theme_variant.dart';
import 'package:shadowmask/theme/tokens.dart';

class _Answer {
  bool? value;
}

Future<_Answer> _open(
  WidgetTester tester, {
  String confirmLabel = Strings.delete,
  bool danger = true,
  double width = 900,
}) async {
  tester.view.physicalSize = Size(width, 700);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final _Answer answer = _Answer();
  await tester.pumpWidget(
    MaterialApp(
      theme: buildTheme(AppThemeVariant.dark),
      home: Scaffold(
        body: Builder(
          builder: (BuildContext context) => TextButton(
            onPressed: () async {
              answer.value = await confirmDialog(
                context,
                title: Strings.deleteUser,
                message: Strings.confirmDeleteUser('ada'),
                confirmLabel: confirmLabel,
                danger: danger,
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  return answer;
}

Color? _fillOf(WidgetTester tester, String label) => tester
    .widget<FilledButton>(find.widgetWithText(FilledButton, label))
    .style
    ?.backgroundColor
    ?.resolve(<WidgetState>{});

void main() {
  testWidgets('the destructive button is not the only one offered', (
    WidgetTester tester,
  ) async {
    await _open(tester);

    // Declining used to mean finding the grey X in the dialog header.
    expect(find.widgetWithText(OutlinedButton, Strings.cancel), findsOneWidget);
    expect(find.widgetWithText(FilledButton, Strings.delete), findsOneWidget);
    expect(_fillOf(tester, Strings.delete), Tokens.dark.danger);
  });

  testWidgets('cancel answers no', (WidgetTester tester) async {
    final _Answer answer = await _open(tester);

    await tester.tap(find.widgetWithText(OutlinedButton, Strings.cancel));
    await tester.pumpAndSettle();

    expect(answer.value, isFalse);
    expect(find.text(Strings.deleteUser), findsNothing);
  });

  testWidgets('confirming answers yes', (WidgetTester tester) async {
    final _Answer answer = await _open(tester);

    await tester.tap(find.widgetWithText(FilledButton, Strings.delete));
    await tester.pumpAndSettle();

    expect(answer.value, isTrue);
  });

  testWidgets('dismissing without answering counts as no', (
    WidgetTester tester,
  ) async {
    final _Answer answer = await _open(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    expect(answer.value, isFalse, reason: 'the safe default on every path out');
  });

  testWidgets('a non-destructive confirm keeps the ordinary fill', (
    WidgetTester tester,
  ) async {
    await _open(tester, confirmLabel: Strings.refreshMetadata, danger: false);

    expect(_fillOf(tester, Strings.refreshMetadata), isNull);
    expect(find.widgetWithText(OutlinedButton, Strings.cancel), findsOneWidget);
  });

  testWidgets('a phone fits both buttons without overflowing', (
    WidgetTester tester,
  ) async {
    await _open(tester, width: 360);

    final Rect cancel = tester.getRect(
      find.widgetWithText(OutlinedButton, Strings.cancel),
    );
    final Rect confirm = tester.getRect(
      find.widgetWithText(FilledButton, Strings.delete),
    );

    // A second button is the change most likely to burst a 360px dialog, so
    // the footer wraps rather than laying out in a fixed row.
    expect(confirm.top, greaterThanOrEqualTo(cancel.top));
    expect(confirm.right, lessThanOrEqualTo(360));
    expect(tester.takeException(), isNull);
  });

  testWidgets('the danger button is not the first thing focus reaches', (
    WidgetTester tester,
  ) async {
    await _open(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();

    expect(
      FocusManager.instance.primaryFocus?.context?.widget,
      isNot(isA<FilledButton>()),
    );
  });
}
