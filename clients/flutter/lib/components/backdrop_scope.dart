import 'package:flutter/material.dart';

import 'package:shadowmask/components/remote_image.dart';
import 'package:shadowmask/model/common/artwork.dart';
import 'package:shadowmask/util/scoped_value.dart';

const double kBackdropOpacity = 0.12;
const int kBackdropWidth = 1920;
const Duration kBackdropFade = Duration(milliseconds: 320);

class BackdropScope extends InheritedWidget {
  const BackdropScope({super.key, required this.url, required super.child});

  final ScopedValue<String?> url;

  static ScopedValue<String?>? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<BackdropScope>()?.url;

  @override
  bool updateShouldNotify(BackdropScope oldWidget) => url != oldWidget.url;
}

class PageBackdrop extends StatefulWidget {
  const PageBackdrop({
    super.key,
    required this.artwork,
    required this.imageBase,
  });

  final Artwork? artwork;
  final String imageBase;

  @override
  State<PageBackdrop> createState() => _PageBackdropState();
}

class _PageBackdropState extends State<PageBackdrop> {
  ScopedValue<String?>? _sink;
  String? _published;

  @override
  void dispose() {
    final String? published = _published;
    if (published != null) {
      _sink?.releaseAfterFrame(published, null);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final String? url = widget.artwork?.backdropUrl(
      widget.imageBase,
      kBackdropWidth,
    );
    _sink = BackdropScope.maybeOf(context);
    final ScopedValue<String?>? sink = _sink;
    if (sink != null && sink.value != url) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          sink.publish(url);
          _published = url;
        }
      });
    }
    return const SizedBox.shrink();
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
