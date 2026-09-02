import 'package:flutter/widgets.dart';

const double kLoadMoreThreshold = 400;

class LoadMore extends StatefulWidget {
  const LoadMore({
    super.key,
    required this.onLoad,
    required this.child,
    this.enabled = true,
    this.threshold = kLoadMoreThreshold,
  });

  final VoidCallback onLoad;
  final Widget child;
  final bool enabled;
  final double threshold;

  @override
  State<LoadMore> createState() => _LoadMoreState();
}

class _LoadMoreState extends State<LoadMore> with WidgetsBindingObserver {
  ScrollPosition? _position;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeMetrics() => _scheduleCheck();

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final ScrollPosition? position = Scrollable.maybeOf(context)?.position;
    if (position == _position) {
      return;
    }
    _position?.removeListener(_check);
    _position = position;
    _position?.addListener(_check);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _position?.removeListener(_check);
    super.dispose();
  }

  void _scheduleCheck() {
    if (!widget.enabled) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((Duration _) => _check());
  }

  void _check() {
    if (!mounted || !widget.enabled) {
      return;
    }
    final ScrollPosition? position = _position;
    if (position == null || !position.hasContentDimensions) {
      return;
    }
    if (position.pixels >= position.maxScrollExtent - widget.threshold) {
      widget.onLoad();
    }
  }

  @override
  Widget build(BuildContext context) {
    _scheduleCheck();
    return widget.child;
  }
}
