import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

const double kVisibilityMargin = 200;

class WhenVisible extends StatefulWidget {
  const WhenVisible({
    super.key,
    required this.onVisible,
    required this.child,
    this.margin = kVisibilityMargin,
  });

  final VoidCallback onVisible;
  final Widget child;
  final double margin;

  @override
  State<WhenVisible> createState() => _WhenVisibleState();
}

class _WhenVisibleState extends State<WhenVisible> {
  ScrollPosition? _position;
  bool _fired = false;

  @override
  void initState() {
    super.initState();
    _scheduleCheck();
  }

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
    _position?.removeListener(_check);
    super.dispose();
  }

  void _scheduleCheck() {
    WidgetsBinding.instance.addPostFrameCallback((Duration _) => _check());
  }

  void _check() {
    if (_fired || !mounted || !_visible()) {
      return;
    }
    _fired = true;
    _position?.removeListener(_check);
    widget.onVisible();
  }

  bool _visible() {
    if (_position == null) {
      return true;
    }
    final RenderObject? object = context.findRenderObject();
    if (object is! RenderBox || !object.hasSize) {
      return false;
    }
    final RenderObject? viewport = RenderAbstractViewport.maybeOf(object);
    if (viewport is! RenderBox) {
      return true;
    }
    final double top = object.localToGlobal(Offset.zero, ancestor: viewport).dy;
    final double bottom = top + object.size.height;
    return bottom >= -widget.margin &&
        top <= viewport.size.height + widget.margin;
  }

  @override
  Widget build(BuildContext context) {
    if (!_fired) {
      _scheduleCheck();
    }
    return widget.child;
  }
}
