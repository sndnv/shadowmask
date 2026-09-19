String? apiBaseFrom({required String define, required String? env}) {
  if (define.isNotEmpty) {
    return define;
  }
  return env != null && env.isNotEmpty ? env : null;
}
