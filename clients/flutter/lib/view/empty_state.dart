class EmptyState {
  const EmptyState(this.text, {this.actionLabel, this.actionRoute});

  final String text;
  final String? actionLabel;
  final String? actionRoute;

  bool get hasAction => actionLabel != null && actionRoute != null;
}
