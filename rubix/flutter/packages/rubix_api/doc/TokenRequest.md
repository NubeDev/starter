# rubix_api.model.TokenRequest

## Load the model package
```dart
import 'package:rubix_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **String** | User's email — same identifier as `POST /auth/login`. | 
**password** | **String** | Plaintext password. | 
**tenantId** | **String** | Optional tenant binding. When omitted, the route resolves the tenant from the user's memberships (requires [`AuthState::with_tenants`]). See design doc §payload. | [optional] 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


