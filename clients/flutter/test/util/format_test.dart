import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/util/format.dart';

void main() {
  test('dateTimeText renders local date and time to the minute', () {
    final DateTime local = DateTime(2026, 8, 17, 9, 24, 11);

    expect(dateTimeText(local.toUtc().toIso8601String()), '2026-08-17 09:24');
  });

  test('scoreText restores the scale each source measures on', () {
    expect(scoreText('Internet Movie Database', 8.1), '8.1/10');
    expect(scoreText('Rotten Tomatoes', 87), '87%');
    expect(scoreText('Metacritic', 72), '72/100');
  });

  test('scoreText leaves an unknown source as a bare number', () {
    expect(scoreText('Some Other Critic', 4.5), '4.5');
    expect(scoreText('Some Other Critic', 4), '4');
  });

  test('scoreSource shortens only the one name that needs it', () {
    expect(scoreSource('Internet Movie Database'), 'IMDb');
    expect(scoreSource('Rotten Tomatoes'), 'Rotten Tomatoes');
  });

  test('dateTimeText passes through what it cannot parse', () {
    expect(dateTimeText('not a date'), 'not a date');
    expect(dateTimeText(''), isNull);
    expect(dateTimeText(null), isNull);
  });

  test('relativeText counts up through the units', () {
    final DateTime now = DateTime(2026, 8, 17, 12);
    String? ago(DateTime at) =>
        relativeText(at.toUtc().toIso8601String(), now: now);

    expect(ago(now.subtract(const Duration(seconds: 20))), 'just now');
    expect(ago(now.subtract(const Duration(minutes: 1))), '1 minute ago');
    expect(ago(now.subtract(const Duration(minutes: 45))), '45 minutes ago');
    expect(ago(now.subtract(const Duration(hours: 2))), '2 hours ago');
    expect(ago(now.subtract(const Duration(days: 1))), '1 day ago');
    expect(ago(now.subtract(const Duration(days: 18))), '18 days ago');
    expect(ago(now.subtract(const Duration(days: 90))), '3 months ago');
    expect(ago(now.subtract(const Duration(days: 800))), '2 years ago');
  });

  test('relativeText handles a clock that is behind the server', () {
    final DateTime now = DateTime(2026, 8, 17, 12);

    expect(
      relativeText(
        now.add(const Duration(minutes: 5)).toUtc().toIso8601String(),
        now: now,
      ),
      'in the future',
    );
  });

  test('relativeText gives nothing for an unparseable value', () {
    expect(relativeText('not a date'), isNull);
    expect(relativeText(null), isNull);
  });
}
