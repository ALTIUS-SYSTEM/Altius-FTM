class RouteDemo {
  static const graph = <String, Map<String, int>>{
    'hub': {'JKT-001': 18, 'JKT-002': 40},
    'JKT-001': {'JKT-002': 14, 'JKT-003': 35},
    'JKT-002': {'JKT-003': 15, 'JKT-004': 36},
    'JKT-003': {'JKT-004': 14},
    'JKT-004': {},
  };
  static Map<String, int> shortestMinutes(Map<String, Map<String, int>> network, String source) {
    if (!network.containsKey(source) || network.values.any((edges) => edges.entries.any((edge) => edge.value < 0 || !network.containsKey(edge.key)))) {
      throw ArgumentError('Invalid graph');
    }
    final distance = <String, int>{source: 0};
    final remaining = network.keys.toSet();
    while (remaining.isNotEmpty) {
      final reachable = remaining.where(distance.containsKey).toList()..sort((a, b) => distance[a]!.compareTo(distance[b]!));
      if (reachable.isEmpty) { break; }
      final current = reachable.first;
      remaining.remove(current);
      for (final edge in network[current]!.entries) {
        final candidate = distance[current]! + edge.value;
        if (remaining.contains(edge.key) && (!distance.containsKey(edge.key) || candidate < distance[edge.key]!)) {
          distance[edge.key] = candidate;
        }
      }
    }
    return distance;
  }
}
