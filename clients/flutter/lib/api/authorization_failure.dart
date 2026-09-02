import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/l10n/strings.dart';

class AuthorizationFailure extends ApiException {
  AuthorizationFailure() : super(Strings.notAuthorized, status: 403);
}
