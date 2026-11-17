import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/view/ring_steps.dart';

void main() {
  test('no progress lights no step', () {
    expect(ringFilledSteps(0), 0);
  });

  test('any progress lights at least one step', () {
    expect(ringFilledSteps(1), 1);
  });

  test('a step lights once progress reaches into it', () {
    expect(ringFilledSteps(50), 20);
    expect(ringFilledSteps(64), 26);
    expect(ringFilledSteps(99), 40);
  });

  test('a finished title lights every step', () {
    expect(ringFilledSteps(100), kRingSteps);
  });

  test('a percent past either end stays on the ring', () {
    expect(ringFilledSteps(140), kRingSteps);
    expect(ringFilledSteps(-5), 0);
  });
}
