import 'package:shadowmask/model/session/capability_overrides.dart';
import 'package:shadowmask/model/session/client_decoding.dart';

const int kReportSchema = 1;
const String kReportApp = 'flutter';

Map<String, Object?> diagnosticsPayload({
  required String platform,
  required String appVersion,
  required ClientDecoding? measured,
  required CapabilityOverrides choices,
  required ClientDecoding? sent,
  required List<Map<String, Object?>> history,
  required List<String> panel,
  required List<String> log,
}) => <String, Object?>{
  'schema': kReportSchema,
  'client': <String, Object?>{
    'app': kReportApp,
    'platform': platform,
    'version': appVersion,
  },
  'capabilities': <String, Object?>{
    'measured': measured?.toJson(),
    'choices': choices.toJson(),
    'sent': sent?.toJson(),
  },
  'history': history,
  'panel': panel,
  'log': log,
};
