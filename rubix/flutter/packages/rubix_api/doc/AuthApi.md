# rubix_api.api.AuthApi

## Load the API package
```dart
import 'package:rubix_api/api.dart';
```

All URIs are relative to *http://127.0.0.1:8088*

Method | HTTP request | Description
------------- | ------------- | -------------
[**issueToken**](AuthApi.md#issuetoken) | **POST** /api/v1/auth/token | 
[**login**](AuthApi.md#login) | **POST** /api/v1/auth/login | 
[**logout**](AuthApi.md#logout) | **POST** /api/v1/auth/logout | 
[**me**](AuthApi.md#me) | **GET** /api/v1/auth/me | 


# **issueToken**
> TokenResponse issueToken(tokenRequest)



### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getAuthApi();
final TokenRequest tokenRequest = ; // TokenRequest | 

try {
    final response = api.issueToken(tokenRequest);
    print(response);
} catch on DioException (e) {
    print('Exception when calling AuthApi->issueToken: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **tokenRequest** | [**TokenRequest**](TokenRequest.md)|  | 

### Return type

[**TokenResponse**](TokenResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **login**
> LoginResponse login(loginRequest)



### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getAuthApi();
final LoginRequest loginRequest = ; // LoginRequest | 

try {
    final response = api.login(loginRequest);
    print(response);
} catch on DioException (e) {
    print('Exception when calling AuthApi->login: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **loginRequest** | [**LoginRequest**](LoginRequest.md)|  | 

### Return type

[**LoginResponse**](LoginResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **logout**
> logout()



### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getAuthApi();

try {
    api.logout();
} catch on DioException (e) {
    print('Exception when calling AuthApi->logout: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **me**
> MeResponse me()



### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getAuthApi();

try {
    final response = api.me();
    print(response);
} catch on DioException (e) {
    print('Exception when calling AuthApi->me: $e\n');
}
```

### Parameters
This endpoint does not need any parameter.

### Return type

[**MeResponse**](MeResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

