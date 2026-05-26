import 'package:freezed_annotation/freezed_annotation.dart';

part 'me_response.freezed.dart';
part 'me_response.g.dart';

/// Wire shape returned by `GET /api/v1/auth/me`.
@freezed
abstract class MeResponse with _$MeResponse {
  const factory MeResponse({
    required String subject,
    required String email,
    required String role,
  }) = _MeResponse;

  factory MeResponse.fromJson(Map<String, dynamic> json) =>
      _$MeResponseFromJson(json);
}
