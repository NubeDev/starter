import 'package:freezed_annotation/freezed_annotation.dart';

part 'connection.freezed.dart';
part 'connection.g.dart';

@freezed
abstract class Connection with _$Connection {
  const factory Connection({
    required int id,
    required String label,
    required String baseUrl,
    required DateTime createdAt,
    DateTime? lastUsedAt,
  }) = _Connection;

  factory Connection.fromJson(Map<String, dynamic> json) =>
      _$ConnectionFromJson(json);
}
