import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import 'package:shadowmask/components/fact.dart';
import 'package:shadowmask/theme/shapes.dart';
import 'package:shadowmask/theme/shapes_context.dart';
import 'package:shadowmask/theme/space.dart';
import 'package:shadowmask/theme/tokens.dart';
import 'package:shadowmask/theme/tokens_context.dart';

const double _kOutline = 1.2;
const double _kValueSize = 13;

class FactReadout extends StatelessWidget {
  const FactReadout(this.fact, {super.key});

  final Fact fact;

  @override
  Widget build(BuildContext context) {
    final Tokens t = context.tokens;
    final Shapes shapes = context.shapes;
    final Size face =
        Marks.readout * (1 - 2 * _kOutline / Marks.readout.height);
    final double grow = math.max(
      1,
      MediaQuery.textScalerOf(context).scale(_kValueSize) / _kValueSize,
    );
    return MergeSemantics(
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          SizedBox.fromSize(
            size: Marks.readout * grow,
            child: FittedBox(
              child: MediaQuery.withNoTextScaling(
                child: Container(
                  width: Marks.readout.width,
                  height: Marks.readout.height,
                  alignment: Alignment.center,
                  decoration: ShapeDecoration(
                    color: shapes.line(t.border),
                    shape: Shapes.hexagon(Marks.readout),
                  ),
                  child: Container(
                    width: face.width,
                    height: face.height,
                    alignment: Alignment.center,
                    decoration: ShapeDecoration(
                      color: t.surface,
                      shape: Shapes.hexagon(face),
                    ),
                    child: Text(
                      fact.value,
                      style: TextStyle(
                        color: t.text,
                        fontSize: _kValueSize,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(width: Space.s2),
          Container(width: 12, height: 1, color: shapes.line(t.border)),
          Container(
            width: Marks.readoutDot.width,
            height: Marks.readoutDot.height,
            decoration: ShapeDecoration(
              color: shapes.highContrast ? t.text : t.accent,
              shape: Shapes.hexagon(Marks.readoutDot),
            ),
          ),
          const SizedBox(width: Space.s1),
          Text(
            shapes.caps(fact.label),
            semanticsLabel: shapes.capsLabel(fact.label),
            style: shapes.mono(
              TextStyle(color: t.accent, fontSize: 12, letterSpacing: 1.2),
            ),
          ),
        ],
      ),
    );
  }
}
