import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/library/scan_state.dart';

void main() {
  test('ScanState parses status and progress', () {
    final ScanState s = ScanState.fromJson(<String, dynamic>{
      'library_id': 'l1',
      'status': 'running',
      'progress': 0.25,
      'started_at': '2026-01-01T00:00:00Z',
      'last_scanned_at': '2026-01-01T00:04:00Z',
    });

    expect(s.libraryId, 'l1');
    expect(s.status, ScanStatus.running);
    expect(s.progress, 0.25);
    expect(s.startedAt, '2026-01-01T00:00:00Z');
    expect(s.error, isNull);
  });

  test('ScanState defaults progress and carries an error', () {
    final ScanState s = ScanState.fromJson(<String, dynamic>{
      'library_id': 'l1',
      'status': 'failed',
      'error': 'permission denied',
    });

    expect(s.status, ScanStatus.failed);
    expect(s.progress, 0);
    expect(s.error, 'permission denied');
  });
}
