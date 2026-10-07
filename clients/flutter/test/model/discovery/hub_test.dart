import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/model/discovery/hub.dart';
import 'package:shadowmask/view/catalog_card.dart';

Map<String, dynamic> _hub(Map<String, dynamic> item) => <String, dynamic>{
  'id': 'recently_added_shows',
  'title': 'Recently Added Shows',
  'items': <dynamic>[item],
};

Map<String, dynamic> _series({int? episodeCount}) => <String, dynamic>{
  'type': 'series',
  'id': 's1',
  'title': 'The Expanse',
  'year': 2015,
  'episode_count': ?episodeCount,
};

void main() {
  test('a show card counts its new episodes instead of showing a year', () {
    final Hub hub = Hub.fromJson(_hub(_series(episodeCount: 3)));
    final CatalogCard card = hub.items.single;

    expect(card.title, 'The Expanse');
    expect(card.subtitle, '3 new episodes');
    expect(card.ref.type, TitleKind.series);
    expect(card.route, '/title?type=series&id=s1');
  });

  test('one new episode is not pluralised', () {
    final Hub hub = Hub.fromJson(_hub(_series(episodeCount: 1)));
    expect(hub.items.single.subtitle, '1 new episode');
  });

  test('a series without a count, or a count of 0, keeps the year', () {
    expect(Hub.fromJson(_hub(_series())).items.single.subtitle, '2015');
    expect(
      Hub.fromJson(_hub(_series(episodeCount: 0))).items.single.subtitle,
      '2015',
    );
  });
}
