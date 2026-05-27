import 'package:rubix_data/rubix_data.dart';
import 'package:rubix_flutter/core/storage/daos/settings_dao.dart';

/// Native impl of [SettingsRepository] — pass-through to Drift's
/// [SettingsDao]. See [LocalConnectionsRepository] for the same shape.
class LocalSettingsRepository implements SettingsRepository {
  LocalSettingsRepository(this._dao);

  final SettingsDao _dao;

  @override
  Future<String?> getConnectionsPin() => _dao.getConnectionsPin();

  @override
  Future<void> setConnectionsPin(String? pin) =>
      _dao.setConnectionsPin(pin);
}
