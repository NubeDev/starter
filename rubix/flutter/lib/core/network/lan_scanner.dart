/// Sweeps a /24 derived from a given IPv4 address, probing each host's
/// `/healthz` endpoint on a chosen port to discover rubix-agent
/// instances on the local network.
library;

import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';

import 'package:rubix_flutter/core/network/dio_client.dart';

/// A successful `/healthz` probe.
@immutable
class LanHit {
  const LanHit({required this.ip, required this.port, this.version});

  final String ip;
  final int port;
  final String? version;

  String get baseUrl => 'http://$ip:$port';
}

/// Progress tick for a running scan.
@immutable
class LanScanProgress {
  const LanScanProgress({
    required this.scanned,
    required this.total,
    required this.hits,
    this.lastIp,
  });

  final int scanned;
  final int total;
  final List<LanHit> hits;
  final String? lastIp;

  double get percent => total == 0 ? 0 : scanned / total;
}

/// Expand an IPv4 address into the 254 host addresses of its /24
/// (skipping `.0` and `.255`). Returns `null` if [localIp] is not a
/// usable IPv4 address (loopback, sentinel, IPv6, …).
List<String>? deriveSubnet24(String localIp) {
  final m = RegExp(r'^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.\d{1,3}$')
      .firstMatch(localIp);
  if (m == null) return null;
  final a = m.group(1)!;
  if (a == '0' || a == '127') return null;
  final b = m.group(2)!;
  final c = m.group(3)!;
  return [for (var i = 1; i <= 254; i++) '$a.$b.$c.$i'];
}

/// Scans the /24 around [localIp], emitting [LanScanProgress] updates
/// as each host completes.
Stream<LanScanProgress> scanLan(
  String localIp, {
  int port = 8088,
  Duration timeout = const Duration(milliseconds: 800),
  int concurrency = 32,
}) async* {
  final hosts = deriveSubnet24(localIp);
  if (hosts == null) {
    throw ArgumentError('Not a scannable IPv4 address: "$localIp"');
  }

  final controller = StreamController<LanScanProgress>();
  final hits = <LanHit>[];
  var done = 0;
  var cursor = 0;

  final dio = probeDio()
    ..options.connectTimeout = timeout
    ..options.receiveTimeout = timeout
    ..options.sendTimeout = timeout;

  Future<void> worker() async {
    while (true) {
      final idx = cursor++;
      if (idx >= hosts.length) return;
      final ip = hosts[idx];
      final hit = await _probe(dio, ip, port);
      done++;
      if (hit != null) hits.add(hit);
      if (!controller.isClosed) {
        controller.add(
          LanScanProgress(
            scanned: done,
            total: hosts.length,
            hits: List.unmodifiable(hits),
            lastIp: ip,
          ),
        );
      }
    }
  }

  unawaited(
    Future.wait(
      List.generate(concurrency, (_) => worker()),
    ).whenComplete(() async {
      if (!controller.isClosed) await controller.close();
    }),
  );

  yield* controller.stream;
}

Future<LanHit?> _probe(Dio dio, String ip, int port) async {
  try {
    final resp = await dio.getUri<dynamic>(
      Uri.parse('http://$ip:$port/healthz'),
    );
    final code = resp.statusCode ?? 0;
    if (code < 200 || code >= 300) return null;
    String? version;
    final data = resp.data;
    if (data is Map && data['version'] is String) {
      version = data['version'] as String;
    }
    return LanHit(ip: ip, port: port, version: version);
  } on DioException {
    return null;
  } catch (_) {
    return null;
  }
}
