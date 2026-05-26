// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class AppLocalizationsEs extends AppLocalizations {
  AppLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get appTitle => 'Rubix';

  @override
  String get connections => 'Conexiones';

  @override
  String get addConnection => 'Agregar conexión';

  @override
  String get editConnection => 'Editar conexión';

  @override
  String get deleteConnection => 'Eliminar conexión';

  @override
  String get deleteConnectionConfirm =>
      '¿Estás seguro de que deseas eliminar esta conexión?';

  @override
  String get connectionLabel => 'Etiqueta';

  @override
  String get connectionUrl => 'URL del servidor';

  @override
  String get connectionProbing => 'Probando servidor…';

  @override
  String get connectionProbeSuccess => 'Servidor accesible';

  @override
  String get connectionProbeFailed => 'Servidor inaccesible';

  @override
  String get connectionProbeTimeout => 'Conexión agotada';

  @override
  String get connectionProbeError => 'Error de red';

  @override
  String get save => 'Guardar';

  @override
  String get probeAndSave => 'Probar y guardar';

  @override
  String get cancel => 'Cancelar';

  @override
  String get delete => 'Eliminar';

  @override
  String get login => 'Iniciar sesión';

  @override
  String get email => 'Correo electrónico';

  @override
  String get password => 'Contraseña';

  @override
  String get signIn => 'Iniciar sesión';

  @override
  String get signOut => 'Cerrar sesión';

  @override
  String get loginFailed => 'Correo o contraseña inválidos';

  @override
  String get loginError => 'Error al iniciar sesión. Intente de nuevo.';

  @override
  String get settings => 'Configuración';

  @override
  String get theme => 'Tema';

  @override
  String get themeSystem => 'Sistema';

  @override
  String get themeLight => 'Claro';

  @override
  String get themeDark => 'Oscuro';

  @override
  String get language => 'Idioma';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageSpanish => 'Español';

  @override
  String get home => 'Inicio';

  @override
  String get welcome => 'Bienvenido a Rubix';

  @override
  String connectedTo(String url) {
    return 'Conectado a $url';
  }

  @override
  String get manageConnections => 'Administrar conexiones';

  @override
  String get noConnections => 'Aún no hay conexiones';

  @override
  String get loading => 'Cargando…';

  @override
  String get error => 'Algo salió mal';

  @override
  String get retry => 'Reintentar';

  @override
  String get unreachable => 'Servidor inaccesible';

  @override
  String get unreachableDescription =>
      'No se pudo conectar al servidor. Verifica tu conexión e intenta de nuevo.';

  @override
  String get agentHealthy => 'Agente en línea';

  @override
  String get agentUnreachable => 'Agente fuera de línea';

  @override
  String get agentHealthSection => 'Estado del agente';

  @override
  String get currentUserSection => 'Sesión iniciada como';

  @override
  String get currentUserError => 'No se pudo cargar el usuario';

  @override
  String get activeConnectionSection => 'Conexión activa';
}
