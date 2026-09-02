import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/api/api_exception.dart';
import 'package:shadowmask/l10n/strings.dart';
import 'package:shadowmask/view/failure_reason.dart';

ApiException _api(String code, String message) =>
    ApiException(message, status: 409, code: code);

void main() {
  test('a known code renders the client copy, never the server text', () {
    expect(
      failureText(
        Strings.errorDelete,
        _api('not_empty', 'the movie still has versions'),
      ),
      '${Strings.errorDelete} ${Strings.reasonNotEmpty}',
    );
    expect(
      failureText(
        Strings.errorRelink,
        _api('unavailable', '3 episodes have no available file'),
      ),
      '${Strings.errorRelink} ${Strings.reasonUnavailable}',
    );
  });

  test('every code the server can return for a title action is mapped', () {
    for (final String code in <String>[
      'not_empty',
      'unavailable',
      'not_found',
      'access_denied',
    ]) {
      expect(
        failureReason(_api(code, 'raw server english')),
        isNotNull,
        reason: '$code falls through to the untranslatable server message',
      );
    }
  });

  test('an unmapped code still says something rather than nothing', () {
    final String text = failureText(
      Strings.errorDelete,
      _api('something_new', 'a reason we have no copy for'),
    );

    expect(text, startsWith(Strings.errorDelete));
    expect(
      text,
      contains('a reason we have no copy for'),
      reason:
          'the server fallback is the deliberate escape hatch for codes '
          'the client has not caught up with yet',
    );
  });

  test('a bare failure with no code is just the action', () {
    expect(
      failureText(Strings.errorDelete, Exception('boom')),
      Strings.errorDelete,
    );
    expect(failureText(Strings.errorDelete, null), Strings.errorDelete);
  });
}
