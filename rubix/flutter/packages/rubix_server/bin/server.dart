import 'dart:io';

import 'package:args/args.dart';
import 'package:rubix_server/src/auth.dart';
import 'package:rubix_server/src/db.dart';
import 'package:rubix_server/src/handlers.dart';
import 'package:shelf/shelf.dart';
import 'package:shelf/shelf_io.dart' as io;

Future<void> main(List<String> argv) async {
  final home = Platform.environment['HOME'] ?? '.';
  final parser = ArgParser()
    ..addOption('port', abbr: 'p', defaultsTo: '8787')
    ..addOption('host', abbr: 'h', defaultsTo: 'localhost')
    ..addOption('db', defaultsTo: '$home/.rubix/rubix_web.sqlite')
    ..addOption('token-file', defaultsTo: '$home/.rubix/server.token')
    ..addMultiOption(
      'cors-origin',
      defaultsTo: const [
        'http://localhost:3031',
        'http://127.0.0.1:3031',
      ],
      help: 'Origin allowed by CORS. Repeatable. Defaults match make start-web.',
    );
  final args = parser.parse(argv);

  final dbPath = args['db'] as String;
  final dbFile = File(dbPath);
  await dbFile.parent.create(recursive: true);
  final db = openDatabase(dbPath);

  final tokenPath = args['token-file'] as String;
  final token = loadOrCreateToken(tokenPath);

  final origins = (args['cors-origin'] as List<String>).toSet();

  final handler = Pipeline()
      .addMiddleware(logRequests())
      .addMiddleware(corsMiddleware(origins))
      .addMiddleware(requireBearer(token, openPaths: const {'/healthz'}))
      .addHandler(buildRouter(db).call);

  final server = await io.serve(
    handler,
    args['host'] as String,
    int.parse(args['port'] as String),
  );

  stdout
    ..writeln(
      '[rubix_server] listening on http://${server.address.host}:${server.port}',
    )
    ..writeln('[rubix_server] db:     $dbPath')
    ..writeln('[rubix_server] token:  $tokenPath')
    ..writeln('[rubix_server] cors:   ${origins.join(', ')}');
}
