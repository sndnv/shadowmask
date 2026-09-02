import 'package:freezed_annotation/freezed_annotation.dart';

enum Quality {
  @JsonValue('sd')
  sd,
  @JsonValue('hd')
  hd,
  @JsonValue('fhd')
  fhd,
  @JsonValue('uhd')
  uhd,
}

extension QualityLabel on Quality {
  String get label => name.toUpperCase();
}
