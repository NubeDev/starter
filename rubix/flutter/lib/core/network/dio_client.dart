import 'package:dio/dio.dart';

/// Returns a bare Dio instance with no interceptors.
/// Used only for connection probing (Block 2 auth posture: no auth needed).
Dio probeDio() => Dio(
      BaseOptions(
        connectTimeout: const Duration(seconds: 5),
        receiveTimeout: const Duration(seconds: 5),
      ),
    );
