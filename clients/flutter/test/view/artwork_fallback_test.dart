import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/view/artwork_fallback.dart';

Artwork _art({bool poster = false, bool backdrop = false}) => Artwork(
  posters: poster
      ? const <ImageSet>[
          ImageSet(base: '/own-poster', widths: <int>[480]),
        ]
      : const <ImageSet>[],
  backdrops: backdrop
      ? const <ImageSet>[
          ImageSet(base: '/own-backdrop', widths: <int>[960]),
        ]
      : const <ImageSet>[],
);

void main() {
  final Artwork parent = _art(poster: true, backdrop: true);

  test('a title with its own backdrop keeps it', () {
    final Artwork own = _art(backdrop: true);
    expect(backdropOrParent(own, parent), same(own));
  });

  test('a title with no backdrop of its own borrows the parent', () {
    expect(backdropOrParent(_art(), parent), same(parent));
  });

  test('having only a poster does not count as having a backdrop', () {
    expect(
      backdropOrParent(_art(poster: true), parent),
      same(parent),
      reason: 'the two kinds are separate lists, so a poster is no substitute',
    );
  });

  test('no artwork at all still falls through to the parent', () {
    expect(backdropOrParent(null, parent), same(parent));
    expect(posterOrParent(null, parent), same(parent));
  });

  test('a missing parent leaves nothing rather than throwing', () {
    expect(backdropOrParent(_art(), null), isNull);
    expect(posterOrParent(null, null), isNull);
  });

  test('posters follow the same rule as backdrops', () {
    final Artwork own = _art(poster: true);
    expect(posterOrParent(own, parent), same(own));
    expect(posterOrParent(_art(backdrop: true), parent), same(parent));
  });
}
