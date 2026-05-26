//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_import

import 'package:one_of_serializer/any_of_serializer.dart';
import 'package:one_of_serializer/one_of_serializer.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:built_value/serializer.dart';
import 'package:built_value/standard_json_plugin.dart';
import 'package:built_value/iso_8601_date_time_serializer.dart';
import 'package:rubix_api/src/date_serializer.dart';
import 'package:rubix_api/src/model/date.dart';

import 'package:rubix_api/src/model/login_request.dart';
import 'package:rubix_api/src/model/login_response.dart';
import 'package:rubix_api/src/model/me_response.dart';
import 'package:rubix_api/src/model/missing_tenant_id_response.dart';
import 'package:rubix_api/src/model/password_not_set_response.dart';
import 'package:rubix_api/src/model/problem.dart';
import 'package:rubix_api/src/model/role.dart';
import 'package:rubix_api/src/model/tenant_membership_entry.dart';
import 'package:rubix_api/src/model/tenant_required_response.dart';
import 'package:rubix_api/src/model/token_request.dart';
import 'package:rubix_api/src/model/token_response.dart';

part 'serializers.g.dart';

@SerializersFor([
  LoginRequest,
  LoginResponse,
  MeResponse,
  MissingTenantIdResponse,
  PasswordNotSetResponse,
  Problem,
  Role,
  TenantMembershipEntry,
  TenantRequiredResponse,
  TokenRequest,
  TokenResponse,
])
Serializers serializers = (_$serializers.toBuilder()
      ..add(const OneOfSerializer())
      ..add(const AnyOfSerializer())
      ..add(const DateSerializer())
      ..add(Iso8601DateTimeSerializer()))
    .build();

Serializers standardSerializers =
    (serializers.toBuilder()..addPlugin(StandardJsonPlugin())).build();
