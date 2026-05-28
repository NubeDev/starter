import 'package:dio/dio.dart';

/// Translates technical exception strings into human-friendly copy.
///
/// Per `DESIGN.md` §9 — **no raw exception text in the UI.** The original
/// error message is returned separately as [details] so callers can put
/// it behind a "View error details" affordance.
class HumanError {
  const HumanError({required this.headline, required this.body, this.details});

  final String headline;
  final String body;
  final String? details;
}

HumanError humanizeNetworkError(Object error) {
  final raw = error.toString();

  if (error is DioException) {
    switch (error.type) {
      case DioExceptionType.connectionTimeout:
      case DioExceptionType.sendTimeout:
      case DioExceptionType.receiveTimeout:
        return HumanError(
          headline: 'Connection timed out',
          body: "The Rubix agent didn't respond in time. "
              'Check that it is running, then try again.',
          details: raw,
        );
      case DioExceptionType.connectionError:
        return HumanError(
          headline: "Can't reach the Rubix agent",
          body: 'No response from the agent. '
              'Check your connection or that the service is online.',
          details: raw,
        );
      case DioExceptionType.badCertificate:
        return HumanError(
          headline: 'Secure connection failed',
          body: "The agent's certificate could not be verified.",
          details: raw,
        );
      case DioExceptionType.badResponse:
        final code = error.response?.statusCode;
        if (code == 401 || code == 403) {
          return HumanError(
            headline: 'Not authorised',
            body: 'Your session may have expired. Sign in again.',
            details: raw,
          );
        }
        if (code != null && code >= 500) {
          return HumanError(
            headline: 'Agent error',
            body: 'The Rubix agent returned an unexpected error.',
            details: raw,
          );
        }
        return HumanError(
          headline: 'Request failed',
          body: 'The agent rejected the request.',
          details: raw,
        );
      case DioExceptionType.cancel:
        return HumanError(
          headline: 'Request cancelled',
          body: 'The request was cancelled before it finished.',
          details: raw,
        );
      case DioExceptionType.unknown:
        return HumanError(
          headline: 'Connection failed',
          body: "Couldn't talk to the Rubix agent right now.",
          details: raw,
        );
    }
  }

  return HumanError(
    headline: 'Something went wrong',
    body: "We couldn't complete that just now. Please try again.",
    details: raw,
  );
}
