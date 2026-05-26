# rubix_api.model.PasswordNotSetResponse

## Load the model package
```dart
import 'package:rubix_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**error** | **String** | Always `\"password_not_set\"`. Discriminator field; lets clients pattern-match without inspecting the HTTP status alone. | 
**providers** | **BuiltList&lt;String&gt;** | Provider ids the user has linked. Empty list when no third-party path is configured (the default [`crate::NoLinkedProviders`] impl). | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


