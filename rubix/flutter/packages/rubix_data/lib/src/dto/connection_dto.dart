/// Wire-format representation of a saved rubix-agent connection.
///
/// Lives in `rubix_data` (pure Dart) so the same shape can be produced
/// by the REST server and consumed by the Flutter app without dragging
/// Drift/Freezed in.
class ConnectionDto {
  const ConnectionDto({
    required this.id,
    required this.label,
    required this.baseUrl,
    required this.createdAt,
    this.lastUsedAt,
  });

  factory ConnectionDto.fromJson(Map<String, dynamic> json) => ConnectionDto(
        id: json['id'] as int,
        label: json['label'] as String,
        baseUrl: json['baseUrl'] as String,
        createdAt: DateTime.parse(json['createdAt'] as String),
        lastUsedAt: json['lastUsedAt'] == null
            ? null
            : DateTime.parse(json['lastUsedAt'] as String),
      );

  final int id;
  final String label;
  final String baseUrl;
  final DateTime createdAt;
  final DateTime? lastUsedAt;

  Map<String, dynamic> toJson() => {
        'id': id,
        'label': label,
        'baseUrl': baseUrl,
        'createdAt': createdAt.toIso8601String(),
        'lastUsedAt': lastUsedAt?.toIso8601String(),
      };
}
