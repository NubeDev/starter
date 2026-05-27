/// Legacy path — auth state machine now lives in
/// `features/auth/data/auth_controller.dart`. This file re-exports the
/// new symbols so older imports keep resolving while consumers migrate.
library;

export 'package:rubix_flutter/features/auth/data/auth_controller.dart';
export 'package:rubix_flutter/features/auth/data/auth_state.dart';
