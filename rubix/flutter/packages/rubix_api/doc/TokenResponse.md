# rubix_api.model.TokenResponse

## Load the model package
```dart
import 'package:rubix_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**expiresAt** | [**DateTime**](DateTime.md) | Absolute UTC expiry (RFC3339). Advisory in v1 — clients react to 401 rather than pre-emptively refreshing. | 
**token** | **String** | The plaintext bearer (`sak_<id>.<secret>`). Shown once; the server stores only the argon2id hash of the secret. | 
**tokenType** | **String** | Always `\"Bearer\"`. Reserved for the future refresh-token flow. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


