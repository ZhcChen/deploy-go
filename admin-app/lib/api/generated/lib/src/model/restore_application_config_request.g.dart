// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'restore_application_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RestoreApplicationConfigRequest
    extends RestoreApplicationConfigRequest {
  @override
  final int expectedVersion;
  @override
  final String? templateVersion;
  @override
  final int? version;

  factory _$RestoreApplicationConfigRequest([
    void Function(RestoreApplicationConfigRequestBuilder)? updates,
  ]) => (RestoreApplicationConfigRequestBuilder()..update(updates))._build();

  _$RestoreApplicationConfigRequest._({
    required this.expectedVersion,
    this.templateVersion,
    this.version,
  }) : super._();
  @override
  RestoreApplicationConfigRequest rebuild(
    void Function(RestoreApplicationConfigRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RestoreApplicationConfigRequestBuilder toBuilder() =>
      RestoreApplicationConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RestoreApplicationConfigRequest &&
        expectedVersion == other.expectedVersion &&
        templateVersion == other.templateVersion &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, expectedVersion.hashCode);
    _$hash = $jc(_$hash, templateVersion.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RestoreApplicationConfigRequest')
          ..add('expectedVersion', expectedVersion)
          ..add('templateVersion', templateVersion)
          ..add('version', version))
        .toString();
  }
}

class RestoreApplicationConfigRequestBuilder
    implements
        Builder<
          RestoreApplicationConfigRequest,
          RestoreApplicationConfigRequestBuilder
        > {
  _$RestoreApplicationConfigRequest? _$v;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  String? _templateVersion;
  String? get templateVersion => _$this._templateVersion;
  set templateVersion(String? templateVersion) =>
      _$this._templateVersion = templateVersion;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  RestoreApplicationConfigRequestBuilder() {
    RestoreApplicationConfigRequest._defaults(this);
  }

  RestoreApplicationConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _expectedVersion = $v.expectedVersion;
      _templateVersion = $v.templateVersion;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RestoreApplicationConfigRequest other) {
    _$v = other as _$RestoreApplicationConfigRequest;
  }

  @override
  void update(void Function(RestoreApplicationConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RestoreApplicationConfigRequest build() => _build();

  _$RestoreApplicationConfigRequest _build() {
    final _$result =
        _$v ??
        _$RestoreApplicationConfigRequest._(
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'RestoreApplicationConfigRequest',
            'expectedVersion',
          ),
          templateVersion: templateVersion,
          version: version,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
