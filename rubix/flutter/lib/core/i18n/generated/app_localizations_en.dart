// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Rubix';

  @override
  String get connections => 'Connections';

  @override
  String get addConnection => 'Add Connection';

  @override
  String get editConnection => 'Edit Connection';

  @override
  String get deleteConnection => 'Delete Connection';

  @override
  String get deleteConnectionConfirm =>
      'Are you sure you want to delete this connection?';

  @override
  String get connectionLabel => 'Label';

  @override
  String get connectionUrl => 'Server URL';

  @override
  String get connectionProbing => 'Probing server…';

  @override
  String get connectionProbeSuccess => 'Server reachable';

  @override
  String get connectionProbeFailed => 'Server unreachable';

  @override
  String get connectionProbeTimeout => 'Connection timed out';

  @override
  String get connectionProbeError => 'Network error';

  @override
  String get save => 'Save';

  @override
  String get probeAndSave => 'Probe & Save';

  @override
  String get cancel => 'Cancel';

  @override
  String get delete => 'Delete';

  @override
  String get login => 'Login';

  @override
  String get email => 'Email';

  @override
  String get password => 'Password';

  @override
  String get signIn => 'Sign In';

  @override
  String get signOut => 'Sign Out';

  @override
  String get loginFailed => 'Invalid email or password';

  @override
  String get loginError => 'Login failed. Please try again.';

  @override
  String get settings => 'Settings';

  @override
  String get theme => 'Theme';

  @override
  String get themeSystem => 'System';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get language => 'Language';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageSpanish => 'Español';

  @override
  String get home => 'Home';

  @override
  String get welcome => 'Welcome to Rubix';

  @override
  String connectedTo(String url) {
    return 'Connected to $url';
  }

  @override
  String get manageConnections => 'Manage Connections';

  @override
  String get noConnections => 'No connections yet';

  @override
  String get loading => 'Loading…';

  @override
  String get error => 'Something went wrong';

  @override
  String get retry => 'Retry';

  @override
  String get unreachable => 'Server unreachable';

  @override
  String get unreachableDescription =>
      'Could not reach the server. Check your connection and try again.';

  @override
  String get agentHealthy => 'Agent online';

  @override
  String get agentUnreachable => 'Agent offline';

  @override
  String get agentHealthSection => 'Agent status';

  @override
  String get currentUserSection => 'Signed in as';

  @override
  String get currentUserError => 'Could not load user';

  @override
  String get activeConnectionSection => 'Active connection';
}
