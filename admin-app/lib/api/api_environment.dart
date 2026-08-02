class ApiEnvironment {
  const ApiEnvironment({required this.baseUrl, required this.allowedOrigin});

  factory ApiEnvironment.fromBuildConfiguration() => const ApiEnvironment(
    baseUrl: String.fromEnvironment(
      'DEPLOY_GO_API_BASE_URL',
      defaultValue: 'http://localhost',
    ),
    allowedOrigin: String.fromEnvironment(
      'DEPLOY_GO_ALLOWED_ORIGIN',
      defaultValue: 'http://localhost',
    ),
  ).validated();

  final String baseUrl;
  final String allowedOrigin;

  ApiEnvironment validated() {
    _validateAbsoluteHttpUrl(baseUrl, 'DEPLOY_GO_API_BASE_URL');
    final origin = _validateAbsoluteHttpUrl(
      allowedOrigin,
      'DEPLOY_GO_ALLOWED_ORIGIN',
    );
    if (origin.path.isNotEmpty && origin.path != '/' ||
        origin.hasQuery ||
        origin.hasFragment) {
      throw const FormatException(
        'DEPLOY_GO_ALLOWED_ORIGIN 必须是不带路径、查询和片段的 origin',
      );
    }
    return this;
  }

  static Uri _validateAbsoluteHttpUrl(String value, String name) {
    final uri = Uri.tryParse(value);
    if (uri == null ||
        !uri.hasAuthority ||
        (uri.scheme != 'http' && uri.scheme != 'https')) {
      throw FormatException('$name 必须是绝对 http(s) URL');
    }
    return uri;
  }
}
