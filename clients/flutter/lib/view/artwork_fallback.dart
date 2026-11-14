import 'package:shadowmask/model/common/artwork.dart';

Artwork? backdropOrParent(Artwork? own, Artwork? parent) =>
    (own?.backdrops.isNotEmpty ?? false) ? own : parent;

Artwork? posterOrParent(Artwork? own, Artwork? parent) =>
    (own != null && (own.posters.isNotEmpty || own.backdrops.isNotEmpty))
    ? own
    : parent;
