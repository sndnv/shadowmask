import 'package:material_ui/material_ui.dart';

const double kBracketReach = 7;
const double _kArm = 16;
const double _kStroke = 2;

class CornerBrackets extends StatelessWidget {
  const CornerBrackets({super.key, required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    final BorderSide side = BorderSide(color: color, width: _kStroke);
    return IgnorePointer(
      child: Stack(
        fit: StackFit.expand,
        children: <Widget>[
          for (final Alignment corner in const <Alignment>[
            Alignment.topLeft,
            Alignment.topRight,
            Alignment.bottomLeft,
            Alignment.bottomRight,
          ])
            Align(
              alignment: corner,
              child: SizedBox.square(
                dimension: _kArm,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    border: Border(
                      top: corner.y < 0 ? side : BorderSide.none,
                      bottom: corner.y > 0 ? side : BorderSide.none,
                      left: corner.x < 0 ? side : BorderSide.none,
                      right: corner.x > 0 ? side : BorderSide.none,
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
