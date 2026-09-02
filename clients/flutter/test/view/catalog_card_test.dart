import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/catalog/episode.dart';
import 'package:shadowmask/model/common/resume_card.dart';
import 'package:shadowmask/model/common/title_ref.dart';
import 'package:shadowmask/view/card_aspect.dart';
import 'package:shadowmask/view/catalog_card.dart';

void main() {
  test('episode card is landscape and carries an episode code subtitle', () {
    final Episode e = Episode.fromJson(<String, dynamic>{
      'id': 'e1',
      'season_id': 'se1',
      'number': 2,
      'title': 'The One',
      'series_id': 'sr1',
      'series_title': 'Show',
      'season_number': 1,
    });
    final CatalogCard c = CatalogCard.fromEpisode(e);
    expect(c.aspect, CardAspect.landscape);
    expect(c.subtitle, contains('S01E02'));
    expect(c.route, startsWith('/episode?'));
  });

  test('a show card takes the series poster and names the episode', () {
    final Episode e = Episode.fromJson(<String, dynamic>{
      'id': 'e1',
      'season_id': 'se1',
      'number': 2,
      'title': 'The One',
      'series_id': 'sr1',
      'series_title': 'Show',
      'season_number': 1,
      'artwork': <String, dynamic>{
        'backdrops': <dynamic>[
          <String, dynamic>{
            'base': '/images/e1',
            'widths': <int>[480],
          },
        ],
      },
      'series_artwork': <String, dynamic>{
        'posters': <dynamic>[
          <String, dynamic>{
            'base': '/images/sr1',
            'widths': <int>[180],
          },
        ],
      },
    });
    final CatalogCard c = CatalogCard.fromEpisode(e, asSeriesPoster: true);
    expect(c.aspect, CardAspect.poster);
    expect(c.title, 'Show');
    expect(c.subtitle, 'S01E02');
    expect(c.caption, 'The One');
    expect(c.artwork?.posters.first.base, '/images/sr1');
    expect(c.route, startsWith('/episode?'));
  });

  test('a resume card for a movie puts the year under the title', () {
    final CatalogCard c = CatalogCard.fromResume(
      ResumeCard.fromJson(<String, dynamic>{
        'title': <String, dynamic>{'type': 'movie', 'id': 'm1'},
        'display_title': 'Alpha',
        'year': 2020,
        'progress_percent': 30,
      }),
    );
    expect(c.aspect, CardAspect.poster);
    expect(c.title, 'Alpha');
    expect(c.subtitle, '2020');
    expect(c.caption, isNull);
  });

  test('a resume card for an episode reads series, code, episode', () {
    final CatalogCard c = CatalogCard.fromResume(
      ResumeCard.fromJson(<String, dynamic>{
        'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
        'display_title': 'The One',
        'series_title': 'Show',
        'season_number': 1,
        'episode_number': 2,
        'series_artwork': <String, dynamic>{
          'posters': <dynamic>[
            <String, dynamic>{
              'base': '/images/sr1',
              'widths': <int>[180],
            },
          ],
        },
      }),
    );
    expect(c.aspect, CardAspect.poster);
    expect(c.title, 'Show');
    expect(c.subtitle, 'S01E02');
    expect(c.caption, 'The One');
    expect(c.artwork?.posters.first.base, '/images/sr1');
  });

  test('a resume card falls back to the episode title with no series', () {
    final CatalogCard c = CatalogCard.fromResume(
      ResumeCard.fromJson(<String, dynamic>{
        'title': <String, dynamic>{'type': 'episode', 'id': 'e1'},
        'display_title': 'The One',
        'episode_number': 2,
      }),
    );
    expect(c.title, 'The One');
    expect(c.subtitle, 'E02');
    expect(c.caption, isNull);
  });

  test('search json dispatches by type into cards', () {
    final CatalogCard person = CatalogCard.fromJson(<String, dynamic>{
      'type': 'person',
      'id': 'p1',
      'name': 'Ada',
    });
    expect(person.ref.type, TitleKind.person);
    expect(person.title, 'Ada');
    final CatalogCard movie = CatalogCard.fromJson(<String, dynamic>{
      'type': 'movie',
      'id': 'm1',
      'title': 'Alpha',
    });
    expect(movie.ref.type, TitleKind.movie);
  });
}
