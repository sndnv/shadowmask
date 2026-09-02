import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/library/library.dart';

void main() {
  test('Library parses kind, origin, watcher and lists', () {
    final Library l = Library.fromJson(<String, dynamic>{
      'id': 'l1',
      'name': 'Movies',
      'kind': 'movie',
      'origin': 'external',
      'roots': <String>['/a', '/b'],
      'watcher': 'polling',
      'scan_schedule': '0 0 * * *',
      'metadata_sources': <String>['tmdb'],
      'sort_articles': <String>['der', 'die', 'das'],
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-02T00:00:00Z',
    });

    expect(l.kind, LibraryKind.movie);
    expect(l.origin, LibraryOrigin.external);
    expect(l.watcher, WatcherStrategy.polling);
    expect(l.roots, <String>['/a', '/b']);
    expect(l.scanSchedule, '0 0 * * *');
    expect(l.metadataSources, <String>['tmdb']);
    expect(l.sortArticles, <String>['der', 'die', 'das']);
  });

  test('Library defaults origin to local and lists to empty', () {
    final Library l = Library.fromJson(<String, dynamic>{
      'id': 'l1',
      'name': 'Shows',
      'kind': 'tv',
      'watcher': 'manual',
      'created_at': '2026-01-01T00:00:00Z',
      'updated_at': '2026-01-01T00:00:00Z',
    });

    expect(l.kind, LibraryKind.tv);
    expect(l.origin, LibraryOrigin.local);
    expect(l.roots, isEmpty);
    expect(l.metadataSources, isEmpty);
    expect(l.sortArticles, isEmpty);
    expect(l.scanSchedule, isNull);
  });
}
