import 'package:shadowmask/model/common/artwork.dart';

Artwork? backdropOrParent(Artwork? own, Artwork? parent) =>
    (own?.backdrops.isNotEmpty ?? false) ? own : parent;

Artwork? posterOrParent(Artwork? own, Artwork? parent) =>
    (own?.posters.isNotEmpty ?? false) ? own : parent;
