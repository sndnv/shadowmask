import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/scoped_value.dart';

Future<void> _frame(WidgetTester tester) async {
  tester.binding.scheduleFrame();
  await tester.pump();
}

void main() {
  testWidgets('publish notifies once and only on a change', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>(null);
    addTearDown(value.dispose);
    int notifications = 0;
    value.addListener(() => notifications++);

    value.publish('a');
    value.publish('a');
    expect(value.value, 'a');
    expect(notifications, 1);
  });

  testWidgets('a release lands after the frame, not during it', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>('a');
    addTearDown(value.dispose);

    value.releaseAfterFrame('a', null);
    expect(value.value, 'a');

    await _frame(tester);
    expect(value.value, isNull);
  });

  testWidgets('a release is dropped once someone else holds the value', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>('a');
    addTearDown(value.dispose);

    value.releaseAfterFrame('a', null);
    value.publish('b');
    await _frame(tester);

    expect(value.value, 'b');
  });

  testWidgets('a successor keeps the value its predecessor is releasing', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>(null);
    addTearDown(value.dispose);
    final Object leaving = Object();
    final Object arriving = Object();

    value.publish('a', owner: leaving);
    value.releaseAfterFrame('a', null, owner: leaving);
    value.publish('a', owner: arriving);
    await _frame(tester);

    expect(
      value.value,
      'a',
      reason:
          'the outgoing page schedules its release from dispose, and it fires '
          'after the incoming page has claimed the same value, so a release '
          'that only compares values would clobber the new owner',
    );
  });

  testWidgets('an owned release still lands when nobody takes over', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>(null);
    addTearDown(value.dispose);
    final Object leaving = Object();

    value.publish('a', owner: leaving);
    value.releaseAfterFrame('a', null, owner: leaving);
    await _frame(tester);

    expect(value.value, isNull);
  });

  testWidgets('a disposed value accepts neither a publish nor a release', (
    WidgetTester tester,
  ) async {
    final ScopedValue<String?> value = ScopedValue<String?>('a');
    value.releaseAfterFrame('a', null);
    value.dispose();

    expect(value.disposed, isTrue);
    value.publish('b');
    await _frame(tester);

    expect(tester.takeException(), isNull);
  });
}
