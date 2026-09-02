import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/pages/viewer/list_prefs_store.dart';
import 'package:shadowmask/pages/viewer/list_query.dart';
import 'package:shared_preferences/shared_preferences.dart';

ListQuery _query({bool sortFromUrl = false, bool orderFromUrl = false}) =>
    ListQuery(
      offset: 0,
      sort: 'added_at',
      order: 'asc',
      genres: const <String>[],
      sortFromUrl: sortFromUrl,
      orderFromUrl: orderFromUrl,
    );

void main() {
  setUp(() => SharedPreferences.setMockInitialValues(<String, Object>{}));

  test('movies and series keep separate orderings', () async {
    await const ListPrefsStore('movies').save('title', 'asc');
    await const ListPrefsStore('series').save('year', 'desc');

    expect(await const ListPrefsStore('movies').load(), (
      sort: 'title',
      order: 'asc',
    ));
    expect(await const ListPrefsStore('series').load(), (
      sort: 'year',
      order: 'desc',
    ));
  });

  test('an unset scope loads nothing', () async {
    expect(await const ListPrefsStore('movies').load(), isNull);
  });

  test('a stored ordering fills in what the url does not carry', () {
    const ListOrdering stored = (sort: 'title', order: 'desc');

    final ListQuery applied = _query().withStored(stored);
    expect(applied.sort, 'title');
    expect(applied.order, 'desc');
  });

  test('the url wins over the stored ordering', () {
    const ListOrdering stored = (sort: 'title', order: 'desc');

    final ListQuery applied = _query(
      sortFromUrl: true,
      orderFromUrl: true,
    ).withStored(stored);
    expect(applied.sort, 'added_at');
    expect(applied.order, 'asc');
  });
}
