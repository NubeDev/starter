import 'package:freezed_annotation/freezed_annotation.dart';

part 'app_config.freezed.dart';
part 'app_config.g.dart';

/// Trivial freezed model proving codegen runs end-to-end.
@freezed
abstract class AppConfig with _$AppConfig {
  const factory AppConfig({
    required String appName,
    @Default('0.1.0') String version,
  }) = _AppConfig;

  factory AppConfig.fromJson(Map<String, dynamic> json) =>
      _$AppConfigFromJson(json);
}
