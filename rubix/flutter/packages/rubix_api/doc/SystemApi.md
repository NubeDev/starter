# rubix_api.api.SystemApi

## Load the API package
```dart
import 'package:rubix_api/api.dart';
```

All URIs are relative to *http://127.0.0.1:8088*

Method | HTTP request | Description
------------- | ------------- | -------------
[**dispatch**](SystemApi.md#dispatch) | **POST** /api/v1/tools/{tool_id} | Handler — kept at ≤20 lines. Any growth here is a smell: domain logic belongs in &#x60;rubix-tools&#x60; (push into &#x60;probe()&#x60;), shaping logic belongs in [&#x60;shape_response&#x60;].
[**healthz**](SystemApi.md#healthz) | **GET** /healthz | Liveness probe. Returns 200 with a tiny JSON body — no DB, no downstream calls. A reachable port is the entire signal.


# **dispatch**
> dispatch(toolId, body, render)

Handler — kept at ≤20 lines. Any growth here is a smell: domain logic belongs in `rubix-tools` (push into `probe()`), shaping logic belongs in [`shape_response`].

### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getSystemApi();
final String toolId = toolId_example; // String | Registered tool id (e.g. `rubix.system.disk`, `rubix.user.create`, `rubix.flow.deploy`, `rubix.undo.last`).
final JsonObject body = ; // JsonObject | 
final String render = render_example; // String | Pass `server` to ask the agent to render the `summary` Diagnostic against the negotiated locale and return it as `rendered_summary` alongside the raw structured form.

try {
    api.dispatch(toolId, body, render);
} catch on DioException (e) {
    print('Exception when calling SystemApi->dispatch: $e\n');
}
```

### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **toolId** | **String**| Registered tool id (e.g. `rubix.system.disk`, `rubix.user.create`, `rubix.flow.deploy`, `rubix.undo.last`). | 
 **body** | **JsonObject**|  | 
 **render** | **String**| Pass `server` to ask the agent to render the `summary` Diagnostic against the negotiated locale and return it as `rendered_summary` alongside the raw structured form. | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **healthz**
> healthz()

Liveness probe. Returns 200 with a tiny JSON body — no DB, no downstream calls. A reachable port is the entire signal.

### Example
```dart
import 'package:rubix_api/api.dart';

final api = RubixApi().getSystemApi();

try {
    api.healthz();
} catch on DioException (e) {
    print('Exception when calling SystemApi->healthz: $e\n');
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

