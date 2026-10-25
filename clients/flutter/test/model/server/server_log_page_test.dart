import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/server/server_log_page.dart';

void main() {
  test('ServerLogPage.fromJson reads the entries and both cursors', () {
    final ServerLogPage page = ServerLogPage.fromJson(<String, dynamic>{
      'entries': <String>['a', 'b'],
      'before': '2026-10-24-08:3',
      'after': '2026-10-24-09:1',
      'more': true,
    });

    expect(page.entries, <String>['a', 'b']);
    expect(page.before, '2026-10-24-08:3');
    expect(page.after, '2026-10-24-09:1');
    expect(page.more, isTrue);
  });

  test('ServerLogPage.fromJson of nothing logged has no cursors', () {
    final ServerLogPage page = ServerLogPage.fromJson(<String, dynamic>{
      'entries': <String>[],
      'before': null,
      'after': null,
      'more': false,
    });

    expect(page, const ServerLogPage());
  });
}
