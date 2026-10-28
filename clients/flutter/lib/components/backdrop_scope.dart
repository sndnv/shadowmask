import 'package:flutter/material.dart';

import 'package:shadowmask/components/remote_image.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/util/page_director.dart';

const double kBackdropOpacity = 0.12;
const int kBackdropWidth = 1920;
const Duration kBackdropFade = Duration(milliseconds: 320);

class BackdropScope extends InheritedWidget {
  const BackdropScope({
    super.key,
    required this.backdrops,
    required super.child,
  });

  final PageDirector<String?> backdrops;

  static PageDirector<String?>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<BackdropScope>()?.backdrops;

  @override
  bool updateShouldNotify(BackdropScope oldWidget) =>
      backdrops != oldWidget.backdrops;
}

Future<bool> precacheBackdrop(String? url, BuildContext context) async {
  bool ready = true;
  await precacheImage(
    remoteImage(url!),
    context,
    onError: (Object _, StackTrace? _) => ready = false,
  );
  return ready;
}

class PageBackdrop extends StatelessWidget {
  const PageBackdrop({
    super.key,
    required this.artwork,
    required this.imageBase,
  }) : child = null;

  const PageBackdrop.none({super.key, this.child})
    : artwork = null,
      imageBase = '';

  final Artwork? artwork;
  final String imageBase;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final Route<dynamic>? page = ModalRoute.of(context);
    if (page != null) {
      BackdropScope.maybeOf(
        context,
      )?.request(page, artwork?.backdropUrl(imageBase, kBackdropWidth));
    }
    return child ?? const SizedBox.shrink();
  }
}

class BackdropWash extends StatelessWidget {
  const BackdropWash({super.key, required this.url});

  final String? url;

  @override
  Widget build(BuildContext context) {
    final String? source = url;
    final Widget wash = source == null
        ? const SizedBox.shrink(key: ValueKey<String>(''))
        : _WashImage(key: ValueKey<String>(source), url: source);
    if (MediaQuery.disableAnimationsOf(context)) {
      return IgnorePointer(child: wash);
    }
    return IgnorePointer(
      child: AnimatedSwitcher(
        duration: kBackdropFade,
        switchInCurve: Curves.easeOut,
        switchOutCurve: Curves.easeOut,
        child: wash,
      ),
    );
  }
}

class _WashImage extends StatelessWidget {
  const _WashImage({super.key, required this.url});

  final String url;

  @override
  Widget build(BuildContext context) {
    return Image(
      image: remoteImage(url),
      fit: BoxFit.cover,
      alignment: Alignment.topCenter,
      width: double.infinity,
      height: double.infinity,
      frameBuilder:
          (
            BuildContext context,
            Widget child,
            int? frame,
            bool wasSynchronouslyLoaded,
          ) {
            if (wasSynchronouslyLoaded ||
                MediaQuery.disableAnimationsOf(context)) {
              return Opacity(
                opacity: frame == null ? 0 : kBackdropOpacity,
                child: child,
              );
            }
            return AnimatedOpacity(
              opacity: frame == null ? 0 : kBackdropOpacity,
              duration: kBackdropFade,
              curve: Curves.easeOut,
              child: child,
            );
          },
      errorBuilder: (BuildContext _, Object _, StackTrace? _) =>
          const SizedBox.shrink(),
    );
  }
}
