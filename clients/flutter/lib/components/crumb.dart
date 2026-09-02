class Crumb {
  const Crumb(this.label, {this.route});

  final String label;
  final String? route;

  @override
  bool operator ==(Object other) =>
      other is Crumb && other.label == label && other.route == route;

  @override
  int get hashCode => Object.hash(label, route);
}
