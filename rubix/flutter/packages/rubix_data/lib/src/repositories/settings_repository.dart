/// Storage-agnostic interface for the `app_settings` table.
abstract class SettingsRepository {
  Future<String?> getConnectionsPin();
  Future<void> setConnectionsPin(String? pin);
}
