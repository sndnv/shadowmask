import 'package:flutter/material.dart';

import 'package:shadowmask/components/skeleton.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/theme/radii.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';
import 'package:shadowmask/view/card_aspect.dart';

const int kFallbackArtWidth = 480;

class CardArt extends StatelessWidget {
  const CardArt({
    super.key,
    required this.artwork,
    required this.aspect,
    required this.imageBase,
    this.mosaic = false,
    this.borderRadius = const BorderRadius.all(Radii.md),
    this.width,
  });

  final Artwork? artwork;
  final CardAspect aspect;
  final String imageBase;
  final bool mosaic;
  final BorderRadius borderRadius;
  final double? width;

  double get _ratio => aspect == CardAspect.landscape ? 16 / 9 : 2 / 3;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final double? logical = width;
    final int pixels = logical == null
        ? kFallbackArtWidth
        : (logical * MediaQuery.devicePixelRatioOf(context)).round();
    return AspectRatio(
      aspectRatio: _ratio,
      child: ClipRRect(
        borderRadius: borderRadius,
        child: ColoredBox(color: t.artBg, child: _content(t, pixels)),
      ),
    );
  }

  Widget _content(Tokens t, int pixels) {
    final Artwork? art = artwork;
    if (mosaic && art != null) {
      final List<String> urls = art.posterMosaic(
        imageBase,
        (pixels / 2).round(),
      );
      if (urls.length > 1) {
        return _mosaic(urls, t, (pixels / 2).round());
      }
    }
    final String? url = aspect == CardAspect.landscape
        ? art?.backdropUrl(imageBase, pixels) ??
              art?.posterUrl(imageBase, pixels)
        : art?.posterUrl(imageBase, pixels) ??
              art?.backdropUrl(imageBase, pixels);
    if (url == null) {
      return _placeholder(t);
    }
    return _networkImage(url, t, pixels);
  }

  Widget _networkImage(String url, Tokens t, int pixels) => Image.network(
    url,
    fit: BoxFit.cover,
    width: double.infinity,
    height: double.infinity,
    cacheWidth: width == null ? null : pixels,
    errorBuilder: (BuildContext _, Object _, StackTrace? _) => _failed(t),
    loadingBuilder: (BuildContext _, Widget child, ImageChunkEvent? progress) =>
        progress == null ? child : _loading(),
  );

  Widget _mosaic(List<String> urls, Tokens t, int pixels) {
    Widget cell(int i) =>
        i < urls.length ? _networkImage(urls[i], t, pixels) : _placeholder(t);
    Widget row(int a, int b) => Expanded(
      child: Row(
        children: <Widget>[
          Expanded(child: cell(a)),
          Expanded(child: cell(b)),
        ],
      ),
    );
    return Column(children: <Widget>[row(0, 1), row(2, 3)]);
  }

  Widget _placeholder(Tokens t) =>
      Center(child: Icon(_placeholderIcon, color: t.border, size: 40));

  IconData get _placeholderIcon => switch (aspect) {
    CardAspect.person => Icons.person_outline,
    CardAspect.landscape => Icons.tv_outlined,
    CardAspect.poster => Icons.movie_outlined,
  };

  Widget _loading() => const Skeleton(
    width: double.infinity,
    height: double.infinity,
    radius: Radius.zero,
  );

  Widget _failed(Tokens t) => Center(
    child: Icon(
      Icons.broken_image_outlined,
      color: t.danger.withValues(alpha: 0.55),
      size: 32,
    ),
  );
}
