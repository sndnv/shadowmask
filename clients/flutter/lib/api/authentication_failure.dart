import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/l10n/strings.dart';

class AuthenticationFailure extends ApiException {
  AuthenticationFailure() : super(Strings.signInRequired, status: 401);
}
