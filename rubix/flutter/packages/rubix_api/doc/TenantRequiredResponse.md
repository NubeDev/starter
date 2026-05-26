# rubix_api.model.TenantRequiredResponse

## Load the model package
```dart
import 'package:rubix_api/api.dart';
```

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**error** | **String** | Always `\"tenant_required\"`. Discriminator string. | 
**memberships** | [**BuiltList&lt;TenantMembershipEntry&gt;**](TenantMembershipEntry.md) | One entry per membership row for the authenticated user. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


