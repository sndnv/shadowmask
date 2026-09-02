import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/l10n/strings.dart';

const String kUsernameTaken = 'username_taken';

const Map<String, String> _reasons = <String, String>{
  'access_denied': Strings.reasonAccessDenied,
  'forbidden': Strings.reasonAccessDenied,
  'not_found': Strings.reasonNotFound,
  'version_not_found': Strings.reasonNotFound,
  'session_not_found': Strings.reasonNotFound,
  'scan_in_progress': Strings.reasonScanInProgress,
  'not_cancellable': Strings.reasonNotCancellable,
  'concurrent_limit': Strings.reasonConcurrentLimit,
  'feature_disabled': Strings.reasonFeatureDisabled,
  'not_empty': Strings.reasonNotEmpty,
  'unavailable': Strings.reasonUnavailable,
  'upstream_error': Strings.reasonUpstream,
  'negotiation_failed': Strings.reasonNegotiationFailed,
  kUsernameTaken: Strings.reasonUsernameTaken,
  'unknown_link_code': Strings.reasonUnknownLinkCode,
};

String? failureCode(Object? error) => error is ApiException ? error.code : null;

String? failureReason(Object? error) => _reasons[failureCode(error)];

String? _serverReason(Object? error) {
  if (error is! ApiException) {
    return null;
  }
  final String message = error.message.trim();
  if (message.isEmpty) {
    return null;
  }
  return message.endsWith('.') ? message : '$message.';
}

String failureText(String action, Object? error) {
  final String? reason = failureReason(error) ?? _serverReason(error);
  return reason == null ? action : '$action $reason';
}
